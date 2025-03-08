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
use log::info;
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

async fn sync(ctx: &AppContext, batch_size: i16) -> Result<usize> {
    let mut conn = ctx.db.get().await?;

    // Query for pairs with missing reserves using Diesel
    let pairs_missing_reserves: Vec<Pair> = pairs::table
        .filter(pairs::reserve0.is_null().or(pairs::reserve1.is_null()))
        .select(Pair::as_select())
        .limit(i64::from(batch_size))
        .load::<Pair>(&mut conn)
        .await?;

    info!(
        "Fetching reserves by range range from {} to {:?}",
        batch_size, pairs_missing_reserves
    );

    // Get addresses of pairs with missing reserves
    let pair_addresses: Vec<Address> = pairs_missing_reserves
        .iter()
        .map(crate::models::pair::Pair::address)
        .collect();

    // Fetch reserves for these pairs
    let reserves = match fetch_reserves_by_range(ctx, pair_addresses.clone()).await {
        Ok(reserves) => reserves,
        Err(e) => {
            log::error!("sync::reserves: Error fetching reserves: {e}");
            return Err(e);
        }
    };

    // Update pairs with reserves
    for (index, pair) in pairs_missing_reserves.iter().enumerate() {
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

        log::debug!(
            "sync::reserves: Updated pair {} with reserve0: {}, reserve1: {}",
            pair.address(),
            reserve0_val,
            reserve1_val,
        );
    }

    Ok(pairs_missing_reserves.len())
}
