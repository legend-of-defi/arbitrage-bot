use crate::models::pair::Pair;
use crate::models::token::Token;
use crate::schemas::pairs;
use crate::schemas::tokens;
use crate::utils::app_context::AppContext;
use bigdecimal::BigDecimal;
use diesel::SelectableHelper;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use eyre::Result;
use log;
use std::collections::HashMap;
use std::str::FromStr;

// Hardcoded token addresses
/// WETH address
const WETH_ADDRESS: &str = "0x4200000000000000000000000000000000000006";
/// USDC address
const USDC_ADDRESS: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
/// USDT address
const USDT_ADDRESS: &str = "0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2";
/// DAI address
const DAI_ADDRESS: &str = "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb";

// Hardcoded token prices in USD
/// WETH price
const WETH_PRICE: f64 = 2211.90;
/// USDC price
const USDC_PRICE: f64 = 1.00;
/// USDT price
const USDT_PRICE: f64 = 1.00;
/// DAI price
const DAI_PRICE: f64 = 1.00;

/// Sync USD values for pairs
/// This function continuously looks for pairs with tokens and reserves but no USD value
/// and calculates the USD value based on token reserves and hardcoded prices
/// # Errors
/// Returns an error if the database connection fails
pub async fn usd(ctx: &AppContext) -> Result<()> {
    loop {
        let _updated_pairs_count = sync(ctx, 100).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

/// Sync a batch of pairs' USD values
async fn sync(ctx: &AppContext, limit: i64) -> Result<usize> {
    let mut conn = ctx.db.get().await?;
    let mut updated_count = 0;

    // Create token price map (without logging)
    let token_prices = get_token_price_map();

    // Query for pairs with tokens and reserves but missing USD values
    let pairs: Vec<Pair> = diesel::QueryDsl::filter(
        pairs::table,
        pairs::token0_id
            .is_not_null()
            .and(pairs::token1_id.is_not_null())
            .and(pairs::reserve0.is_not_null())
            .and(pairs::reserve1.is_not_null())
            .and(pairs::usd.is_null()),
    )
    .select(Pair::as_select())
    .limit(limit)
    .load::<Pair>(&mut conn)
    .await?;

    if pairs.is_empty() {
        return Ok(0);
    }

    // Get all required token IDs
    let token_ids: Vec<i32> = pairs
        .iter()
        .flat_map(|pair| [pair.token0_id, pair.token1_id])
        .flatten()
        .collect();

    // Fetch all tokens in one query
    let tokens: Vec<Token> = diesel::QueryDsl::filter(tokens::table, tokens::id.eq_any(&token_ids))
        .select(Token::as_select())
        .load::<Token>(&mut conn)
        .await?;

    // Create a map of token ID to Token for easy lookup
    let token_map: HashMap<i32, &Token> = tokens.iter().map(|token| (token.id(), token)).collect();

    // Process each pair
    for pair in &pairs {
        if let (Some(token0_id), Some(token1_id), Some(reserve0), Some(reserve1)) = (
            pair.token0_id,
            pair.token1_id,
            pair.reserve0.clone(),
            pair.reserve1.clone(),
        ) {
            // Get tokens
            let token0_ref = token_map.get(&token0_id);
            let token1_ref = token_map.get(&token1_id);

            if let (Some(token0_ref), Some(token1_ref)) = (token0_ref, token1_ref) {
                // Calculate USD value
                let usd_value = calculate_usd_value(
                    token0_ref,
                    token1_ref,
                    &reserve0,
                    &reserve1,
                    &token_prices,
                );

                // For special marker value (-1), log differently
                if usd_value < 0.0 {
                    diesel::update(pairs::table.find(pair.id()))
                        .set(pairs::usd.eq(-1))
                        .execute(&mut conn)
                        .await?;

                    log::info!(
                        "sync::usd: Updated pair {} with special value -1 (no price data)",
                        pair.address()
                    );
                    updated_count += 1;
                } else {
                    // Normal case - update with calculated value
                    // Pair reserve USD value can hardly be greater than ~$4 bln
                    #[allow(clippy::cast_possible_truncation)]
                    diesel::update(pairs::table.find(pair.id()))
                        .set(pairs::usd.eq(usd_value as i32))
                        .execute(&mut conn)
                        .await?;

                    log::info!(
                        "sync::usd: Updated pair {} with USD value: ${}",
                        pair.address(),
                        usd_value
                    );
                    updated_count += 1;
                }
            }
        }
    }

    Ok(updated_count)
}

/// Calculate USD value for a pair based on its tokens and reserves
fn calculate_usd_value(
    token0: &Token,
    token1: &Token,
    reserve0: &BigDecimal,
    reserve1: &BigDecimal,
    token_prices: &HashMap<String, f64>,
) -> f64 {
    // Convert addresses to lowercase for comparison
    let token0_address = token0.address().to_string().to_lowercase();
    let token1_address = token1.address().to_string().to_lowercase();

    let token0_price = token_prices.get(&token0_address);
    let token1_price = token_prices.get(&token1_address);

    // Convert reserves to f64 considering decimals
    // SAFETY: decimals are not null - we check for null in the query
    #[allow(clippy::unwrap_used)]
    let reserve0_adjusted = convert_reserve_to_float(reserve0, token0.decimals().unwrap());
    #[allow(clippy::unwrap_used)]
    let reserve1_adjusted = convert_reserve_to_float(reserve1, token1.decimals().unwrap());

    match (token0_price, token1_price) {
        // Both tokens have known prices
        (Some(price0), Some(price1)) => {
            let value0 = reserve0_adjusted * price0;
            let value1 = reserve1_adjusted * price1;
            value0 + value1
        }
        // Only token0 has a known price
        (Some(price0), None) => {
            let value0 = reserve0_adjusted * price0;
            value0 * 2.0 // Double the value as per requirements
        }
        // Only token1 has a known price
        (None, Some(price1)) => {
            let value1 = reserve1_adjusted * price1;
            value1 * 2.0 // Double the value as per requirements
        }
        // No known prices for either token - return -1 as a marker
        (None, None) => -1.0,
    }
}

/// Convert token reserve to float value considering decimals
fn convert_reserve_to_float(reserve: &BigDecimal, decimals: i32) -> f64 {
    let divisor = 10.0_f64.powi(decimals);
    let reserve_str = reserve.to_string();

    // Parse reserve string to f64 and divide by the appropriate power of 10
    match f64::from_str(&reserve_str) {
        Ok(reserve_float) => reserve_float / divisor,
        Err(_) => 0.0,
    }
}

/// Create a map of token address -> price for hardcoded tokens
fn get_token_price_map() -> HashMap<String, f64> {
    let mut map = HashMap::new();
    map.insert(WETH_ADDRESS.to_lowercase(), WETH_PRICE);
    map.insert(USDC_ADDRESS.to_lowercase(), USDC_PRICE);
    map.insert(USDT_ADDRESS.to_lowercase(), USDT_PRICE);
    map.insert(DAI_ADDRESS.to_lowercase(), DAI_PRICE);
    map
}
