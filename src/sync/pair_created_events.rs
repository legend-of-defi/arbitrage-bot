use alloy::{
    primitives::{Address, Bytes, U256},
    providers::{Provider, MULTICALL3_ADDRESS},
    rpc::types::{BlockNumberOrTag, Filter},
    sol,
    sol_types::{SolCall, SolEvent, SolValue},
};
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;
use eyre::Result;
use futures::StreamExt;
use log::{error, info};

use crate::models::token::NewToken;
use crate::schemas::tokens::{self};
use crate::{schemas::pairs, utils::app_context::AppContext};

// Event emitted by UniswapV2Factory when a new trading pair is created.
// Contains information about the two tokens in the pair and the pair contract address.
sol! {
    event PairCreated(
        address indexed token0,  // First token in the pair
        address indexed token1,  // Second token in the pair
        address pair,           // Address of the newly created pair contract
        uint256                // Initial liquidity (not used in our case)
    );
}

sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IMulticall3.sol"
}

sol! {
    #[sol(rpc)]
    "contracts/src/interfaces/IERC20.sol"
}

/// Sync pair created events.
/// These are emitted by `UniswapV2Factory` contracts.
/// # Errors
/// Returns an error if the database connection fails
pub async fn pair_created_events(ctx: &AppContext) -> Result<()> {
    info!("sync::pair_created_events: Starting event sync...");

    let mut conn = ctx.db.get().await?;
    let provider = &ctx.base_provider;

    // Create a filter for PairCreated events starting from the latest block
    let filter = Filter::new()
        .event(PairCreated::SIGNATURE)
        .from_block(BlockNumberOrTag::Latest);

    // Subscribe to logs with retry logic
    let mut stream = loop {
        match provider.subscribe_logs(&filter).await {
            Ok(sub) => break sub.into_stream(),
            Err(e) => {
                error!("sync::events: Failed to subscribe to logs: {e}");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    };

    // Process events as they arrive
    while let Some(log) = stream.next().await {
        // Decode the event from the log
        let event = match PairCreated::decode_log(&log.inner, true) {
            Ok(event) => event,
            Err(e) => {
                error!("sync::events: Failed to decode event: {e}");
                continue;
            }
        };

        // Get or create token records for both tokens in the pair
        let token0_id = token_id_by_address(ctx, event.token0).await?;
        let token1_id = token_id_by_address(ctx, event.token1).await?;

        // Create a new pair record in the database
        diesel::insert_into(pairs::table)
            .values((
                pairs::address.eq(event.pair.to_string()),
                pairs::token0_id.eq(token0_id),
                pairs::token1_id.eq(token1_id),
                pairs::is_valid.eq(true), // New pairs are valid by default
            ))
            .execute(&mut conn)
            .await?;
    }

    Ok(())
}

/// Get the token ID for a given address, creating a new token record if it doesn't exist.
///
/// This function:
/// 1. Uses multicall to efficiently fetch token information (name, symbol, decimals)
/// 2. Validates the token by checking if all calls succeed
/// 3. Creates or updates the token record in the database
/// 4. Returns the token's database ID
///
/// # Arguments
/// * `ctx` - Application context containing database and provider
/// * `token_address` - Ethereum address of the token
///
/// # Returns
/// * `Result<i32>` - Database ID of the token
async fn token_id_by_address(ctx: &AppContext, token_address: Address) -> Result<i32> {
    let mut conn = ctx.db.get().await?;
    info!("token_id_by_address: {}", token_address);

    // Create multicall instance for batch calling
    let multicall = IMulticall3::new(MULTICALL3_ADDRESS, &ctx.base_provider);

    // Prepare calls to fetch token information
    let calls = vec![
        IMulticall3::Call3 {
            target: token_address,
            allowFailure: true,
            callData: Bytes::from(IERC20::nameCall::new(()).abi_encode()),
        },
        IMulticall3::Call3 {
            target: token_address,
            allowFailure: true,
            callData: Bytes::from(IERC20::symbolCall::new(()).abi_encode()),
        },
        IMulticall3::Call3 {
            target: token_address,
            allowFailure: true,
            callData: Bytes::from(IERC20::decimalsCall::new(()).abi_encode()),
        },
    ];

    // Execute all calls in a single transaction
    let results = multicall.aggregate3(calls).call().await?;

    // Process results with fallbacks for failed calls
    let name = if results.returnData[0].success {
        String::abi_decode(&results.returnData[0].returnData, true).unwrap_or_default()
    } else {
        String::new()
    };

    let symbol = if results.returnData[1].success {
        String::abi_decode(&results.returnData[1].returnData, true).unwrap_or_default()
    } else {
        String::new()
    };

    let decimals = if results.returnData[2].success {
        i32::from(
            U256::abi_decode(&results.returnData[2].returnData, true)
                .unwrap_or_default()
                .to::<u8>(),
        )
    } else {
        0
    };

    // Token is considered valid only if all calls succeed
    let is_valid = results.returnData.iter().all(|r| r.success);

    // Create new token record with fetched information
    let new_token = NewToken::new(
        token_address,
        Some(symbol),
        Some(name),
        decimals,
        is_valid,
        None,
        None,
    );

    // Insert or update token record in database
    let id = diesel::insert_into(tokens::table)
        .values((
            tokens::address.eq(token_address.to_string()),
            tokens::name.eq(new_token.name()),
            tokens::symbol.eq(new_token.symbol()),
            tokens::decimals.eq(new_token.decimals()),
            tokens::is_valid.eq(new_token.is_valid()),
        ))
        .on_conflict(tokens::address)
        .do_update()
        .set((
            tokens::name.eq(new_token.name()),
            tokens::symbol.eq(new_token.symbol()),
            tokens::decimals.eq(new_token.decimals()),
            tokens::is_valid.eq(new_token.is_valid()),
        ))
        .returning(tokens::id)
        .get_result::<i32>(&mut conn)
        .await?;
    Ok(id)
}
