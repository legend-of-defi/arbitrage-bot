use std::sync::Arc;

use eyre::Result;

use crate::sync;
use crate::utils::app_context::AppContext;

/// Start the bot
///
/// # Arguments
///
/// * `ctx` - The application context
///
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

    // Spawn WETH price sync task
    let ctx8 = Arc::clone(&ctx);
    tokio::spawn(async move {
        if let Err(e) = sync::weth(&ctx8).await {
            log::error!("{}", e);
        }
    });

    // Wait for all spawned tasks to complete
    tokio::signal::ctrl_c().await?;
    log::info!("Received shutdown signal, waiting for tasks to complete...");
    Ok(())
}
