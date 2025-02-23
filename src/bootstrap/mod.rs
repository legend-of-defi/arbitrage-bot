pub mod types;

use alloy::{
    primitives::{address, Address, U256},
    sol,
};
use std::ops::Add;
use std::str::FromStr;

use crate::bootstrap::types::{PairInfo, Reserves};
use crate::db_service::{DbManager, PairService};
use crate::models::factory::NewFactory;
use crate::models::token::NewToken;
use crate::utils::app_context::AppContext;
use crate::utils::providers::create_http_provider;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)] IUniswapV2BatchRequest,
    r#"[{"inputs":[{"internalType":"contract UniswapV2Factory","name":"_uniswapFactory","type":"address"}],"name":"allPairsLength","outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"contract UniswapV2Factory","name":"_uniswapFactory","type":"address"},{"internalType":"uint256","name":"_start","type":"uint256"},{"internalType":"uint256","name":"_stop","type":"uint256"}],"name":"getPairsByIndexRange","outputs":[{"components":[{"components":[{"internalType":"address","name":"tokenAddress","type":"address"},{"internalType":"string","name":"name","type":"string"},{"internalType":"string","name":"symbol","type":"string"},{"internalType":"uint8","name":"decimals","type":"uint8"}],"internalType":"struct UniswapQuery.Token","name":"token0","type":"tuple"},{"components":[{"internalType":"address","name":"tokenAddress","type":"address"},{"internalType":"string","name":"name","type":"string"},{"internalType":"string","name":"symbol","type":"string"},{"internalType":"uint8","name":"decimals","type":"uint8"}],"internalType":"struct UniswapQuery.Token","name":"token1","type":"tuple"},{"internalType":"address","name":"pairAddress","type":"address"}],"internalType":"struct UniswapQuery.PairInfo[]","name":"","type":"tuple[]"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"contract IUniswapV2Pair[]","name":"_pairs","type":"address[]"}],"name":"getReservesByPairs","outputs":[{"internalType":"uint256[3][]","name":"","type":"uint256[3][]"}],"stateMutability":"view","type":"function"}]"#
);

/// Retrieves pairs within a specified index range from a factory contract
///
/// # Arguments
/// * `factory` - The address of the factory contract
/// * `from` - Starting index
/// * `to` - Ending index
///
/// # Returns
/// A vector of `PairInfo` containing pair and token information
///
/// # Errors
/// * If HTTP provider creation fails
/// * If contract call fails
pub async fn read_pairs_v2_by_range(
    factory: Address,
    from: U256,
    to: U256,
) -> Result<Vec<PairInfo>, eyre::Report> {
    let app_context = AppContext::new().await.expect("app context");
    let provider = app_context.base_remote;

    let uniswap_v2_batch_request = IUniswapV2BatchRequest::new(
        address!("0x72D6545d3F45F20754F66a2B99fc1A4D75BFEf5c"),
        provider,
    );

    Ok(uniswap_v2_batch_request
        .getPairsByIndexRange(factory, from, to)
        .call()
        .await?
        ._0
        .into_iter()
        .map(PairInfo::from)
        .collect())
}

/// Retrieves all pairs from a factory contract in batches
///
/// # Arguments
/// * `factory` - The address of the factory contract
/// * `batch_size` - Number of pairs to fetch in each batch
///
/// # Returns
/// A vector of tuples containing Factory, Token0, Token1, and Pair information
///
/// # Errors
/// * If HTTP provider creation fails
/// * If contract calls fail
/// * If database operations fail
pub async fn read_all_pairs_v2(factory: Address, batch_size: u64) -> Result<(), eyre::Report> {
    let context = AppContext::new().await.expect("Failed to create context");
    let mut conn = context.conn;
    let provider = context.base_remote;

    // Get last saved pair index
    let mut start = (DbManager::get_last_pair_index(&mut conn, &factory.to_string())?)
        .map_or_else(|| U256::from(0), |last_index| U256::from(last_index + 1));

    let uniswap_v2_batch_request = IUniswapV2BatchRequest::new(
        address!("0x72D6545d3F45F20754F66a2B99fc1A4D75BFEf5c"),
        provider,
    );

    let pairs_len = uniswap_v2_batch_request
        .allPairsLength(factory)
        .call()
        .await?
        ._0;
    println!("Resuming from index {start}, total pairs: {pairs_len}");

    while start < pairs_len {
        let end = (start.add(U256::from(batch_size))).min(pairs_len);

        // Process single batch
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let pairs = match read_pairs_v2_by_range(factory, start, end).await {
            Ok(pairs) => pairs,
            Err(e) => {
                println!("Error fetching pairs for range {start} to {end}: {e}");
                start = end;
                continue;
            }
        };
        println!("Processing batch: {start} to {end}");

        // Convert pairs to database format
        let mut dex_infos = Vec::new();
        let uniswap_factory = NewFactory {
            address: factory.to_string(),
            version: "2".parse()?,
            fee: 300,
            name: "Uniswap V2".parse()?,
        };

        for pair in pairs {
            let token0 = NewToken {
                address: pair.token0.address.to_string(),
                symbol: pair.token0.symbol,
                name: pair.token0.name,
                decimals: pair.token0.decimals,
            };
            let token1 = NewToken {
                address: pair.token1.address.to_string(),
                symbol: pair.token1.symbol,
                name: pair.token1.name,
                decimals: pair.token1.decimals,
            };
            dex_infos.push((
                uniswap_factory.clone(),
                token0,
                token1,
                pair.address.to_string(),
            ));
        }

        // Save batch to database
        let _ = DbManager::batch_save_dex_info(&mut conn, dex_infos);

        start = end;
    }

    Ok(())
}

/// Retrieves reserves for a list of pairs
///
/// # Arguments
/// * `context` - Application context
/// * `pairs` - Vector of pair addresses
///
/// # Returns
/// Vector of `Reserves` containing reserve information for each pair
///
/// # Panics
/// * If contract call to get reserves fails
/// * If batch request contract initialization fails
pub async fn read_reserves_by_range(pairs: Vec<Address>) -> Vec<Reserves> {
    let provider = create_http_provider().await.unwrap();
    let uniswap_v2_batch_request = IUniswapV2BatchRequest::new(
        address!("0x72D6545d3F45F20754F66a2B99fc1A4D75BFEf5c"),
        // context.base_remote, // Using base_remote as the provider
        provider
    );

    println!("pairs: {pairs:?}");

    uniswap_v2_batch_request
        .getReservesByPairs(pairs)
        .call()
        .await
        .unwrap()
        ._0
        .into_iter()
        .map(|reserves| reserves.into())
        .collect()
}

/// Retrieves reserves for all pairs in the database
///
/// # Arguments
/// * `batch_size` - Number of pairs to process in each batch
///
/// # Returns
/// Vector of tuples containing pair address and its reserves
///
/// # Panics
/// * If database connection fails
/// * If HTTP provider creation fails
/// * If contract calls fail
/// * If pair addresses cannot be parsed
pub async fn read_all_reserves(batch_size: usize) -> Vec<(String, Reserves)> {
    let mut context = AppContext::new().await.expect("app context");
    let pairs = PairService::read_all_pairs(&mut context.conn);
    let mut all_reserves = Vec::with_capacity(pairs.len());

    // Process pairs in batches sequentially
    for chunk in pairs.chunks(batch_size) {
        let addresses: Vec<Address> = chunk
            .iter()
            .map(|pair| Address::from_str(&pair.address).unwrap())
            .collect();

        // Process single batch
        let reserves = read_reserves_by_range(addresses).await;

        // Add results to all_reserves
        for (pair, reserve) in chunk.iter().zip(reserves) {
            all_reserves.push((pair.address.clone(), reserve));
        }

        // Add delay between batches
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    all_reserves
}

