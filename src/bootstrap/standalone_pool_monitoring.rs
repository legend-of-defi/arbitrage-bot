use crate::db_service::FactoryService;
use std::time::Duration;
use log::{error, info};
use std::str::FromStr;
use alloy::primitives::Address;
use eyre::Result;

/// Start pool monitoring as a background task
///
/// This function doesn't capture any context - it creates its own as needed.
pub fn start_pool_monitoring(time_interval_secs: u64) -> Result<(), eyre::Error> {
    info!("Starting pool bootstrapping background task");

    // Spawn a completely isolated background thread with no external references
    std::thread::spawn(move || {
        // Set up the async runtime for this thread
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build runtime");

        rt.block_on(async {
            // Initial delay to ensure application is fully initialized
            tokio::time::sleep(Duration::from_secs(5)).await;

            loop {
                run_single_monitoring_cycle().await;
                tokio::time::sleep(Duration::from_secs(time_interval_secs)).await;
            }
        });
    });

    Ok(())
}

/// Run a single monitoring cycle with its own isolated context
async fn run_single_monitoring_cycle() {
    info!("Starting pool monitoring cycle");

    // Create a new context just for this cycle
    match crate::utils::app_context::AppContext::new().await {
        Ok(mut ctx) => {
            // Process factories
            let factories = FactoryService::read_all_factories(&mut ctx.pg_connection);
            info!("Found {} factories to bootstrap", factories.len());

            // Process each factory
            for factory in factories {
                info!("Processing factory: {} ({})", factory.name, factory.address);

                if let Ok(factory_address) = Address::from_str(&factory.address) {
                    if let Err(e) = crate::bootstrap::fetch_all_pairs_v2_by_factory(
                        &mut ctx,
                        factory_address,
                        3000
                    ).await {
                        error!("Error processing factory {}: {}", factory.name, e);
                    }
                }
            }

            // Update pool reserves
            info!("Updating pool reserves...");
            match crate::bootstrap::fetch_all_pools(&mut ctx, 100).await {
                Ok(pools) => info!("Updated reserves for {} pools", pools.len()),
                Err(e) => error!("Failed to update pool reserves: {}", e),
            }
        },
        Err(e) => {
            error!("Failed to create context: {}", e);
        }
    }

    info!("Pool monitoring cycle completed");
}