use serde_json::{json, Value};
use std::error::Error;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures::StreamExt;
use chrono::Local;

const SYNC_TOPIC: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";

/// Subscribes to sync events from the network
///
/// Listens for Sync events from Uniswap V2 pairs and processes reserve updates
///
/// # Returns
/// * `Result<(), Box<dyn Error>>` - Ok(()) on successful subscription
///
/// # Errors
/// * If WebSocket connection cannot be established
/// * If subscription request fails
/// * If message parsing fails
/// * If network connection is lost
/// * If received message format is invalid
/// * If WebSocket stream terminates unexpectedly
/// * If message sending fails
pub async fn subscribe_to_sync() -> Result<(), Box<dyn Error>> {
    let subscribe_request = json!({
        "jsonrpc": "2.0",
        "method": "eth_subscribe",
        "params": ["logs"],
        "id": 1
    });

    let mut ws_stream = crate::utils::providers::send_ws_request(subscribe_request.to_string()).await?;

    while let Some(msg) = ws_stream.next().await {
        let text = match msg {
            Ok(Message::Text(text)) => text,
            Err(e) => {
                eprintln!("Error receiving message: {e:?}");
                break;
            }
            _ => continue,
        };

        let json: Value = match serde_json::from_str(&text) {
            Ok(json) => json,
            Err(_) => continue,
        };

        // Get params or continue
        let params = match json.get("params") {
            Some(params) => params,
            None => continue,
        };

        // Get result or continue
        let result = match params.get("result") {
            Some(result) => result,
            None => continue,
        };

        // Get topics or continue
        let topics = match result.get("topics") {
            Some(topics) => topics,
            None => continue,
        };

        // Get first topic or continue
        let first_topic = match topics.as_array().and_then(|t| t.first()) {
            Some(topic) => topic,
            None => continue,
        };

        // Check if it matches our sync topic
        if first_topic.as_str() != Some(SYNC_TOPIC) {
            continue;
        }

        // Process sync event
        let now = Local::now();
        println!("\n🔄 Sync Event Detected:");
        println!("------------------------");
        println!("⏰ Time: {}", now.format("%Y-%m-%d %H:%M:%S%.3f"));

        if let Some(tx_hash) = result.get("transactionHash") {
            println!("📝 Transaction: {tx_hash}");
        }

        if let Some(address) = result.get("address") {
            println!("📍 Pool Address: {address}");
        }

        // Decode the reserve data
        if let Some(data) = result.get("data").and_then(|d| d.as_str()) {
            let data = data.trim_start_matches("0x");
            if data.len() >= 128 {  // 2 * 32 bytes in hex
                let reserve0 = u128::from_str_radix(&data[0..64], 16)
                    .unwrap_or_default();
                let reserve1 = u128::from_str_radix(&data[64..128], 16)
                    .unwrap_or_default();

                println!("💰 Reserve0: {reserve0}");
                println!("💰 Reserve1: {reserve1}");
            }
        }

        if let Some(block_number) = result.get("blockNumber") {
            println!("🔢 Block: {block_number}");
        }
        println!("------------------------\n");
    }

    Ok(())
}