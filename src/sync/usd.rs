use crate::models::pair::Pair;
use crate::models::token::Token;
use crate::schemas::pairs;
use crate::schemas::tokens;
use crate::utils::app_context::AppContext;
use bigdecimal::BigDecimal;
use chrono::Utc;
use diesel::SelectableHelper;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use eyre::Result;
use log;
use std::collections::HashMap;
use std::str::FromStr;

/// The number of pairs to process in each batch to balance memory usage and performance
const BATCH_SIZE: i64 = 750;

/// Sync USD values for pairs
/// This function calculates and updates USD values for ALL pairs
/// based on token reserves and exchange rates, then sleeps for 24 hours
/// # Errors
/// Returns an error if the database connection fails
pub async fn usd(ctx: &AppContext) -> Result<()> {
    loop {
        let start_time = Utc::now();
        let updated_pairs_count = sync(ctx).await?;
        let end_time = Utc::now();
        let duration = end_time.signed_duration_since(start_time);

        log::info!(
            "sync::usd: Completed updating {} pairs in {} minutes. Sleeping for 24 hours.",
            updated_pairs_count,
            duration.num_minutes()
        );

        // Sleep for 5 days
        tokio::time::sleep(tokio::time::Duration::from_secs(5 * 24 * 60 * 60)).await;
    }
}

/// Sync ALL pairs' USD values
async fn sync(ctx: &AppContext) -> Result<usize> {
    let mut total_updated_count = 0;
    let mut total_processed_count = 0;
    let mut offset = 0;

    // Count total pairs to process for progress reporting
    let total_pairs_count = count_total_pairs(ctx).await?;
    log::info!(
        "sync::usd: Found {} total pairs to process",
        total_pairs_count
    );

    // Process pairs in batches
    loop {
        let (batch_count, updated_count) = process_batch(ctx, offset, BATCH_SIZE).await?;

        // Update counters
        total_processed_count += batch_count;
        total_updated_count += updated_count;

        // Log progress
        if batch_count > 0 {
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let percentage =
                (total_processed_count as f64 / total_pairs_count as f64 * 100.0) as i32;

            log::info!(
                "sync::usd: Progress: {}/{} pairs processed ({}%), Updated {} pairs in this batch",
                total_processed_count,
                total_pairs_count,
                percentage,
                updated_count
            );
        }

        // If we got fewer pairs than the batch size, it means we've processed all pairs
        // (Note: this check comes AFTER processing the current batch, so partial batches ARE processed)
        if batch_count < BATCH_SIZE {
            break;
        }

        // Move to next batch
        offset += BATCH_SIZE;

        // Small pause between batches to avoid overwhelming the database
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    log::info!(
        "sync::usd: Completed - Updated {}/{} pairs total",
        total_updated_count,
        total_processed_count
    );

    Ok(total_updated_count)
}

/// Count total pairs that need to be processed
async fn count_total_pairs(ctx: &AppContext) -> Result<i64> {
    let mut conn = ctx.db.get().await?;

    let count: i64 = pairs::table
        .filter(
            pairs::token0_id
                .is_not_null()
                .and(pairs::token1_id.is_not_null())
                .and(pairs::reserve0.is_not_null())
                .and(pairs::reserve1.is_not_null()),
        )
        .count()
        .get_result(&mut conn)
        .await?;

    Ok(count)
}

/// Process a batch of pairs
async fn process_batch(ctx: &AppContext, offset: i64, limit: i64) -> Result<(i64, usize)> {
    let mut conn = ctx.db.get().await?;
    let mut updated_count = 0;

    // Query for this batch of pairs
    let pairs: Vec<Pair> = diesel::QueryDsl::filter(
        pairs::table,
        pairs::token0_id
            .is_not_null()
            .and(pairs::token1_id.is_not_null())
            .and(pairs::reserve0.is_not_null())
            .and(pairs::reserve1.is_not_null()),
    )
    .select(Pair::as_select())
    .offset(offset)
    .limit(limit)
    .load::<Pair>(&mut conn)
    .await?;

    #[allow(clippy::cast_possible_wrap)]
    let batch_count = pairs.len() as i64;

    if pairs.is_empty() {
        return Ok((0, 0));
    }

    // Get all required token IDs for this batch
    let token_ids: Vec<i32> = pairs
        .iter()
        .flat_map(|pair| [pair.token0_id, pair.token1_id])
        .flatten()
        .collect();

    // Fetch tokens with exchange rates and decimals for this batch
    let tokens: Vec<(Token, Option<BigDecimal>, Option<i32>)> =
        diesel::QueryDsl::filter(tokens::table, tokens::id.eq_any(&token_ids))
            .select((Token::as_select(), tokens::exchange_rate, tokens::decimals))
            .load::<(Token, Option<BigDecimal>, Option<i32>)>(&mut conn)
            .await?;

    // Create token lookup map
    let token_map: HashMap<i32, (Token, Option<BigDecimal>, Option<i32>)> = tokens
        .into_iter()
        .map(|(token, exchange_rate, decimals)| (token.id(), (token, exchange_rate, decimals)))
        .collect();

    // Prepare updates with necessary information for logging
    let mut updates = Vec::new();

    // Process each pair in this batch
    for pair in &pairs {
        if let (Some(token0_id), Some(token1_id), Some(reserve0), Some(reserve1)) = (
            pair.token0_id,
            pair.token1_id,
            pair.reserve0.clone(),
            pair.reserve1.clone(),
        ) {
            // Get token data
            let token0_data = token_map.get(&token0_id);
            let token1_data = token_map.get(&token1_id);

            if let (
                Some((_token0, exchange_rate0, decimals0)),
                Some((_token1, exchange_rate1, decimals1)),
            ) = (token0_data, token1_data)
            {
                // Calculate USD value if both tokens have exchange rates and decimals
                if let (
                    Some(exchange_rate0),
                    Some(exchange_rate1),
                    Some(decimals0),
                    Some(decimals1),
                ) = (exchange_rate0, exchange_rate1, decimals0, decimals1)
                {
                    // Convert exchange rates and calculate USD value
                    if let (Ok(rate0), Ok(rate1)) = (
                        f64::from_str(&exchange_rate0.to_string()),
                        f64::from_str(&exchange_rate1.to_string()),
                    ) {
                        let reserve0_normalized = convert_reserve_to_float(&reserve0, *decimals0);
                        let reserve1_normalized = convert_reserve_to_float(&reserve1, *decimals1);

                        let usd_value = rate0 * reserve0_normalized + rate1 * reserve1_normalized;

                        // Add to updates with pair address for logging
                        #[allow(clippy::cast_possible_truncation)]
                        updates.push((pair.id(), pair.address(), usd_value, usd_value as i32));
                        updated_count += 1;
                    }
                }
            }
        }
    }

    // Execute updates if any
    if !updates.is_empty() {
        // Process updates in smaller sub-batches to avoid too large transactions
        for sub_batch in updates.chunks(100) {
            for (pair_id, pair_address, usd_value_f64, usd_value_i32) in sub_batch {
                // Update the database
                diesel::update(pairs::table.find(pair_id))
                    .set(pairs::usd.eq(usd_value_i32))
                    .execute(&mut conn)
                    .await?;

                // Log each update with details including USD value
                log::info!(
                    "sync::usd: Updated pair {} combined reserves in USD: ${:.2}",
                    pair_address,
                    usd_value_f64
                );
            }
        }
    }

    Ok((batch_count, updated_count))
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
