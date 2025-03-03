pub use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::utils::app_context::AppContext;

const TRADE_CHANNEL_SIZE: usize = 1000; // Adjust size as needed

#[derive(Clone)]
pub struct MempoolMonitor {
    // is_running: Arc<Mutex<bool>>,
    // filter: TradeFilter,
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
    async fn bot_loop(&self, _context: &mut AppContext) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            println!("Bot is working...");
        }
    }
}
