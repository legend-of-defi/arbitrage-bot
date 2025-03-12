use alloy::primitives::{Address, Bytes, U256};
use eyre::Result;
use log::{debug, info, warn};

use crate::models::pair::Pair;
use crate::schemas::{pairs, tokens};
use crate::utils::app_context::AppContext;
use diesel::QueryDsl;
use diesel::SelectableHelper;
use diesel::{BoolExpressionMethods, ExpressionMethods};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use alloy::providers::MULTICALL3_ADDRESS;
use alloy::sol;
use alloy::sol_types::{SolCall, SolValue};

// Multicall3 interface for batch calling multiple contract functions in a single transaction.
// Used to efficiently fetch token information (name, symbol, decimals) in one RPC call.
sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IMulticall3.sol"
}

// ERC20 token interface for fetching basic token information.
// Used to get token name, symbol, and decimals.
sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IERC20.sol"
}

// UniswapV2Pair interface for fetching pair-specific information.
// Used to get token addresses and other pair-related data.
sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IUniswapV2Pair.sol"
}

/// Sync pairs tokens
/// Reads pairs from the database that don't have tokens, reads pair's contract and fetches
/// token info
/// # Errors
/// Returns an error if the database connection fails
///
pub async fn pair_tokens(ctx: &AppContext) -> Result<()> {
    info!("sync::pair_tokens: Starting token sync...");

    loop {
        let synced_tokens_count = sync(ctx, 100).await?;

        if synced_tokens_count == 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

/// Sync a batch of pairs' tokens.
///
/// This function:
/// 1. Queries the database for pairs missing token information
/// 2. For each pair:
///    - Fetches token addresses using multicall
///    - Fetches token information for both tokens
///    - Updates database records
///
/// # Arguments
/// * `ctx` - Application context containing database and provider
/// * `limit` - Maximum number of pairs to process in this batch
///
/// # Returns
/// * `Result<usize>` - Number of pairs processed in this batch
///
/// TODO: refactor this function to be shorter
#[allow(clippy::too_many_lines)]
async fn sync(ctx: &AppContext, limit: i64) -> Result<usize> {
    let mut conn = ctx.db.get().await?;

    // Query for pairs missing token info
    let pairs: Vec<Pair> = pairs::table
        .filter(pairs::token0_id.is_null().or(pairs::token1_id.is_null()))
        .select(Pair::as_select())
        .limit(limit)
        .load::<Pair>(&mut conn)
        .await?;

    info!(
        "sync::pair_tokens(): Found {} pairs missing tokens info",
        pairs.len()
    );

    // Create multicall instance for batch calling
    let multicall = IMulticall3::new(MULTICALL3_ADDRESS, &ctx.base_provider);

    // Prepare all calls in a single batch
    let mut all_calls = Vec::new();
    let mut pair_indices = Vec::new(); // Track which calls belong to which pair

    for (pair_idx, pair) in pairs.iter().enumerate() {
        // Add calls for token addresses
        all_calls.extend([
            IMulticall3::Call3 {
                target: pair.address(),
                allowFailure: true,
                callData: Bytes::from(IUniswapV2Pair::token0Call::new(()).abi_encode()),
            },
            IMulticall3::Call3 {
                target: pair.address(),
                allowFailure: true,
                callData: Bytes::from(IUniswapV2Pair::token1Call::new(()).abi_encode()),
            },
        ]);
        pair_indices.extend([pair_idx, pair_idx]); // Each pair has 2 address calls
    }

    // Execute all address calls in a single multicall
    let address_results = match multicall.aggregate3(all_calls).call().await {
        Ok(results) => results.returnData,
        Err(e) => {
            warn!("sync::pair_tokens: Failed to fetch token addresses: {e}");
            return Ok(0);
        }
    };

    // Process results and prepare token info calls
    let mut token_calls = Vec::new();
    let mut token_info = Vec::new(); // Store token addresses and their metadata

    for (i, result) in address_results.iter().enumerate() {
        let pair_idx = pair_indices[i];
        let pair = &pairs[pair_idx];
        let is_token0 = i % 2 == 0;

        if !result.success {
            warn!(
                "sync::pair_tokens: Failed to get {} for pair {}",
                if is_token0 { "token0" } else { "token1" },
                pair.address()
            );
            continue;
        }

        let token_addr = match Address::abi_decode(&result.returnData, true) {
            Ok(addr) => addr,
            Err(e) => {
                warn!(
                    "sync::pair_tokens: Failed to decode {} address for pair {}: {e}",
                    if is_token0 { "token0" } else { "token1" },
                    pair.address()
                );
                continue;
            }
        };

        if token_addr == Address::ZERO {
            continue;
        }

        // Add calls for token info
        token_calls.extend([
            IMulticall3::Call3 {
                target: token_addr,
                allowFailure: true,
                callData: Bytes::from(IERC20::nameCall::new(()).abi_encode()),
            },
            IMulticall3::Call3 {
                target: token_addr,
                allowFailure: true,
                callData: Bytes::from(IERC20::symbolCall::new(()).abi_encode()),
            },
            IMulticall3::Call3 {
                target: token_addr,
                allowFailure: true,
                callData: Bytes::from(IERC20::decimalsCall::new(()).abi_encode()),
            },
        ]);

        token_info.push((pair_idx, is_token0, token_addr));
    }

    // Execute all token info calls in a single multicall
    let token_results = match multicall.aggregate3(token_calls).call().await {
        Ok(results) => results.returnData,
        Err(e) => {
            warn!("sync::pair_tokens: Failed to fetch token info: {e}");
            return Ok(0);
        }
    };

    // Process token results and update database
    for (i, (pair_idx, is_token0, token_addr)) in token_info.iter().enumerate() {
        let pair = &pairs[*pair_idx];
        let offset = i * 3;
        let results = &token_results[offset..offset + 3];

        if let Err(e) = process_token(&mut conn, *token_addr, results, pair.id(), *is_token0).await
        {
            warn!(
                "sync::pair_tokens: Failed to process {} for pair {}: {e}",
                if *is_token0 { "token0" } else { "token1" },
                pair.address()
            );
        }
    }

    Ok(pairs.len())
}

/// Process token information and update the database.
///
/// This function:
/// 1. Decodes token information from multicall results
/// 2. Validates the token based on call success
/// 3. Creates or updates the token record in the database
/// 4. Updates the pair record with the token ID
///
/// # Arguments
/// * `conn` - Database connection
/// * `token_addr` - Token contract address
/// * `results` - Multicall results for name, symbol, and decimals
/// * `pair_id` - ID of the pair to update
/// * `is_token0` - Whether this is token0 or token1
///
/// # Returns
/// * `Result<()>` - Success or failure of the operation
async fn process_token(
    conn: &mut AsyncPgConnection,
    token_addr: Address,
    results: &[IMulticall3::Result],
    pair_id: i32,
    is_token0: bool,
) -> Result<()> {
    // Decode token information from results
    let name = if results[0].success {
        String::abi_decode(&results[0].returnData, true).unwrap_or_default()
    } else {
        String::new()
    };

    let symbol = if results[1].success {
        String::abi_decode(&results[1].returnData, true).unwrap_or_default()
    } else {
        String::new()
    };

    let decimals = if results[2].success {
        i32::from(
            U256::abi_decode(&results[2].returnData, true)
                .unwrap_or_default()
                .to::<u8>(),
        )
    } else {
        0
    };

    // Token is valid only if all calls succeed
    let is_valid = results.iter().all(|r| r.success);

    debug!(
        "sync::pair_tokens: Processing token {} ({}): name={}, symbol={}, decimals={}, is_valid={}",
        token_addr,
        if is_token0 { "token0" } else { "token1" },
        name,
        symbol,
        decimals,
        is_valid
    );

    // Upsert token and get its ID
    let token_id = diesel::insert_into(tokens::table)
        .values((
            tokens::address.eq(token_addr.to_string()),
            tokens::name.eq(&name),
            tokens::symbol.eq(&symbol),
            tokens::decimals.eq(decimals),
            tokens::is_valid.eq(is_valid),
        ))
        .on_conflict(tokens::address)
        .do_update()
        .set((
            tokens::name.eq(&name),
            tokens::symbol.eq(&symbol),
            tokens::decimals.eq(decimals),
            tokens::is_valid.eq(is_valid),
        ))
        .returning(tokens::id)
        .get_result::<i32>(conn)
        .await?;

    // Update pair with token ID and set is_valid based on token validity
    if is_token0 {
        diesel::update(pairs::table)
            .filter(pairs::id.eq(pair_id))
            .set((pairs::token0_id.eq(token_id), pairs::is_valid.eq(is_valid)))
            .execute(conn)
            .await?;
    } else {
        diesel::update(pairs::table)
            .filter(pairs::id.eq(pair_id))
            .set((pairs::token1_id.eq(token_id), pairs::is_valid.eq(is_valid)))
            .execute(conn)
            .await?;
    }

    Ok(())
}
