use alloy::rpc::types::BlockNumberOrTag;
use alloy::{
    primitives::Address, providers::Provider, rpc::types::Filter, sol, sol_types::SolEvent,
};
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;
use eyre::Result;
use futures::StreamExt;

use crate::schemas::tokens::{self};
use crate::{schemas::pairs, utils::app_context::AppContext};

// Event emitted when a pair is created.
sol! {
    event PairCreated(
        address indexed token0,
        address indexed token1,
        address pair,
        uint256
    );
}

/// Sync pair created events.
/// These are emitted by UniswapV2Factory contracts.
pub async fn pair_created_events(ctx: &AppContext) -> Result<()> {
    let mut conn = ctx.db.get().await?;
    let provider = &ctx.base_provider;

    let filter = Filter::new()
        .event(PairCreated::SIGNATURE)
        .from_block(BlockNumberOrTag::Latest);
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
        let event = match PairCreated::decode_log(&log.inner, true) {
            Ok(event) => event,
            Err(e) => {
                log::error!("sync::events: Failed to decode event: {e}");
                continue;
            }
        };

        let token0_id = token_id_by_address(ctx, event.token0).await?;
        let token1_id = token_id_by_address(ctx, event.token1).await?;

        diesel::insert_into(pairs::table)
            .values((
                pairs::address.eq(event.pair.to_string()),
                pairs::token0_id.eq(token0_id),
                pairs::token1_id.eq(token1_id),
            ))
            .execute(&mut conn)
            .await?;
    }

    Ok(())
}

/// Get the token id for a given address. If the token does not exist, it will be created.
async fn token_id_by_address(ctx: &AppContext, token_address: Address) -> Result<i32> {
    let mut conn = ctx.db.get().await?;
    log::info!("token_id_by_address: {}", token_address);

    let id = diesel::insert_into(tokens::table)
        .values((tokens::address.eq(token_address.to_string()),))
        .on_conflict(tokens::address)
        .do_update()
        .set(tokens::address.eq(token_address.to_string()))
        .returning(tokens::id)
        .get_result::<i32>(&mut conn)
        .await?;
    Ok(id)
}
