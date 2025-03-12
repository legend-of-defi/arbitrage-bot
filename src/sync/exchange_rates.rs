use crate::schemas::{pairs, tokens};
use crate::utils::app_context::AppContext;
use bigdecimal::{BigDecimal, RoundingMode};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use eyre::Result;
use log;
use std::collections::HashSet;
use std::str::FromStr;

/// Type alias for the actual connection type used in the project
type DbConn = diesel_async::AsyncPgConnection;

/// Number of tokens to process in each batch
const BATCH_SIZE: i64 = 100;
/// ID of WETH token in our database
const WETH_TOKEN_ID: i32 = 3;
/// Hardcoded WETH price in USD for now
const WETH_USD_PRICE: f64 = 2100.0;
/// Maximum decimal places to allow for tokens
const MAX_DECIMALS: i32 = 30;

/// Synchronizes exchange rates between tokens.
///
/// This function is the main entry point for the exchange rates sync service.
/// It processes token pairs to calculate and update exchange rates in the database.
///
/// # Errors
///
/// Returns an error if database operations fail, if there are issues with rate calculations,
/// or if the sync process encounters any other problem.
pub async fn exchange_rates(ctx: &AppContext) -> Result<()> {
    log::info!("sync::exchange_rates: Starting exchange rates sync service");

    loop {
        let updated_count = sync(ctx, BATCH_SIZE).await?;
        log::info!(
            "sync::exchange_rates: Completed sync iteration. Updated exchange rates for {} tokens",
            updated_count
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

/// Inner sync function that does the actual work
async fn sync(ctx: &AppContext, _limit: i64) -> Result<usize> {
    let mut conn = ctx.db.get().await?;
    let now_timestamp = Utc::now().naive_utc();

    // Update WETH price
    update_weth_price(&mut conn, now_timestamp).await?;

    // Load all tokens with known exchange rates
    let mut known_tokens = load_known_tokens(&mut conn).await?;

    // Track processed tokens and update counts
    let mut processed_token_ids = HashSet::new();
    processed_token_ids.insert(WETH_TOKEN_ID); // Consider WETH already processed

    let mut updated_count = 0;
    let mut iteration = 0;
    let mut new_tokens_discovered = true;

    // Process tokens in waves until no new exchange rates can be discovered
    while new_tokens_discovered {
        iteration += 1;
        new_tokens_discovered = false;

        // Find pairs where exactly one token has a known exchange rate
        let candidate_pairs = find_candidate_pairs(&mut conn, &known_tokens).await?;

        if candidate_pairs.is_empty() {
            log::info!("sync::exchange_rates: No more pairs to process. Updated 0 tokens.");
            break;
        }

        let mut iteration_updated_count = 0;

        // Process each candidate pair
        for (pair_id, token0_id, token1_id, reserve0, reserve1) in candidate_pairs {
            // Skip if both tokens already have known exchange rates
            if both_tokens_known(token0_id, token1_id, &known_tokens) {
                continue;
            }

            // Determine which token already has a price and which needs a price
            let (known_token_id, unknown_token_id) =
                identify_token_roles(token0_id, token1_id, &known_tokens);

            // Skip if we can't determine both tokens
            let (Some(known_token_id), Some(unknown_token_id)) = (known_token_id, unknown_token_id)
            else {
                log::debug!(
                    "sync::exchange_rates: Pair {} - Skipping due to None token ID",
                    pair_id
                );
                continue;
            };

            // Skip if we've already processed this token in this sync
            if processed_token_ids.contains(&unknown_token_id) {
                continue;
            }

            // Mark token as processed to avoid checking it again
            processed_token_ids.insert(unknown_token_id);

            // Calculate and update the exchange rate
            if let Some(token_price) = calculate_exchange_rate(
                &mut conn,
                known_token_id,
                unknown_token_id,
                token0_id,
                reserve0,
                reserve1,
            )
            .await?
            {
                // Update the token's exchange rate in the database
                if update_token_exchange_rate(
                    &mut conn,
                    unknown_token_id,
                    &token_price,
                    now_timestamp,
                )
                .await?
                {
                    // Get token address for logging
                    let token_address = get_token_address(&mut conn, unknown_token_id).await?;

                    log::info!(
                        "sync::exchange_rates: Updated exchange rate for token {} (ID: {}) based on {} (ID: {}): ${}",
                        token_address,
                        unknown_token_id,
                        if known_token_id == WETH_TOKEN_ID { "WETH".to_string() } else { format!("token {known_token_id}") },
                        known_token_id,
                        token_price
                    );

                    updated_count += 1;
                    iteration_updated_count += 1;

                    // Add this token to known tokens for future iterations
                    known_tokens.insert(unknown_token_id);
                    new_tokens_discovered = true;
                }
            }
        }

        log::info!(
            "sync::exchange_rates: Iteration {} - Updated {} tokens",
            iteration,
            iteration_updated_count
        );

        // Stop if no new tokens were updated in this iteration
        if iteration_updated_count == 0 {
            log::info!("sync::exchange_rates: No tokens updated in this iteration. Stopping.");
            break;
        }
    }

    Ok(updated_count)
}

/// Update the WETH token's price
async fn update_weth_price(conn: &mut DbConn, timestamp: chrono::NaiveDateTime) -> Result<()> {
    let weth_price = BigDecimal::from_str(&WETH_USD_PRICE.to_string())?;

    diesel::update(tokens::table.filter(tokens::id.eq(WETH_TOKEN_ID)))
        .set((
            tokens::exchange_rate.eq(&weth_price),
            tokens::updated_last.eq(timestamp),
        ))
        .execute(conn)
        .await?;

    log::info!(
        "sync::exchange_rates: Updated WETH price to ${}",
        WETH_USD_PRICE
    );

    Ok(())
}

/// Load all tokens that already have exchange rates
async fn load_known_tokens(conn: &mut DbConn) -> Result<HashSet<i32>> {
    let mut known_tokens = HashSet::new();

    let tokens_with_rates = tokens::table
        .filter(tokens::exchange_rate.is_not_null())
        .filter(tokens::decimals.is_not_null())
        .select(tokens::id)
        .load::<i32>(conn)
        .await?;

    for token_id in tokens_with_rates {
        known_tokens.insert(token_id);
    }

    log::debug!(
        "sync::exchange_rates: Starting with {} tokens that already have exchange rates",
        known_tokens.len()
    );

    Ok(known_tokens)
}

/// Find pairs where at least one token has a known exchange rate
async fn find_candidate_pairs(
    conn: &mut DbConn,
    known_tokens: &HashSet<i32>,
) -> Result<
    Vec<(
        i32,
        Option<i32>,
        Option<i32>,
        Option<BigDecimal>,
        Option<BigDecimal>,
    )>,
> {
    let pairs = pairs::table
        .filter(
            pairs::token0_id
                .eq_any(known_tokens)
                .or(pairs::token1_id.eq_any(known_tokens)),
        )
        .filter(pairs::reserve0.gt(BigDecimal::from(0)))
        .filter(pairs::reserve1.gt(BigDecimal::from(0)))
        .select((
            pairs::id,
            pairs::token0_id,
            pairs::token1_id,
            pairs::reserve0,
            pairs::reserve1,
        ))
        .load::<(
            i32,
            Option<i32>,
            Option<i32>,
            Option<BigDecimal>,
            Option<BigDecimal>,
        )>(conn)
        .await?;

    log::debug!(
        "sync::exchange_rates: Found {} potential pairs to process",
        pairs.len()
    );

    Ok(pairs)
}

/// Check if both tokens in a pair already have known exchange rates
fn both_tokens_known(
    token0_id: Option<i32>,
    token1_id: Option<i32>,
    known_tokens: &HashSet<i32>,
) -> bool {
    token0_id.is_some_and(|id| known_tokens.contains(&id))
        && token1_id.is_some_and(|id| known_tokens.contains(&id))
}

/// Identify which token has a known rate and which needs a rate
fn identify_token_roles(
    token0_id: Option<i32>,
    token1_id: Option<i32>,
    known_tokens: &HashSet<i32>,
) -> (Option<i32>, Option<i32>) {
    if token0_id.is_some_and(|id| known_tokens.contains(&id)) {
        (token0_id, token1_id)
    } else {
        (token1_id, token0_id)
    }
}

/// Calculate a token's exchange rate based on a known token's price
async fn calculate_exchange_rate(
    conn: &mut DbConn,
    known_token_id: i32,
    unknown_token_id: i32,
    token0_id: Option<i32>,
    reserve0: Option<BigDecimal>,
    reserve1: Option<BigDecimal>,
) -> Result<Option<BigDecimal>> {
    // Get information about the unknown token
    let Ok(Some(unknown_token_info)) = tokens::table
        .filter(tokens::id.eq(unknown_token_id))
        .select(tokens::decimals)
        .first::<Option<i32>>(conn)
        .await
    else {
        log::debug!(
            "sync::exchange_rates: Token {} has no decimals, skipping",
            unknown_token_id
        );
        return Ok(None);
    };

    // Get exchange rate and decimals of the known token
    let Ok((Some(known_token_exchange_rate), Some(known_token_decimals))) = tokens::table
        .filter(tokens::id.eq(known_token_id))
        .select((tokens::exchange_rate, tokens::decimals))
        .first::<(Option<BigDecimal>, Option<i32>)>(conn)
        .await
    else {
        log::debug!(
            "sync::exchange_rates: Known token {} missing exchange rate or decimals",
            known_token_id
        );
        return Ok(None);
    };

    // Skip tokens with extremely large decimals to prevent numeric overflow
    if unknown_token_info > MAX_DECIMALS || known_token_decimals > MAX_DECIMALS {
        log::warn!(
            "sync::exchange_rates: Token {} or known token {} has too many decimals ({} or {}), skipping to prevent overflow",
            unknown_token_id,
            known_token_id,
            unknown_token_info,
            known_token_decimals
        );
        return Ok(None);
    }

    // Unwrap reserves which should be non-null due to our filters
    let reserve0 = reserve0.unwrap_or_else(|| BigDecimal::from(0));
    let reserve1 = reserve1.unwrap_or_else(|| BigDecimal::from(0));

    // Determine known token and unknown token reserves
    let (known_reserve, unknown_reserve) = if token0_id == Some(known_token_id) {
        (reserve0, reserve1)
    } else {
        (reserve1, reserve0)
    };

    // Calculate decimal bases the safe way
    let known_decimal_base = calculate_decimal_base(known_token_decimals);
    let unknown_decimal_base = calculate_decimal_base(unknown_token_info);

    // Normalize reserves by dividing by their decimal bases
    let known_reserve_normalized = known_reserve / &known_decimal_base;
    let unknown_reserve_normalized = unknown_reserve / &unknown_decimal_base;

    // Calculate the token price (Formula 2)
    let price = if unknown_reserve_normalized == BigDecimal::from(0) {
        return Ok(None); // Avoid division by zero
    } else {
        known_token_exchange_rate * (known_reserve_normalized / unknown_reserve_normalized)
    };

    // Round to 18 decimal places for consistency
    let price_rounded = price.with_scale_round(18, RoundingMode::HalfUp);

    Ok(Some(price_rounded))
}

/// Calculate 10^decimals safely using `BigDecimal`
fn calculate_decimal_base(decimals: i32) -> BigDecimal {
    match decimals {
        0 => BigDecimal::from(1),
        d => {
            let mut result = BigDecimal::from(1);
            for _ in 0..d {
                result *= BigDecimal::from(10);
            }
            result
        }
    }
}

/// Update a token's exchange rate in the database
async fn update_token_exchange_rate(
    conn: &mut DbConn,
    token_id: i32,
    price: &BigDecimal,
    timestamp: chrono::NaiveDateTime,
) -> Result<bool> {
    let updated = diesel::update(tokens::table.filter(tokens::id.eq(token_id)))
        .set((
            tokens::exchange_rate.eq(price),
            tokens::updated_last.eq(timestamp),
        ))
        .execute(conn)
        .await?;

    Ok(updated > 0)
}

/// Get a token's address for logging purposes
async fn get_token_address(conn: &mut DbConn, token_id: i32) -> Result<String> {
    let address = tokens::table
        .filter(tokens::id.eq(token_id))
        .select(tokens::address)
        .first::<String>(conn)
        .await?;

    Ok(address)
}
