use crate::models::token::PriceSupportStatus;
use crate::utils::app_context::AppContext;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use eyre::Result;
use log;
use reqwest;
use serde::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;

const BATCH_SIZE: i64 = 100; // Number of tokens to process in each batch
const MORALIS_API_URL: &str = "https://deep-index.moralis.io/api/v2.2/erc20/prices";

#[derive(Debug, Serialize)]
struct TokenRequest {
    exchange: Option<String>,
    token_address: String,
}

#[derive(Debug, Serialize)]
struct PriceRequest {
    tokens: Vec<TokenRequest>,
}

#[derive(Debug, Deserialize)]
struct TokenPrice {
    #[serde(rename = "tokenAddress")]
    token_address: String,
    #[serde(rename = "usdPrice")]
    usd_price: f64,
}

/// Main function that continuously syncs token exchange rates
/// Fetches prices from Moralis API and updates the tokens table
pub async fn exchange_rates(ctx: &AppContext) -> Result<()> {
    log::info!("sync::exchange_rates: Starting exchange rates sync service");

    loop {
        log::info!("sync::exchange_rates: Starting sync iteration");
        match sync(ctx, BATCH_SIZE).await {
            Ok(count) => {
                log::info!(
                    "sync::exchange_rates: Completed sync iteration. Updated exchange rates for {} tokens",
                    count
                );
            }
            Err(e) => {
                log::error!("sync::exchange_rates: Error syncing exchange rates: {}", e);
            }
        }

        log::info!("sync::exchange_rates: Sleeping before next sync iteration");
        // Sleep before the next sync
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

/// Sync exchange rates for a batch of tokens
/// Updates tokens that:
/// 1. Don't have a price_support_status value, OR
/// 2. Have a last_updated timestamp that's more than 24 hours old
async fn sync(ctx: &AppContext, limit: i64) -> Result<usize> {
    let mut conn = ctx.db.get().await?;

    // Calculate the timestamp for 24 hours ago
    let one_day_ago = Utc::now().naive_utc() - Duration::days(1);
    log::info!(
        "sync::exchange_rates: One day ago timestamp: {}",
        one_day_ago
    );

    #[derive(QueryableByName, Debug)]
    struct TokenToUpdate {
        #[diesel(sql_type = diesel::sql_types::Text)]
        address: String,
    }

    let sql_query = "SELECT id, address FROM tokens
                 WHERE price_support_status IS NULL
                 OR (updated_last IS NOT NULL AND updated_last < $1)
                 LIMIT $2";

    let tokens: Vec<TokenToUpdate> = diesel::sql_query(sql_query)
        .bind::<diesel::sql_types::Timestamp, _>(one_day_ago)
        .bind::<diesel::sql_types::BigInt, _>(limit)
        .load(&mut conn)
        .await?;

    log::info!(
        "sync::exchange_rates: Found {} tokens that need updates",
        tokens.len()
    );

    if tokens.is_empty() {
        return Ok(0);
    }

    // Get API key and chain ID from environment
    let api_key = match env::var("MORALIS_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            log::error!("sync::exchange_rates: MORALIS_API_KEY not found in environment variables");
            return Err(eyre::eyre!(
                "MORALIS_API_KEY not found in environment variables"
            ));
        }
    };

    let chain_id = match env::var("MORALIS_API_BASE_CHAIN_ID") {
        Ok(id) => id,
        Err(_) => {
            log::error!("sync::exchange_rates: MORALIS_API_BASE_CHAIN_ID not found in environment variables");
            return Err(eyre::eyre!(
                "MORALIS_API_BASE_CHAIN_ID not found in environment variables"
            ));
        }
    };

    // Create HTTP client
    let reqwest_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Prepare token request objects
    let token_addresses: Vec<String> = tokens
        .iter()
        .map(|token| token.address.to_lowercase())
        .collect();

    let token_requests: Vec<TokenRequest> = token_addresses
        .iter()
        .map(|address| {
            log::debug!("sync::exchange_rates: Adding token to request: {}", address);
            TokenRequest {
                exchange: Some("uniswapv2".to_string()),
                token_address: address.clone(),
            }
        })
        .collect();

    log::info!(
        "sync::exchange_rates: Created {} token requests",
        token_requests.len()
    );

    // Create the request payload
    let request_payload = PriceRequest {
        tokens: token_requests,
    };

    // Make request to Moralis API
    log::info!(
        "sync::exchange_rates: Sending request to Moralis API (URL: {})",
        MORALIS_API_URL
    );
    let response_future = reqwest_client
        .post(MORALIS_API_URL)
        .header("accept", "application/json")
        .header("X-API-Key", api_key)
        .header("content-type", "application/json")
        .query(&[("chain", chain_id)])
        .json(&request_payload)
        .send();

    // Add a timeout to the request
    let response = match tokio::time::timeout(std::time::Duration::from_secs(30), response_future)
        .await
    {
        Ok(response_result) => match response_result {
            Ok(response) => response,
            Err(e) => {
                log::error!(
                    "sync::exchange_rates: Failed to send request to Moralis API: {}",
                    e
                );
                return Err(eyre::eyre!("Failed to send request to Moralis API: {}", e));
            }
        },
        Err(_) => {
            log::error!("sync::exchange_rates: Request to Moralis API timed out after 30 seconds");
            return Err(eyre::eyre!(
                "Request to Moralis API timed out after 30 seconds"
            ));
        }
    };

    // Store the status code before consuming the response
    let status = response.status();

    if !status.is_success() {
        let error_text = response.text().await?;
        log::error!(
            "sync::exchange_rates: Failed to fetch prices from Moralis API: {} - {}",
            status,
            error_text
        );
        return Err(eyre::eyre!("Moralis API error: {}", error_text));
    }

    // Parse response
    log::info!("sync::exchange_rates: Parsing response body");
    let response_text = response.text().await?;
    log::debug!("sync::exchange_rates: Response body: {}", response_text);

    let prices: Vec<TokenPrice> = match serde_json::from_str::<Vec<TokenPrice>>(&response_text) {
        Ok(parsed) => {
            log::info!(
                "sync::exchange_rates: Successfully parsed response into {} token prices",
                parsed.len()
            );
            parsed
        }
        Err(e) => {
            log::error!(
                "sync::exchange_rates: Failed to parse Moralis API response: {} - Response: {}",
                e,
                response_text
            );
            return Err(eyre::eyre!("Failed to parse Moralis API response: {}", e));
        }
    };

    // Track which tokens received prices
    let returned_addresses: Vec<String> = prices
        .iter()
        .map(|price| price.token_address.to_lowercase())
        .collect();

    // Find tokens without price data
    let missing_prices: Vec<&String> = token_addresses
        .iter()
        .filter(|address| !returned_addresses.contains(&address.to_lowercase()))
        .collect();

    let mut updated_count = 0;

    // For tokens without price data - mark them as UNSUPPORTED with NULL exchange_rate
    if !missing_prices.is_empty() {
        log::warn!(
            "sync::exchange_rates: Missing prices for {} tokens:",
            missing_prices.len()
        );
        for (i, address) in missing_prices.iter().enumerate() {
            match i.cmp(&20) {
                std::cmp::Ordering::Less => {
                    log::warn!("  - {}", address);
                }
                std::cmp::Ordering::Equal => {
                    log::warn!("  ... and {} more", missing_prices.len() - 20);
                    break;
                }
                std::cmp::Ordering::Greater => {
                    // We've already logged the message about more tokens at i==20
                    break;
                }
            }
        }

        // Update tokens without price data as UNSUPPORTED
        log::info!(
            "sync::exchange_rates: Marking {} tokens as UNSUPPORTED",
            missing_prices.len()
        );

        for addr in missing_prices {
            let now_timestamp = Utc::now().naive_utc();

            // Use SQL query with proper parameter binding
            let update_query = "UPDATE tokens SET
                               updated_last = $1,
                               price_support_status = $2
                               WHERE LOWER(address) = LOWER($3)";

            let updated = diesel::sql_query(update_query)
                .bind::<diesel::sql_types::Timestamp, _>(now_timestamp)
                .bind::<crate::schemas::sql_types::PriceSupportStatus, _>(
                    PriceSupportStatus::Unsupported,
                )
                .bind::<diesel::sql_types::Text, _>(addr)
                .execute(&mut conn)
                .await?;

            if updated > 0 {
                log::info!("Marked token as UNSUPPORTED: {}", addr);
                updated_count += 1;
            }
        }
    }

    // Update each token with its exchange rate
    log::info!("sync::exchange_rates: Updating tokens in database with price data");
    for price in prices {
        // Convert token address to lowercase for consistency
        let token_address = price.token_address.to_lowercase();
        log::debug!(
            "Processing token: {} with price: {}",
            token_address,
            price.usd_price
        );

        let now_timestamp = Utc::now().naive_utc();

        // Convert price to BigDecimal
        let price_decimal = match BigDecimal::from_str(&price.usd_price.to_string()) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to convert price to BigDecimal: {}", e);
                continue;
            }
        };

        // Use SQL query with proper parameter binding
        let update_query = "UPDATE tokens SET
                           exchange_rate = $1,
                           updated_last = $2,
                           price_support_status = $3
                           WHERE LOWER(address) = LOWER($4)";

        let updated = diesel::sql_query(update_query)
            .bind::<diesel::sql_types::Numeric, _>(price_decimal)
            .bind::<diesel::sql_types::Timestamp, _>(now_timestamp)
            .bind::<crate::schemas::sql_types::PriceSupportStatus, _>(PriceSupportStatus::Supported)
            .bind::<diesel::sql_types::Text, _>(token_address.clone())
            .execute(&mut conn)
            .await?;

        if updated > 0 {
            log::info!(
                "sync::exchange_rates: Updated exchange rate for token {}: ${}",
                token_address,
                price.usd_price
            );
            updated_count += 1;
        } else {
            log::warn!(
                "sync::exchange_rates: Token not found or not updated: {}",
                token_address
            );
        }
    }

    log::info!(
        "sync::exchange_rates: Completed sync with {} tokens updated",
        updated_count
    );
    Ok(updated_count)
}
