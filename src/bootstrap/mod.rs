pub mod types;

use crate::arb::pool::Pool;
use crate::bootstrap::types::{PairInfo, Reserves};
use crate::db_service::{DbManager, PairService};
use crate::models::factory::NewFactory;
use crate::models::token::NewToken;
use crate::utils::app_context::AppContext;
use crate::utils::constants::UNISWAP_V2_BATCH_QUERY_ADDRESS;

use alloy::{
    primitives::{Address, U256},
    sol,
};
use eyre::Report;
use log::{error, info};
use std::collections::HashSet;
use std::ops::Add;
use std::str::FromStr;
use futures_util::future::join_all;
use std::time::Duration;

sol!(
    // #[allow(missing_docs)]
    #[sol(rpc)]
    // UniswapQuery,
    "contracts/src/UniswapQuery.sol"
);

/// Retrieves pairs within a specified index range from a factory contract
///
/// # Arguments
/// * `factory` - The address of the factory contract
/// * `from` - Starting index
/// * `to` - Ending index
///
/// # Returns
/// A vector of `PairInfo` containing pair and token information
///
/// # Errors
/// * If HTTP provider creation fails
/// * If contract call fails
///
/// # Panics
/// * If application context creation fails
pub async fn fetch_pairs_v2_by_range(
    ctx: &AppContext,
    factory: Address,
    from: U256,
    to: U256,
) -> Result<Vec<PairInfo>, Report> {
    let uniswap_v2_batch_request =
        UniswapQuery::new(UNISWAP_V2_BATCH_QUERY_ADDRESS, &ctx.base_provider);

    Ok(uniswap_v2_batch_request
        .getPairsByIndexRange(factory, from, to)
        .gas(30_000_000)
        .call()
        .await?
        ._0
        .into_iter()
        .map(PairInfo::from)
        .collect())
}

/// Retrieves all pairs from a factory contract in batches
///
/// # Arguments
/// * `factory` - The address of the factory contract
/// * `batch_size` - Number of pairs to fetch in each batch
///
/// # Returns
/// A vector of tuples containing Factory, Token0, Token1, and Pair information
///
/// # Errors
/// * If HTTP provider creation fails
/// * If contract calls fail
/// * If database operations fail
///
/// # Panics
/// * If application context creation fails
/// * If database connection fails
pub async fn fetch_all_pairs_v2_by_factory(
    ctx: &mut AppContext,
    factory: Address,
    batch_size: u64,
) -> Result<(), eyre::Report> {
    // As discussed with pawel, we fetch all pairs
    let mut start = U256::from(0);

    let uniswap_v2_batch_request =
        UniswapQuery::new(UNISWAP_V2_BATCH_QUERY_ADDRESS, &ctx.base_provider);

    let pairs_len_block: U256 = uniswap_v2_batch_request
        .allPairsLength(factory)
        .gas(30_000_000)
        .call()
        .await?
        ._0;

    let pairs_len_db = PairService::count_pairs_by_factory_address(&mut ctx.pg_connection, factory.to_string().as_str())?;

    // Get existing pair addresses to avoid duplicates
    let existing_pairs = PairService::get_pair_addresses_by_factory(&mut ctx.pg_connection, factory.to_string())?;
    let existing_pairs_set: HashSet<String> = HashSet::from_iter(existing_pairs);

    if U256::from(pairs_len_db).eq(&pairs_len_block) {
        return Ok(());
    }

    info!("Start from index {start}, total pairs: {pairs_len_block}");

    let mut fetch_pair_task = Vec::new();
    while start < pairs_len_block {
        let end = (start.add(U256::from(batch_size))).min(pairs_len_block);

        // Process single batch
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        info!("Fetching pairs for range {start} to {end}");

        fetch_pair_task.push(fetch_pairs_v2_by_range(ctx, factory, start, end));
        info!("Add batch: {start} to {end}");

        start = end;
    }

    let pair_batches = join_all(fetch_pair_task).await;

    // Convert pairs to database format
    let mut dex_infos = Vec::new();
    let uniswap_factory = NewFactory {
        address: factory.to_string(),
        version: "2".parse()?,
        fee: 300,
        name: "Uniswap V2".parse()?,
    };
    for pairs in pair_batches {
        for pair in pairs? {
            if existing_pairs_set.contains(&pair.address) {
                continue;
            }

            let token0 = NewToken::new(
                pair.token0.address.to_string(),
                pair.token0.symbol,
                pair.token0.name,
                pair.token0.decimals,
            );
            let token1 = NewToken::new(
                pair.token1.address.to_string(),
                pair.token1.symbol,
                pair.token1.name,
                pair.token1.decimals,
            );
            dex_infos.push((
                uniswap_factory.clone(),
                token0,
                token1,
                pair.address.to_string(),
            ));
        }
    }

    // Save batch to database
    let _ = DbManager::batch_save_dex_info(&mut ctx.pg_connection, dex_infos);
    Ok(())
}

/// Retrieves reserves for a list of pairs
///
/// # Arguments
/// * `pairs` - Vector of pair addresses
///
/// # Returns
/// Vector of `Reserves` containing reserve information for each pair
///
/// # Errors
/// * If contract call to get reserves fails
/// * If batch request initialization fails
/// * If the RPC connection fails
///
/// # Panics
/// * If contract call to get reserves fails
/// * If batch request contract initialization fails
pub async fn fetch_reserves_by_range(
    ctx: &AppContext,
    pool_chunk: &[Pool],
) -> Result<Vec<Pool>, eyre::Report> {
    let uniswap_v2_batch_request =
        UniswapQuery::new(UNISWAP_V2_BATCH_QUERY_ADDRESS, &ctx.base_provider);

    let pair_addresses: Vec<Address> = pool_chunk
        .iter()
        .map(|pair| Address::from_str(&pair.id.to_string()).unwrap())
        .collect();
    let mut pools_to_replace = Vec::new();

    let reserves: Vec<Reserves> = uniswap_v2_batch_request
        .getReservesByPairs(pair_addresses)
        .gas(30_000_000)
        .call()
        .await?
        ._0
        .into_iter()
        .map(Into::into)
        .collect();

    for (i, pool) in pool_chunk.iter().enumerate() {
        let new_reserves = &reserves[i];

        let mut updated_pool = pool.clone();
        updated_pool.reserve0 = Some(new_reserves.reserve0);
        updated_pool.reserve1 = Some(new_reserves.reserve1);
        pools_to_replace.push(updated_pool);
    }

    Ok(pools_to_replace)
}

/// Retrieves reserves for all pairs in the database
///
/// # Arguments
/// * `batch_size` - Number of pairs to process in each batch
///
/// # Returns
/// Vector of tuples containing pair address and its reserves
///
/// # Panics
/// * If database connection fails
/// * If HTTP provider creation fails
/// * If contract calls fail
/// * If pair addresses cannot be parsed
pub async fn fetch_all_pools(ctx: &mut AppContext, batch_size: usize) -> Result<HashSet<Pool>, eyre::Report> {
    // Create context in a block to drop PgConnection before async operations
    let pools = PairService::load_all_pools(&mut ctx.pg_connection);
    let pools_clone: Vec<Pool> = pools.iter().cloned().collect();
    // let mut pools_to_replace = Vec::new();
    let mut result_pools = pools;
    let mut pool_reserve_tasks = Vec::new();

    // Process pairs in batches sequentially
    for mut pool_chunk in pools_clone.chunks(batch_size) {
        let addresses: Vec<Address> = pool_chunk
            .iter()
            .map(|pair| Address::from_str(&pair.id.to_string()).unwrap())
            .collect();

        pool_reserve_tasks.push(fetch_reserves_by_range(ctx, pool_chunk));
        // // Process single batch
        // let reserves = match fetch_reserves_by_range(ctx, addresses).await {
        //     Ok(reserves) => reserves,
        //     Err(e) => {
        //         error!("Error fetching reserves: {e}");
        //         continue;
        //     }
        // };

        // for (i, pool) in pool_chunk.iter().enumerate() {
        //     let new_reserves = &reserves[i];
        //
        //     // Remove old pool and insert updated one
        //     if result_pools.remove(pool) {
        //         let mut updated_pool = pool.clone();
        //         updated_pool.reserve0 = Some(new_reserves.reserve0);
        //         updated_pool.reserve1 = Some(new_reserves.reserve1);
        //         pools_to_replace.push(updated_pool);
        //     }
        // }

        // Add delay between batches
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    let pool_reserve_batch = join_all(pool_reserve_tasks).await;
    for pool_reserves in pool_reserve_batch {
        for pool in pool_reserves? {
            if result_pools.remove(&pool) {
                result_pools.insert(pool);
            }
        }
    }
    //
    // // Insert all updated pools at once (after iteration)
    // for pool in pools_to_replace {
    //     result_pools.insert(pool);
    // }

    Ok(result_pools)
}

use crate::db_service::FactoryService;
use std::time::Duration;

/// Start pool bootstrapping as a background task
pub fn start_pool_monitoring(time_interval_by_sec: u64) -> Result<(), eyre::Error> {
    info!("Starting pool bootstrapping background task");
    
    tokio::spawn(async move {
        // Create a new AppContext for the background task
        let mut ctx = match AppContext::new().await {
            Ok(ctx) => ctx,
            Err(e) => {
                error!("Failed to create AppContext: {}", e);
                return;
            }
        };

        // Wait a bit before starting to ensure the application is fully initialized
        tokio::time::sleep(Duration::from_secs(10)).await;

        loop {
            // Wait before next iteration
            tokio::time::sleep(Duration::from_secs(time_interval_by_sec)).await;

            info!("Starting pool monitoring cycle");

            // Get all factories from database
            let factories = match FactoryService::read_all_factories(&mut ctx.pg_connection) {
                factories => {
                    info!("Found {} factories to bootstrap", factories.len());
                    factories
                }
            };

            // Process each factory
            for factory in factories {
                info!("Monitoring new pools for factory: {} ({})", factory.name, factory.address);

                // Convert factory address string to Address type
                match Address::from_str(&factory.address) {
                    Ok(factory_address) => {
                        // fetch_all_pairs_v2 handles resuming from the last saved index
                        match fetch_all_pairs_v2_by_factory(&mut ctx, factory_address, 3000).await {
                            Ok(_) => info!("Successfully detect new pairs for factory: {}", factory.name),
                            Err(e) => error!("Failed to monitor new pairs for factory {}: {}", factory.name, e),
                        }
                    },
                    Err(e) => error!("Invalid factory address {}: {}", factory.address, e),
                }
            }

            // Update all pools with reserves
            info!("Updating pool reserves...");
            match fetch_all_pools(&mut ctx, 100).await {
                Ok(pools) => {
                    info!("Pool reserves updated successfully for {} pools", pools.len());
                }
                Err(e) => {
                    error!("Failed to update pool reserves: {}", e);
                }
            }

            info!("Pool monitoring cycle completed");
        }
    });

    Ok(())
}

// Bootstrap pools function that takes a mutable reference to AppContext
// pub async fn monitor_new_pools_by_background(ctx: &mut AppContext) -> Result<(), eyre::Error> {
//     info!("Starting pool monitoring cycle");
//
//     // Get all factories from database
//     let factories = FactoryService::read_all_factories(&mut ctx.pg_connection);
//
//     if factories.is_empty() {
//         info!("No factories found in the database");
//         return Ok(());
//     }
//
//     info!("Found {} factories to bootstrap", factories.len());
//
//     // Process each factory
//     for factory in factories {
//         info!("Monitoring new pools for factory: {} ({})", factory.name, factory.address);
//
//         // Convert factory address string to Address type
//         match Address::from_str(&factory.address) {
//             Ok(factory_address) => {
//                 // fetch_all_pairs_v2 handles resuming from the last saved index
//                 match fetch_all_pairs_v2(ctx, factory_address, 3000).await {
//                     Ok(_) => info!("Successfully detect new pairs for factory: {}", factory.name),
//                     Err(e) => error!("Failed to monitor new pairs for factory {}: {}", factory.name, e),
//                 }
//             },
//             Err(e) => error!("Invalid factory address {}: {}", factory.address, e),
//         }
//     }
//
//     // Update all pools with reserves
//     info!("Updating pool reserves...");
//     let pools = fetch_all_pools(ctx, 100).await;
//     info!("Pool reserves updated successfully for {} pools", pools.len());
//
//     info!("Pool monitoring cycle completed");
//     Ok(())
// }