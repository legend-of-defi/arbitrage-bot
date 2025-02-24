use serde_json::{json, Value};
use std::error::Error;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures::StreamExt;
use chrono::Local;

pub async fn subscribe_to_sync() -> Result<(), Box<dyn Error>> {
    let subscribe_request = json!({
        "jsonrpc": "2.0",
        "method": "eth_subscribe",
        "params": ["logs"],
        "id": 1
    });

    let mut ws_stream = crate::websocket::ws_client::send_ws_request(subscribe_request.to_string()).await?;

    // Sync event topic
    const SYNC_TOPIC: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";

    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    // Check if it's a sync event
                    if let Some(params) = json.get("params") {
                        if let Some(result) = params.get("result") {
                            if let Some(topics) = result.get("topics") {
                                if let Some(first_topic) = topics.as_array().and_then(|t| t.first()) {
                                    // Only process if it matches our sync topic
                                    if first_topic.as_str() == Some(SYNC_TOPIC) {
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
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error receiving message: {:?}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}