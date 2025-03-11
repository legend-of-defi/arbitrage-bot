use std::sync::Arc;

use eyre::Result;
pub use serde_json::Value;
use tokio::sync::mpsc;

use crate::sync;
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
                    log::info!("Trade: {trade:?}");
                });
            }
        });

        Self { tx }
    }

    async fn send_trade(&self, trade: Value) {
        if let Err(e) = self.tx.send(trade).await {
            log::error!("Error sending trade to processor: {e}");
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

    pub async fn start(&self, _context: &mut AppContext) -> Result<()> {
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
            log::info!("Bot is working...");
        }
    }
}

pub async fn start(ctx: AppContext) -> Result<()> {
    let ctx = Arc::new(ctx);

    // Spawn events sync task
    let ctx1 = Arc::clone(&ctx);
    tokio::spawn(async move {
        if let Err(e) = sync::events(&ctx1).await {
            log::error!("{}", e);
        }
    });

    // Spawn reserve sync task
    let ctx2 = Arc::clone(&ctx);
    tokio::spawn(async move {
        if let Err(e) = sync::reserves(&ctx2).await {
            log::error!("{}", e);
        }
    });

    // Spawn pair tokens sync task
    let ctx3 = Arc::clone(&ctx);
    tokio::spawn(async move {
        if let Err(e) = sync::pair_tokens(&ctx3).await {
            log::error!("{}", e);
        }
    });

    // Spawn factories sync task
    let ctx4 = Arc::clone(&ctx);
    tokio::spawn(async move {
        if let Err(e) = sync::factories(&ctx4).await {
            log::error!("{}", e);
        }
    });

    // Spawn USD value sync task
    let ctx5 = Arc::clone(&ctx);
    tokio::spawn(async move {
        if let Err(e) = sync::usd(&ctx5).await {
            log::error!("{}", e);
        }
    });

    // Spawn exchange rates sync task
    let ctx6 = Arc::clone(&ctx);
    tokio::spawn(async move {
        if let Err(e) = sync::exchange_rates(&ctx6).await {
            log::error!("{}", e);
        }
    });

    // Spawn factory pairs sync task
    let ctx7 = Arc::clone(&ctx);
    tokio::spawn(async move {
        if let Err(e) = sync::factory_pairs(&ctx7).await {
            log::error!("{}", e);
        }
    });

    // Wait for all spawned tasks to complete
    tokio::signal::ctrl_c().await?;
    log::info!("Received shutdown signal, waiting for tasks to complete...");
    Ok(())
}
