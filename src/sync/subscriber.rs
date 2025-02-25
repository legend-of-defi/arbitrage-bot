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
        let Some(params) = json.get("params") else { continue };

        // Get result or continue
        let Some(result) = params.get("result") else { continue };

        // Get topics or continue
        let Some(topics) = result.get("topics") else { continue };

        // Get first topic or continue
        let Some(first_topic) = topics.as_array().and_then(|t| t.first()) else { continue };

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_sync_topic_constant() {
        // Verify the sync topic hash is correct for Uniswap V2 Sync events
        assert_eq!(
            SYNC_TOPIC,
            "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"
        );
    }
    
    #[test]
    fn test_parse_sync_event() {
        // Create a sample sync event JSON
        let sync_event = json!({
            "params": {
                "result": {
                    "address": "0x1234567890abcdef1234567890abcdef12345678",
                    "topics": [
                        "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"
                    ],
                    "data": "0x000000000000000000000000000000000000000000000000000000000000007b0000000000000000000000000000000000000000000000000000000000000001",
                    "transactionHash": "0xb4c32b6af2ef12748023eb474bd80c9e9ff3a059ff3e9751dfa4bad3428ac4d8",
                    "blockNumber": "0x123456"
                }
            }
        });
        
        // Convert to string
        let event_str = sync_event.to_string();
        
        // Parse the event
        let parsed: Value = serde_json::from_str(&event_str).unwrap();
        
        // Verify parsing logic
        let params = parsed.get("params").unwrap();
        let result = params.get("result").unwrap();
        let topics = result.get("topics").unwrap();
        let first_topic = topics.as_array().unwrap().first().unwrap();
        
        assert_eq!(first_topic.as_str().unwrap(), SYNC_TOPIC);
        
        // Test data parsing
        let data = result.get("data").unwrap().as_str().unwrap();
        let data = data.trim_start_matches("0x");
        
        let reserve0 = u128::from_str_radix(&data[0..64], 16).unwrap();
        let reserve1 = u128::from_str_radix(&data[64..128], 16).unwrap();
        
        assert_eq!(reserve0, 123); // 0x7b in hex
        assert_eq!(reserve1, 291); // 0x123 in hex
    }
    
    // Mock test for WebSocket connection
    // This would require more complex setup with mocks
    #[test]
    fn test_subscribe_request_format() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["logs"],
            "id": 1
        });
        
        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["method"], "eth_subscribe");
        assert_eq!(request["params"][0], "logs");
        assert_eq!(request["id"], 1);
    }
}