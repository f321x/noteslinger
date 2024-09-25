use anyhow::*;
use log::info;
use nostr_sdk::{bitcoin::hashes::{Hash, sha256::Hash as Sha256Hash}, prelude::*};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde_json::json;
use std::env;
use tokio::runtime::Runtime;

fn main() {
    // enable logging to stdout
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // validate cli arguments and parse pow target
    let args = env::args().collect::<Vec<String>>();
    if args.len() != 3 || args[1].is_empty() {
        println!("Give the message you want to publish as argument and pow target as second");
        return;
    }
    let pow_target = args[2].parse::<u8>().unwrap();

    // generate random keys and assemble unsigned nostr event to hash
    let my_keys = Keys::generate();
    let unsigned_event = UnsignedEvent::new(
        my_keys.public_key(),
        Timestamp::now(),
        Kind::TextNote,
        None,
        args[1].clone(),
    );

    // Hash event
    info!("Hashing: '{}' to target: {}", args[1], args[2]);
    let start_time = std::time::Instant::now();
    let pow_event = hash_event(unsigned_event, pow_target).unwrap();

    // Create and run the Tokio runtime and publish the event
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        publish_event(pow_event, my_keys).await;
    });
    info!(
        "Published event with pow {} in {:?}",
        pow_target,
        start_time.elapsed()
    );
}

fn hash_event(event: nostr_sdk::UnsignedEvent, difficulty: u8) -> anyhow::Result<UnsignedEvent> {
    let nostr_sdk::UnsignedEvent {
        kind,
        content,
        created_at,
        pubkey,
        ..
    } = event;

    let start = std::time::Instant::now();
    let result = (1u128..u128::MAX).par_bridge().find_map_any(|nonce| {
        let hash: Sha256Hash = Sha256Hash::hash(json!([0, pubkey, created_at, kind, [["nonce", nonce, difficulty]], content]).to_string().as_bytes());
        if nip13::get_leading_zero_bits(hash) >= difficulty {
            Some(nonce)
        } else {
            None
        }
    });
    let duration = start.elapsed();
    
    if let Some(nonce) = result {
        let tags = vec![Tag::pow(nonce, difficulty)];
        info!("KiloNonces per second: {}", (nonce/1000) as f64 / duration.as_secs_f64());
        Ok(UnsignedEvent {
            id: Some(EventId::new(&pubkey, &created_at, &kind, &tags, &content)),
            pubkey,
            created_at,
            kind,
            tags,
            content,
        })
    } else {
        Err(anyhow!("Failed to find valid PoW"))
    }
}

async fn publish_event(pow_event: UnsignedEvent, keys: Keys) {
    let client = Client::new(keys.clone());
    client
        .add_relay("wss://nostr.bitcoiner.social")
        .await
        .unwrap();
    client.add_relay("wss://nostr.mom").await.unwrap();
    client.add_relay("wss://nos.lol").await.unwrap();
    client.add_relay("wss://powrelay.xyz").await.unwrap();
    client.add_relay("wss://relay.damus.io").await.unwrap();
    client.add_relay("wss://labour.fiatjaf.com/").await.unwrap();
    client.add_relay("wss://140.f7z.io").await.unwrap();
    client.add_relay("wss://nostr.lu.ke").await.unwrap();
    client.add_relay("wss://relay.nostr.band/").await.unwrap();
    client.connect().await;

    let signed_event = pow_event.sign(&keys).unwrap();
    client.send_event(signed_event).await.unwrap();
}

// this is a test to compare the speed of using single threaded nostr-sdk vs multi threaded rayon hashing
// run with RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture
#[test]
fn test_performance_comparison() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    let my_keys = Keys::generate();
    let difficulty = 22;
    let iterations = 6;

    let unsigned_event = UnsignedEvent::new(
        my_keys.public_key(),
        Timestamp::now(),
        Kind::TextNote,
        None,
        "Hello, World!".to_string(),
    );
    let start = std::time::Instant::now();
    for _ in 0..6 {
        let _ = hash_event(unsigned_event.clone(), difficulty).unwrap();
    }
    let duration_rayon_avg = start.elapsed() / iterations;
    println!(
        "Average duration rayon iter pow {} (multi threaded): {:?}",
        difficulty, duration_rayon_avg
    );
    println!("Cooling down for 10 seconds");
    std::thread::sleep(std::time::Duration::from_secs(10));
    // // boilerplate to test new pow implementations // //
    // let start = std::time::Instant::now();
    // for _ in 0..6 {
    //     let _ = hash_event_new(unsigned_event.clone(), difficulty).unwrap();
    // }
    // let duration_rayon_avg = start.elapsed() / iterations;
    // println!(
    //     "Average duration rayon iter pow {} (multi threaded): {:?}",
    //     difficulty, duration_rayon_avg
    // );
    // println!("Cooling down for 10 seconds");
    // std::thread::sleep(std::time::Duration::from_secs(10));

    // get average duration for sdk pow
    let start = std::time::Instant::now();
    for _ in 0..6 {
        let _ = EventBuilder::new(Kind::TextNote, "Hello, World!", None)
            .pow(22)
            .to_unsigned_event(my_keys.public_key());
    }
    let duration_sdk_avg = start.elapsed() / iterations;
    println!(
        "Average duration nostr-sdk pow {} (single threaded): {:?}",
        difficulty, duration_sdk_avg
    );
    assert!(duration_rayon_avg < duration_sdk_avg);
}
