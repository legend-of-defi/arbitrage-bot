use crate::models::pair::Pair;
use crate::schemas::{factories, pairs};
use crate::utils::app_context::AppContext;
use alloy::primitives::{Address, Bytes};
use alloy::providers::MULTICALL3_ADDRESS;
use alloy::sol;
use alloy::sol_types::{SolCall, SolValue};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use eyre::Result;

sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IMulticall3.sol"
}

sol! {
    #[sol(abi)]
    "contracts/src/interfaces/IUniswapV2Pair.sol"
}

/// Sync factories for pairs
/// # Errors
/// Returns an error if the database connection fails
///
/// # Returns
/// Returns the number of pairs synced
pub async fn factories(ctx: &AppContext) -> Result<()> {
    log::info!("sync::factories: Starting factories sync...");

    loop {
        let synced_tokens_count = sync(ctx, 100).await?;

        if synced_tokens_count == 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

/// Sync factories for a batch of pairs
/// # Errors
/// Returns an error if the database connection fails
///
/// # Returns
/// Returns the number of pairs synced
async fn sync(ctx: &AppContext, limit: i64) -> Result<usize> {
    let mut conn = ctx.db.get().await?;

    // Pairs missing factory_id
    let pairs: Vec<Pair> = pairs::table
        .filter(pairs::factory_id.is_null())
        .select(Pair::as_select())
        .limit(limit)
        .load(&mut conn)
        .await?;

    // Multicall3 instance
    let multicall = IMulticall3::new(MULTICALL3_ADDRESS, &ctx.base_provider);

    // Create calls for each pair
    let calls: Vec<IMulticall3::Call3> = pairs
        .iter()
        .map(|p| IMulticall3::Call3 {
            target: p.address(),
            allowFailure: true,
            callData: Bytes::from(IUniswapV2Pair::factoryCall::new(()).abi_encode()),
        })
        .collect();

    // Execute calls
    let result = multicall.aggregate3(calls).call().await?;

    // Update pairs with factory_id
    for (index, pair) in pairs.iter().enumerate() {
        if result.returnData[index].success {
            if let Ok(factory_address) =
                Address::abi_decode(&result.returnData[index].returnData, true)
            {
                // Upsert factory_address to factories table
                let factory_id = diesel::insert_into(factories::table)
                    .values(factories::address.eq(factory_address.to_string()))
                    .on_conflict(factories::address)
                    .do_update()
                    .set(factories::address.eq(factory_address.to_string()))
                    .returning(factories::id)
                    .get_result::<i32>(&mut conn)
                    .await?;

                // Update pair with factory_id
                diesel::update(pairs::table.find(pair.id()))
                    .set(pairs::factory_id.eq(factory_id))
                    .execute(&mut conn)
                    .await?;
            }
        }
    }

    Ok(pairs.len())
}
