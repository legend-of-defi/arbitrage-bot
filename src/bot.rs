use crate::core::{MempoolMonitor, TradeProcessor};
use crate::utils::app_context::AppContext;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Bot {
    context: Mutex<AppContext>,
    mempool_monitor: Mutex<MempoolMonitor>,
}

impl Bot {
    pub fn new(ctx: AppContext) -> Self {
        let processor = Arc::new(TradeProcessor::new());
        Self {
            context: Mutex::new(ctx),
            mempool_monitor: Mutex::new(MempoolMonitor::new(processor)),
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut context = self.context.lock().await;
        self.mempool_monitor.lock().await.start(&mut context).await
    }
}
