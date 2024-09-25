use anyhow::*;
use log::info;
use nostr_sdk::prelude::*;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::env;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let args = env::args().collect::<Vec<String>>();
    if args.len() != 3 || args[1].is_empty() {
        println!("Give the message you want to publish as argument and pow target as second");
        return;
    }
    info!("Working on: '{}', to target: '{}'", args[1], args[2]);
    let pow_target = args[2].parse::<u8>().unwrap();
    let my_keys = Keys::generate();
    let client = Client::new(my_keys.clone());
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

    let unsigned_event = UnsignedEvent::new(
        my_keys.public_key(),
        Timestamp::now(),
        Kind::TextNote,
        None,
        args[1].clone(),
    );
    let pow_event =
        tokio::task::spawn_blocking(move || hash_event(unsigned_event, pow_target).unwrap())
            .await
            .unwrap();
    let signed_event = pow_event.sign(&my_keys).unwrap();
    client.send_event(signed_event).await.unwrap();
}

fn hash_event(event: nostr_sdk::UnsignedEvent, difficulty: u8) -> anyhow::Result<UnsignedEvent> {
    let tags = event.tags;
    let kind = event.kind;
    let content = event.content;
    let created_at = event.created_at;
    let pubkey = event.pubkey;

    let result = (1u128..).par_bridge().find_map_any(|nonce| {
        let mut tags = tags.clone();
        tags.push(Tag::pow(nonce, difficulty));

        let id: EventId = EventId::new(&pubkey, &created_at, &kind, &tags, &content);

        if id.check_pow(difficulty) {
            Some((id, tags))
        } else {
            None
        }
    });

    if let Some((id, tags)) = result {
        Ok(UnsignedEvent {
            id: Some(id),
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
