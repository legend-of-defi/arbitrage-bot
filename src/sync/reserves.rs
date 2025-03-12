use crate::bootstrap::fetch_reserves_by_range;
use crate::models::pair::Pair;
use crate::schemas::pairs;
use crate::utils::app_context::AppContext;
use alloy::primitives::Address;
use bigdecimal::BigDecimal;
use diesel::dsl::sql;
use diesel::sql_types::{Nullable, Numeric};
use diesel::BoolExpressionMethods;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::SelectableHelper;
use diesel_async::RunQueryDsl;
use eyre::Result;
use std::str::FromStr;

/// Update pairs with missing reserves.
/// This runs as a worker thread to continuously update pairs.
///
/// # Arguments
/// * `ctx` - Application context
/// * `batch_size` - Number of pairs to process in each batch
///
/// # Returns
/// Result indicating success or failure
///
/// # Errors
/// * If contract calls fail
/// * If database operations fail
pub async fn reserves(ctx: &AppContext) -> Result<()> {
    loop {
        let pairs_updated = sync(ctx, 50).await?;

        if pairs_updated == 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

/// Sync pairs with missing reserves
///
/// This function queries the database for pairs that don't have reserves,
/// and then fetches the reserves for those pairs.
///
/// # Errors
/// Returns an error if the database connection fails
///
/// # Returns
/// Returns the number of pairs synced
async fn sync(ctx: &AppContext, batch_size: i16) -> Result<usize> {
    let mut conn = ctx.db.get().await?;

    // Query for pairs with missing reserves using Diesel
    let pairs_missing_reserves: Vec<Pair> = pairs::table
        .filter(pairs::reserve0.is_null().or(pairs::reserve1.is_null()))
        .select(Pair::as_select())
        .limit(i64::from(batch_size))
        .load::<Pair>(&mut conn)
        .await?;

    log::info!(
        "sync::reserves: Found {} pairs with missing reserves",
        pairs_missing_reserves.len()
    );

    if pairs_missing_reserves.is_empty() {
        log::info!("sync::reserves: No pairs with missing reserves, nothing to do");
        return Ok(0);
    }

    // Get addresses of pairs with missing reserves
    let pair_addresses: Vec<Address> = pairs_missing_reserves
        .iter()
        .map(crate::models::pair::Pair::address)
        .collect();

    log::info!(
        "sync::reserves: First 5 pair addresses: {:?}",
        pair_addresses.iter().take(5).collect::<Vec<_>>()
    );

    // Fetch reserves for these pairs
    log::info!(
        "sync::reserves: Fetching reserves for {} pairs...",
        pair_addresses.len()
    );
    let reserves = match fetch_reserves_by_range(ctx, pair_addresses.clone()).await {
        Ok(reserves) => {
            log::info!(
                "sync::reserves: Successfully fetched reserves for {} pairs",
                reserves.len()
            );
            reserves
        }
        Err(e) => {
            log::error!("sync::reserves: Error fetching reserves: {e}");
            return Err(e);
        }
    };

    // Update pairs with reserves
    let mut updated_count = 0;
    for (index, pair) in pairs_missing_reserves.iter().enumerate() {
        if index >= reserves.len() {
            log::error!(
                "sync::reserves: Reserves index out of bounds: {}/{}",
                index,
                reserves.len()
            );
            break;
        }

        let reserve = &reserves[index];
        let reserve0_val = BigDecimal::from_str(&reserve.reserve0.to_string())
            .unwrap_or_else(|_| BigDecimal::from(0));
        let reserve1_val = BigDecimal::from_str(&reserve.reserve1.to_string())
            .unwrap_or_else(|_| BigDecimal::from(0));

        // Update pair in database using Diesel
        diesel::update(pairs::table.find(pair.id()))
            .set((
                pairs::reserve0.eq(sql::<Nullable<Numeric>>(&reserve0_val.to_string())),
                pairs::reserve1.eq(sql::<Nullable<Numeric>>(&reserve1_val.to_string())),
            ))
            .execute(&mut conn)
            .await?;

        log::info!(
            "sync::reserves: Updated pair {} with reserve0: {}, reserve1: {}",
            pair.address(),
            reserve0_val,
            reserve1_val,
        );
        updated_count += 1;
    }

    log::info!(
        "sync::reserves: Updated {} pairs with reserves",
        updated_count
    );
    Ok(updated_count)
}
