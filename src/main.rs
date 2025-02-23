use std::sync::Arc;
use alloy::primitives::address;
use crate::utils::logger::setup_logger;
use crate::{bot::Bot, db_service::PairService, utils::app_context::AppContext};
use mev_eth::bootstrap::{read_all_reserves, read_reserves_by_range};

mod arb;
mod bot;
mod core;
mod models;
mod schemas;
mod db_service;
mod config;
mod bootstrap;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut context = AppContext::new().await.expect("Failed to create context");
    setup_logger().expect("Failed to set up logger");

    // Display all pairs with token information
    // let pairs = PairService::read_all_pairs(&mut context.conn);

    // println!("\nFound {} pairs", pairs.len());
    //
    // for pair in pairs {
    //     if let Some((pair, token0, token1)) = PairService::read_pair_with_tokens(&mut context.conn, pair.id) {
    //         println!(
    //             "Pair: {} - Token0: {} ({}), Token1: {} ({})",
    //             pair.address,
    //             token0.symbol.unwrap_or_else(|| "Unknown".to_string()),
    //             token0.address,
    //             token1.symbol.unwrap_or_else(|| "Unknown".to_string()),
    //             token1.address,
    //         );
    //     }
    // }

    // let all_reserves = read_all_reserves(10).await;
    let reserves = read_reserves_by_range(
        vec![
            address!("0x51ca916f50d2f076ba2314d7d43412be786394f6"),
            address!("0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc"),
            address!("0x88a43bbdf9d098eec7bceda4e2494615dfd9bb9c"),
            address!("0x6c39acc16deb25b496c24c21e1e5f5e192ed01c8")
        ]
    ).await;
    println!("\nFound {reserves:?} reserves");

    let bot = Arc::new(Bot::new(context));
    start_bot(bot).await;

    Ok(())
}

async fn start_bot(bot: Arc<Bot>) {
    match bot.start().await {
        Ok(()) => println!("Bot started"),
        Err(e) => println!("Error starting bot: {e}"),
    }
}
