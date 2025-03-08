pub mod types;

use crate::arb::pool::Pool;
use crate::bootstrap::types::{PairInfo, Reserves};
use crate::utils::app_context::AppContext;
use crate::utils::constants::UNISWAP_V2_BATCH_QUERY_ADDRESS;

use alloy::{
    primitives::{Address, U256},
    sol,
};
use bigdecimal::BigDecimal;
use eyre::Error;
use std::collections::HashSet;
use std::str::FromStr;

sol!(
    #[sol(rpc)]
    "contracts/src/UniswapQuery.sol"
);

/// Convert a U256 to a f64
///
/// # Arguments
/// * `value` - The U256 value to convert
///
/// # Returns
/// The f64 value
#[allow(dead_code)]
fn u256_to_f64(value: U256) -> f64 {
    let s: String = value.to_string(); // Convert U256 to string
    s.parse::<f64>().unwrap_or(0.0) // Parse it into f64
}

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
) -> Result<Vec<PairInfo>, Error> {
    let uniswap_v2_batch_request =
        UniswapQuery::new(UNISWAP_V2_BATCH_QUERY_ADDRESS, &ctx.base_provider);

    Ok(uniswap_v2_batch_request
        .getPairsByIndexRange(factory, from, to)
        .gas(3_000_000_000)
        .call()
        .await?
        ._0
        .into_iter()
        .map(PairInfo::from)
        .collect())
}

/// Calculate reserves and USD value for a pair
///
/// # Arguments
/// * `pair` - Pair information
/// * `reserve` - Reserves for the pair
///
/// # Returns
/// Tuple containing token0 reserve, token1 reserve, and USD value
#[allow(dead_code)]
fn calculate_reserves_and_usd(
    pair: &PairInfo,
    reserve: &Reserves,
) -> (BigDecimal, BigDecimal, i32) {
    // Calculate human-readable reserve values
    let reserve0_decimal = u256_to_f64(reserve.reserve0) / 10_f64.powi(pair.token0.decimals());
    let reserve1_decimal = u256_to_f64(reserve.reserve1) / 10_f64.powi(pair.token1.decimals());

    // Convert to BigDecimal for database storage
    let token0_reserve =
        BigDecimal::from_str(&reserve0_decimal.to_string()).unwrap_or_else(|_| BigDecimal::from(0));
    let token1_reserve =
        BigDecimal::from_str(&reserve1_decimal.to_string()).unwrap_or_else(|_| BigDecimal::from(0));

    // Calculate USD value
    let mut usd_value: i32 = 0;

    // Hardcoded token addresses and prices
    let weth_address = "0x4200000000000000000000000000000000000006".to_lowercase();
    let usdc_address = "0xd9fcd98c322942075a5c3860693e9f4f03aae07b".to_lowercase();
    let usdt_address = "0x2f4d3d3f2f3d3f2f4d3d3f2f4d3d3f2f4d3d3f2f".to_lowercase();
    let dai_address = "0x50c5725949a6f0c72e6c4a641f24049a917db0cb".to_lowercase();

    // Check token0
    let token0_address = pair.token0.address().to_string().to_lowercase();
    let token0_symbol = pair.token0.symbol().unwrap_or_default().to_uppercase();

    let token0_price = match token0_address.as_str() {
        addr if addr == weth_address || token0_symbol == "WETH" => 2118.14,
        addr if addr == usdc_address || token0_symbol == "USDC" => 1.0,
        addr if addr == usdt_address || token0_symbol == "USDT" => 1.0,
        addr if addr == dai_address || token0_symbol == "DAI" => 1.0,
        _ => 0.0,
    };

    if token0_price > 0.0 {
        let token0_usd = reserve0_decimal * token0_price;
        // Multiply by 2 to represent total reserve
        let total_usd = token0_usd * 2.0;
        usd_value = total_usd as i32; // Store as whole dollars
    }

    // Check token1 if token0 didn't match
    if usd_value == 0 {
        let token1_address = pair.token1.address().to_string().to_lowercase();
        let token1_symbol = pair.token1.symbol().unwrap_or_default().to_uppercase();

        let token1_price = match token1_address.as_str() {
            addr if addr == weth_address || token1_symbol == "WETH" => 2118.14,
            addr if addr == usdc_address || token1_symbol == "USDC" => 1.0,
            addr if addr == usdt_address || token1_symbol == "USDT" => 1.0,
            addr if addr == dai_address || token1_symbol == "DAI" => 1.0,
            _ => 0.0,
        };

        if token1_price > 0.0 {
            let token1_usd = reserve1_decimal * token1_price;
            // Multiply by 2 to represent total reserve
            let total_usd = token1_usd * 2.0;
            usd_value = total_usd as i32; // Store as whole dollars
        }
    }

    (token0_reserve, token1_reserve, usd_value)
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
    pairs: Vec<Address>,
) -> Result<Vec<Reserves>, eyre::Report> {
    let uniswap_v2_batch_request =
        UniswapQuery::new(UNISWAP_V2_BATCH_QUERY_ADDRESS, &ctx.base_provider);

    Ok(uniswap_v2_batch_request
        .getReservesByPairs(pairs)
        .gas(3_000_000_000)
        .call()
        .await?
        ._0
        .into_iter()
        .map(Into::into)
        .collect())
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
pub async fn fetch_all_pools(_ctx: &mut AppContext, _batch_size: usize) -> HashSet<Pool> {
    todo!();
    // Create context in a block to drop PgConnection before async operations
    // let pools = PairService::load_all_pools(&mut ctx.pg_connection);
    // let pools_clone: Vec<Pool> = pools.iter().cloned().collect();
    // let mut pools_to_replace = Vec::new();
    // let mut result_pools = pools;

    // let total_pools = pools_clone.len();

    // // Create a progress bar
    // let progress_bar = ProgressBar::new(total_pools as u64);

    // // Set a custom style for the progress bar
    // progress_bar.set_style(
    //     ProgressStyle::default_bar()
    //         .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) - {msg}")
    //         .unwrap()
    //         .progress_chars("#>-")
    // );

    // // Set initial message
    // progress_bar.set_message("Starting pool reserve updates...");

    // // Track start time
    // let start_time = Instant::now();
    // let mut updated_count = 0;
    // let mut error_count = 0;

    // // Process pairs in batches sequentially
    // for (chunk_index, pool_chunk) in pools_clone.chunks(batch_size).enumerate() {
    //     let chunk_start = chunk_index * batch_size;
    //     let chunk_end = chunk_start + pool_chunk.len();

    //     progress_bar.set_message(format!(
    //         "Fetching reserves for batch {}/{} (pools {}-{})",
    //         chunk_index + 1,
    //         (total_pools as f64 / batch_size as f64).ceil() as usize,
    //         chunk_start,
    //         chunk_end
    //     ));

    //     let addresses: Vec<Address> = pool_chunk
    //         .iter()
    //         .map(|pair| Address::from_str(&pair.id.to_string()).unwrap())
    //         .collect();

    //     // Process single batch
    //     let reserves = match fetch_reserves_by_range(ctx, addresses).await {
    //         Ok(reserves) => reserves,
    //         Err(e) => {
    //             error!("Error fetching reserves: {e}");
    //             // Update progress bar for skipped pools
    //             progress_bar.inc(pool_chunk.len() as u64);
    //             error_count += pool_chunk.len();
    //             progress_bar.set_message(format!(
    //                 "Error fetching reserves for batch {}: {}",
    //                 chunk_index + 1,
    //                 e
    //             ));
    //             continue;
    //         }
    //     };

    //     progress_bar.set_message(format!(
    //         "Processing batch {}/{} (pools {}-{})",
    //         chunk_index + 1,
    //         (total_pools as f64 / batch_size as f64).ceil() as usize,
    //         chunk_start,
    //         chunk_end
    //     ));

    //     for (i, pool) in pool_chunk.iter().enumerate() {
    //         if i >= reserves.len() {
    //             progress_bar.inc(1);
    //             error_count += 1;
    //             continue;
    //         }

    //         let new_reserves = &reserves[i];

    //         // Remove old pool and insert updated one
    //         if result_pools.remove(pool) {
    //             let mut updated_pool = pool.clone();
    //             updated_pool.reserve0 = Some(new_reserves.reserve0);
    //             updated_pool.reserve1 = Some(new_reserves.reserve1);
    //             pools_to_replace.push(updated_pool);
    //             updated_count += 1;
    //         } else {
    //             error_count += 1;
    //         }

    //         // Update progress
    //         progress_bar.inc(1);

    //         // Calculate and display stats
    //         let elapsed = start_time.elapsed();
    //         let processed = progress_bar.position();
    //         if processed > 0 {
    //             let pools_per_second = processed as f64 / elapsed.as_secs_f64();
    //             let remaining = total_pools as u64 - processed;
    //             let eta_seconds = if pools_per_second > 0.0 {
    //                 remaining as f64 / pools_per_second
    //             } else {
    //                 0.0
    //             };

    //             progress_bar.set_message(format!(
    //                 "Batch {}/{} | Speed: {:.2} pools/sec | Updated: {} | Errors: {} | ETA: {:.2} min",
    //                 chunk_index + 1,
    //                 (total_pools as f64 / batch_size as f64).ceil() as usize,
    //                 pools_per_second,
    //                 updated_count,
    //                 error_count,
    //                 eta_seconds / 60.0
    //             ));
    //         }
    //     }

    //     // Add delay between batches
    //     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    // }

    // Finish the progress bar
    // progress_bar.finish_with_message(format!(
    //     "Pool reserve updates completed in {:.2} minutes! Updated: {} pools, Errors: {} pools",
    //     start_time.elapsed().as_secs_f64() / 60.0,
    //     updated_count,
    //     error_count
    // ));

    // // Insert all updated pools at once (after iteration)
    // for pool in pools_to_replace {
    //     result_pools.insert(pool);
    // }

    // result_pools
}
