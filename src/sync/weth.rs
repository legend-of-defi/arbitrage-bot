use crate::schemas::tokens;
use crate::utils::app_context::AppContext;
use bigdecimal::BigDecimal;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use eyre::Result;
use log;
use reqwest::header;
use serde::Deserialize;
use std::env;
use std::str::FromStr;
use std::time::Duration;

/// WETH token address on Base chain
const WETH_ADDRESS: &str = "0x4200000000000000000000000000000000000006";

/// Response structure for Moralis API token price endpoint
#[derive(Debug, Deserialize)]
struct MoralisTokenPriceResponse {
    /// USD price of the token
    #[serde(rename = "usdPrice")]
    usd_price: f64,
}

/// Synchronizes WETH price from Moralis API.
///
/// This function is the entry point for the WETH price sync microprocess.
/// It fetches the current price of WETH from Moralis API and updates the database.
///
/// # Errors
///
/// Returns an error if API calls fail, database operations fail, or environment variables are missing.
pub async fn weth(ctx: &AppContext) -> Result<()> {
    log::info!("sync::weth: Starting WETH price sync service");

    loop {
        let updated = sync_weth_price(ctx).await?;

        if updated {
            log::info!("sync::weth: Updated WETH price from Moralis API");
        } else {
            log::warn!("sync::weth: Failed to update WETH price");
        }

        // Sleep for 24 hours before next update
        log::info!("sync::weth: Sleeping for 24 hours before next update");
        tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
    }
}

/// Fetches WETH price from Moralis API and updates the database
///
/// # Errors
///
/// Returns an error if API calls fail, database operations fail, or environment variables are missing.
async fn sync_weth_price(ctx: &AppContext) -> Result<bool> {
    // Get API key from environment variable
    let api_key = match env::var("MORALIS_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            log::error!("sync::weth: MORALIS_API_KEY environment variable not set");
            return Ok(false);
        }
    };

    // Build request to Moralis API
    let client = reqwest::Client::new();
    let url = format!(
        "https://deep-index.moralis.io/api/v2.2/erc20/{}/price?chain=base",
        WETH_ADDRESS
    );

    let response = client
        .get(&url)
        .header(header::ACCEPT, "application/json")
        .header("X-API-Key", api_key)
        .send()
        .await?;

    // Process response
    if !response.status().is_success() {
        log::error!(
            "sync::weth: Moralis API request failed with status: {}",
            response.status()
        );
        return Ok(false);
    }

    let price_data: MoralisTokenPriceResponse = response.json().await?;
    log::info!("sync::weth: Got WETH price: ${}", price_data.usd_price);

    // Update database with new price
    let mut conn = ctx.db.get().await?;
    let now_timestamp = Utc::now().naive_utc();
    let weth_price = BigDecimal::from_str(&price_data.usd_price.to_string())?;

    // Update token table with new price
    let updated_rows = diesel::update(tokens::table.filter(tokens::address.eq(WETH_ADDRESS)))
        .set((
            tokens::exchange_rate.eq(&weth_price),
            tokens::updated_last.eq(now_timestamp),
        ))
        .execute(&mut conn)
        .await?;

    Ok(updated_rows > 0)
}
