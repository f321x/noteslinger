use anyhow::*;
use log::info;
use nostr_sdk::prelude::*;
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde_json::json;
use std::{env, ptr};
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
    let pow_event;
    unsafe {
        pow_event = hash_event(unsigned_event, pow_target).unwrap();
    }
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

unsafe fn hash_event(event: nostr_sdk::UnsignedEvent, difficulty: u8) -> anyhow::Result<UnsignedEvent> {
    let nostr_sdk::UnsignedEvent {
        kind,
        content,
        created_at,
        pubkey,
        ..
    } = event;
    
    let mut byte_event = json!([0, pubkey, created_at, kind, [["nonce", "0", difficulty]], content]).to_string().into_bytes();
    // Find the position of the tags field
    let tags_start = byte_event.windows(9).position(|window| window == b"[\"nonce\",").unwrap();
    let nonce_start = tags_start + 9; // Start of the nonce value
    // println!("nonce_start: {}", nonce_start);
    // println!("byte_event: {:?}", byte_event.to_hex_string(Case::Lower));
    let tags_end = tags_start + byte_event[tags_start..].iter().position(|&b| b == b']').unwrap();


    
    let result = (1u128..u128::MAX).par_bridge().find_map_any(|nonce| {
        let mut local_byte_event = byte_event.clone();
        let nonce_str = nonce.to_string();
        let nonce_bytes = nonce_str.as_bytes();
        
        // Write nonce directly into the byte array
        ptr::copy_nonoverlapping(nonce_bytes.as_ptr(), local_byte_event.as_mut_ptr().add(nonce_start), nonce_bytes.len());
        
        // Fill the rest with quotation marks and closing brackets
        for i in nonce_start + nonce_bytes.len()..tags_end {
            *local_byte_event.get_unchecked_mut(i) = b'"';
            *local_byte_event.get_unchecked_mut(i + 1) = b']';
            break;
        }

        // Hash the modified byte array
        let hash = Sha256::digest(&local_byte_event);

        // Check if the hash meets the difficulty criteria
        if check_pow_difficulty(&hash, difficulty) {
            Some((nonce, local_byte_event))
        } else {
            None
        }
    });

    if let Some(tags) = result {
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

#[inline(always)]
unsafe fn check_pow_difficulty(hash: &[u8; 32], difficulty: u8) -> bool {
    let full_bytes = difficulty / 8;
    let remaining_bits = difficulty % 8;

    // Check full bytes
    if full_bytes > 0 {
        let full_bytes_ptr = hash.as_ptr() as *const u64;
        let full_u64s = full_bytes / 8;
        
        for i in 0..full_u64s {
            if *full_bytes_ptr.add(i) != 0 {
                return false;
            }
        }

        let remaining_full_bytes = full_bytes % 8;
        if remaining_full_bytes > 0 {
            let remaining_ptr = hash.as_ptr().add(full_u64s * 8);
            for i in 0..remaining_full_bytes {
                if *remaining_ptr.add(i) != 0 {
                    return false;
                }
            }
        }
    }

    // Check remaining bits
    if remaining_bits > 0 {
        let last_byte = *hash.as_ptr().add(full_bytes as usize);
        let mask = 0xFFu8 << (8 - remaining_bits);
        if last_byte & mask != 0 {
            return false;
        }
    }

    true
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
