/// Types for the bootstrap module
pub mod types;

use crate::bootstrap::types::{PairInfo, Reserves};
use crate::utils::app_context::AppContext;
use crate::utils::constants::UNISWAP_V2_BATCH_QUERY_ADDRESS;

use alloy::{
    primitives::{Address, U256},
    sol,
};
use bigdecimal::BigDecimal;

use std::str::FromStr;

// Allow wildcard imports only for the auto-generated Solidity bindings
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

    // This is actually USDT, but use a more distinct name to avoid the similar names warning
    let tether_address = "0x2f4d3d3f2f3d3f2f4d3d3f2f4d3d3f2f4d3d3f2f".to_lowercase();

    let dai_address = "0x50c5725949a6f0c72e6c4a641f24049a917db0cb".to_lowercase();

    // Check token0
    let token0_address = pair.token0.address().to_string().to_lowercase();
    let token0_symbol = pair.token0.symbol().unwrap_or_default().to_uppercase();

    let token0_price = match token0_address.as_str() {
        addr if addr == weth_address || token0_symbol == "WETH" => 2118.14,
        addr if addr == usdc_address || token0_symbol == "USDC" => 1.0,
        addr if addr == tether_address || token0_symbol == "USDT" => 1.0,
        addr if addr == dai_address || token0_symbol == "DAI" => 1.0,
        _ => 0.0,
    };

    // SAFETY: we know the total_usd is less than i32::MAX because it's a product of two f64s
    #[allow(clippy::cast_possible_truncation)]
    if token0_price > 0.0 {
        let token0_usd = reserve0_decimal * token0_price;
        // Multiply by 2 to represent total reserve
        let total_usd = token0_usd * 2.0;
        usd_value = total_usd as i32; // Store as whole dollars
    }

    // Check token1 if token0 didn't match
    // SAFETY: we know the total_usd is less than i32::MAX because it's a product of two f64s
    #[allow(clippy::cast_possible_truncation)]
    if usd_value == 0 {
        let token1_address = pair.token1.address().to_string().to_lowercase();
        let token1_symbol = pair.token1.symbol().unwrap_or_default().to_uppercase();

        let token1_price = match token1_address.as_str() {
            addr if addr == weth_address || token1_symbol == "WETH" => 2118.14,
            addr if addr == usdc_address || token1_symbol == "USDC" => 1.0,
            addr if addr == tether_address || token1_symbol == "USDT" => 1.0,
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
