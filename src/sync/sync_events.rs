use alloy::{
    eips::BlockNumberOrTag, providers::Provider, rpc::types::Filter, sol, sol_types::SolEvent,
};

use diesel::dsl::{exists, sql};
use diesel::sql_types::{Nullable, Numeric};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use eyre::Result;
use futures::StreamExt;

use crate::schemas::pairs;
use crate::utils::app_context::AppContext;

sol! {
    event Sync(
        uint112 reserve0,
        uint112 reserve1
    );
}

/// Subscribes to sync events from the network
///
/// Listens for Sync events from Uniswap V2 pairs and processes reserve updates
///
/// # Returns
/// * `Result<()>` - Ok(()) on successful subscription
///
/// # Errors
/// * If WebSocket connection cannot be established
/// * If subscription request fails
/// * If message parsing fails
/// * If network connection is lost
/// * If received message format is invalid
/// * If WebSocket stream terminates unexpectedly
/// * If message sending fails
pub async fn events(ctx: &AppContext) -> Result<()> {
    let provider = &ctx.base_provider;
    let filter = Filter::new()
        .event(Sync::SIGNATURE)
        .from_block(BlockNumberOrTag::Latest);

    // Get a database connection
    let mut conn = loop {
        match ctx.db.get().await {
            Ok(conn) => break conn,
            Err(e) => {
                log::error!("sync::events: Failed to get database connection: {e}");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    };

    // Subscribe to sync events
    let mut stream = loop {
        match provider.subscribe_logs(&filter).await {
            Ok(sub) => break sub.into_stream(),
            Err(e) => {
                log::error!("sync::events: Failed to subscribe to logs: {e}");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    };

    // Process sync events
    while let Some(log) = stream.next().await {
        // Process sync event
        let sync = match Sync::decode_log(&log.inner, true) {
            Ok(sync) => sync,
            Err(e) => {
                log::error!("sync::events: Failed to decode sync event: {e}");
                continue;
            }
        };

        let address = log.address();

        // Check if pair exists
        let pair_exists = diesel::select(exists(
            pairs::table.filter(pairs::address.eq(address.to_string())),
        ))
        .get_result::<bool>(&mut conn)
        .await?;

        if pair_exists {
            // Update pair reserves
            diesel::update(pairs::table.filter(pairs::address.eq(address.to_string())))
                .set((
                    pairs::reserve0.eq(sql::<Nullable<Numeric>>(&sync.reserve0.to_string())),
                    pairs::reserve1.eq(sql::<Nullable<Numeric>>(&sync.reserve1.to_string())),
                ))
                .execute(&mut conn)
                .await?;
            log::info!(
                "sync::events: Updated {} pair with {}/{} reserves",
                address,
                sync.reserve0,
                sync.reserve1
            );
        } else {
            // Insert new pair with reserves
            diesel::insert_into(pairs::table)
                .values((
                    pairs::address.eq(address.to_string()),
                    pairs::reserve0.eq(sql::<Nullable<Numeric>>(&sync.reserve0.to_string())),
                    pairs::reserve1.eq(sql::<Nullable<Numeric>>(&sync.reserve1.to_string())),
                ))
                .execute(&mut conn)
                .await?;

            log::info!(
                "sync::events: Inserted new {} pair with {}/{} reserves",
                address,
                sync.reserve0,
                sync.reserve1
            );
        }
    }

    Ok(())
}
