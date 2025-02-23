use alloy::primitives::{address, U256};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::utils::signer::Order;
use crate::AppContext;

const TRADE_CHANNEL_SIZE: usize = 1000; // Adjust size as needed

#[derive(Clone)]
pub struct MempoolMonitor {
    // is_running: Arc<Mutex<bool>>,
    // filter: TradeFilter,
    #[allow(dead_code)]
    processor: Arc<TradeProcessor>,
}

pub struct TradeProcessor {
    tx: mpsc::Sender<Value>,
}

impl TradeProcessor {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(TRADE_CHANNEL_SIZE);

        // Spawn the trade processing worker
        tokio::spawn(async move {
            while let Some(trade) = rx.recv().await {
                // Spawn a new task for each trade
                tokio::spawn(async move {
                    // Do something with the trade
                    println!("Trade: {trade:?}");
                });
            }
        });

        Self { tx }
    }

    #[allow(dead_code)]
    async fn send_trade(&self, trade: Value) {
        if let Err(e) = self.tx.send(trade).await {
            eprintln!("Error sending trade to processor: {e}");
        }
    }
}

impl MempoolMonitor {
    pub const fn new(processor: Arc<TradeProcessor>) -> Self {
        Self {
            // filter,
            processor,
        }
    }

    pub async fn start(&self, _context: &mut AppContext) -> Result<(), Box<dyn std::error::Error>> {
        let tx = serde_json::json!({
            "tx_hash": "0x0",
        });
        self.processor.send_trade(tx).await;
        Ok(())
    }

    /// I image main bot loop to look something like this:
    #[allow(dead_code)]
    async fn bot_loop(&self, context: &mut AppContext) {
        loop {
            // tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            // Complicated logic that generates a bunch of transactions we want to send
            // This Message is just a placeholder. We'll need to come up with an actual interface
            let order = Order {
                amount: U256::from(1000),
                pools: vec![
                    address!("0x0000000000000000000000000000000000000000"),
                    address!("0x0000000000000000000000000000000000000001"),
                    address!("0x0000000000000000000000000000000000000002"),
                ],
            };

            println!("Posting order: {order:?}");
            match context.signer.call(&order).await {
                Ok(()) => {
                    println!("Sent order: {order:?}");
                    break;
                }
                Err(e) => {
                    eprintln!("Failed to send order: {e}");
                }
            }
        }
    }
}
