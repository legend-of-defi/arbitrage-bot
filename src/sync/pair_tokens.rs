use alloy::primitives::Address;
use eyre::Result;
use log::info;

use crate::models::pair::Pair;
use crate::schemas::{pairs, tokens};
use crate::utils::app_context::AppContext;
use diesel::QueryDsl;
use diesel::SelectableHelper;
use diesel::{BoolExpressionMethods, ExpressionMethods};
use diesel_async::RunQueryDsl;

use alloy::sol;

sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IERC20.sol"

}

sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IUniswapV2Pair.sol"
}

/// Sync pairs tokens
/// Reads pairs from the database that don't have tokens, reads pair's contract and fetches
/// token info
pub async fn pair_tokens(ctx: &AppContext) -> Result<()> {
    log::info!("sync::pair_tokens: Starting token sync...");

    loop {
        let synced_tokens_count = sync(ctx, 100).await?;

        if synced_tokens_count == 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

/// Sync a bunch of pairs tokens
async fn sync(ctx: &AppContext, limit: i64) -> Result<usize> {
    let mut conn = ctx.db.get().await?;

    // Query for pairs missing token info
    let pairs: Vec<Pair> = pairs::table
        .filter(pairs::token0_id.is_null().or(pairs::token1_id.is_null()))
        .select(Pair::as_select())
        .limit(limit)
        .load::<Pair>(&mut conn)
        .await?;

    info!(
        "sync::pair_tokens(): Found {} pairs missing tokens info",
        pairs.len()
    );
    for pair in pairs.iter().as_ref() {
        // Read token addresses from pair contract
        let contract = IUniswapV2Pair::new(pair.address(), ctx.base_provider.clone());
        let token0 = contract.token0().call().await?._0;
        let token1 = contract.token1().call().await?._0;
        log::info!(
            "sync::pair_tokens: Syncing pair tokens for pair: {}, token0: {}, token1: {}",
            pair.address(),
            token0,
            token1
        );

        sync_pair_tokens(ctx, pair, token0, true).await?;
        sync_pair_tokens(ctx, pair, token1, false).await?;
    }

    Ok(pairs.len())
}

/// Sync a pair tokens
async fn sync_pair_tokens(
    ctx: &AppContext,
    pair: &Pair,
    token: Address,
    is_token0: bool,
) -> Result<()> {
    let mut conn = ctx.db.get().await?;

    // Create IERC20 contract instances for token
    let token_contract = IERC20::new(token, ctx.base_provider.clone());

    // Get token details
    let name = token_contract.name().call().await?._0.clone();
    let symbol = token_contract.symbol().call().await?._0.clone();
    let decimals = token_contract.decimals().call().await?._0;

    // Upsert token and get its ID
    let token_id = diesel::insert_into(tokens::table)
        .values((
            tokens::address.eq(token.to_string()),
            tokens::name.eq(&name),
            tokens::symbol.eq(&symbol),
            tokens::decimals.eq(i32::from(decimals)),
        ))
        .on_conflict(tokens::address)
        .do_update()
        .set((
            tokens::name.eq(&name),
            tokens::symbol.eq(&symbol),
            tokens::decimals.eq(i32::from(decimals)),
        ))
        .returning(tokens::id)
        .get_result::<i32>(&mut conn)
        .await?;

    if is_token0 {
        diesel::update(pairs::table.find(pair.id))
            .set(pairs::token0_id.eq(token_id))
            .execute(&mut conn)
            .await?;
    } else {
        diesel::update(pairs::table.find(pair.id))
            .set(pairs::token1_id.eq(token_id))
            .execute(&mut conn)
            .await?;
    }

    Ok(())
}
