use crate::models::factory::{Factory, FactoryStatus};
use crate::schemas::{factories, pairs};
use crate::utils::app_context::AppContext;
use alloy::primitives::{Address, Bytes, U256};
use alloy::providers::MULTICALL3_ADDRESS;
use alloy::sol;
use alloy::sol_types::{SolCall, SolValue};
use diesel::QueryDsl;
use diesel::{ExpressionMethods, SelectableHelper};
use diesel_async::RunQueryDsl;
use eyre::Result;

sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IMulticall3.sol"
}

sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IUniswapV2Factory.sol"
}

/// Syncs pairs created by factories
///
/// This function retrieves factory addresses from the database
/// and then fetches all pairs created by each factory.
pub async fn factory_pairs(ctx: &AppContext) -> Result<()> {
    log::info!("sync::factory_pairs: Starting factory pairs sync...");

    loop {
        let synced_pairs_count = sync(ctx).await?;

        if synced_pairs_count == 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

async fn sync(ctx: &AppContext) -> Result<usize> {
    let mut conn = ctx.db.get().await?;

    // First unsynced factory
    let mut results: Vec<Factory> = factories::table
        .filter(factories::status.eq(FactoryStatus::Unsynced))
        .limit(1)
        .select(Factory::as_select())
        .load(&mut conn)
        .await?;

    if results.is_empty() {
        return Ok(0);
    }

    let factory = &mut results[0];

    // Create factory contract instance
    let factory_contract = IUniswapV2Factory::new(factory.address(), &ctx.base_provider);

    // Get total number of pairs
    let pairs_length = match factory_contract.allPairsLength().call().await {
        Ok(length) => length._0.to::<i32>(),
        Err(e) => {
            log::error!("sync::factory_pairs: Failed to get pairs length: {}", e);
            factory
                .update_status(&mut conn, FactoryStatus::Broken)
                .await?;
            return Ok(0);
        }
    };

    log::info!(
        "sync::factory_pairs: Factory {} has {} pairs, last processed: {}",
        factory.address(),
        pairs_length,
        factory.last_pair_id()
    );

    // All pairs already processed, mark factory as synced
    if factory.last_pair_id() >= pairs_length {
        diesel::update(factories::table)
            .filter(factories::id.eq(factory.id()))
            .set(factories::status.eq(FactoryStatus::Synced))
            .execute(&mut conn)
            .await?;
        return Ok(0);
    }

    // Multicall3 instance
    let multicall = IMulticall3::new(MULTICALL3_ADDRESS, &ctx.base_provider);

    // Arbitrary number, can be changed
    let multicall_batch_size = 100;

    // Calculate how many pairs to fetch
    let start_id = factory.last_pair_id() as usize;
    let end_id = std::cmp::min(pairs_length as usize, start_id + multicall_batch_size);
    let pair_indexes = (start_id..end_id).collect::<Vec<usize>>();

    // Prepare multicall calls
    let calls: Vec<IMulticall3::Call3> = pair_indexes
        .iter()
        .map(|pair| IMulticall3::Call3 {
            target: *factory_contract.address(),
            allowFailure: true,
            callData: Bytes::from(
                IUniswapV2Factory::allPairsCall::new((U256::from(*pair),)).abi_encode(),
            ),
        })
        .collect();

    // Execute multicall
    let multicall_result = match multicall.aggregate3(calls).call().await {
        Ok(result) => result,
        Err(e) => {
            log::error!("sync::factory_pairs: Multicall failed: {}", e);
            return Ok(0);
        }
    };

    // Process results
    for (return_index, pair_index) in pair_indexes.iter().enumerate() {
        let result = &multicall_result.returnData[return_index];
        if !result.success {
            log::warn!(
                "sync::factory_pairs: Failed to get pair at index {} for factory {}",
                pair_index,
                factory.address()
            );
            continue;
        }
        let pair_address = Address::abi_decode(&result.returnData, true);

        if let Ok(pair_address) = pair_address {
            // Upsert pair into database
            diesel::insert_into(pairs::table)
                .values((
                    pairs::address.eq(pair_address.to_string()),
                    pairs::factory_id.eq(factory.id()),
                ))
                .on_conflict(pairs::address)
                .do_update()
                .set(pairs::factory_id.eq(factory.id()))
                .execute(&mut conn)
                .await?;
        } else {
            log::warn!(
                "sync::factory_pairs: Failed to decode pair address at index {} for factory {}",
                pair_index,
                factory.address()
            );
            continue;
        }
    }

    // Update factory's last_pair_id
    diesel::update(factories::table)
        .filter(factories::id.eq(factory.id()))
        .set(factories::last_pair_id.eq(end_id as i32))
        .execute(&mut conn)
        .await?;

    log::info!(
        "sync::factory_pairs: Synced {} pairs from factory {}",
        pair_indexes.len(),
        factory.address()
    );

    Ok(pair_indexes.len())
}
