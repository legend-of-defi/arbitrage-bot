#![allow(dead_code, unused_variables)]

use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;

use crate::bootstrap::{read_all_pairs_v2, read_all_reserves};
use crate::bot::Bot;
use crate::config::Config;
use crate::db_service::PairService;
use crate::utils::app_context::AppContext;
use crate::utils::db_connect::establish_connection;
use crate::utils::logger::setup_logger;
use crate::utils::providers::create_http_provider;
use crate::arb::market::Market;
use alloy::primitives::{address, U256};
use clap::{Parser, Subcommand};
use fly::sync::subscriber::subscribe_to_sync;
use crate::arb::pool::{Pool, PoolId};
use crate::arb::token::TokenId;

mod arb;
mod bootstrap;
mod bot;
mod config;
mod core;
mod db_service;
mod models;
mod schemas;
mod utils;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Process batches
    Batches,
}

async fn run_default_behavior() -> Result<(), Box<dyn std::error::Error>> {
    let _config = Config::from_env();
    setup_logger().expect("Failed to set up logger");
    println!(
        "Server Started with DATABASE_URL: {}",
        env::var("DATABASE_URL").unwrap());

    let _provider = create_http_provider().await?;
    let _ =
        read_all_pairs_v2(address!("0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6"), 3000).await;


    let mut conn = establish_connection()?;

    // Display all pairs with token information
    let pairs = PairService::read_all_pairs(&mut conn);

    println!("\nFound {} pairs", pairs.len());

    println!("Database connected successfully!");

    let mut context = AppContext::new().await.expect("Failed to create context");

    // Display all pairs with token information
    // let pairs = PairService::read_all_pairs(&mut context.conn);

    // println!("\nFound {} pairs", pairs.len());
    let all_reserves = read_all_reserves(3000).await;

    let num_pairs = all_reserves.len();
    let mut pools = HashSet::with_capacity(num_pairs);
    let mut balances = HashMap::with_capacity(num_pairs);

    for (pair_id, reserve) in all_reserves {
        if let Some((pair, token0, token1)) = PairService::read_pair_with_tokens(&mut context.conn, pair_id) {
            pools.insert(
                Pool::new(
                    PoolId(pair.address),
                    TokenId::from(token0.address),
                    TokenId::from(token1.address),
                    reserve.reserve0,
                    reserve.reserve1
                )
            );
        }
    }

    // Tether Address on base (we can update it later)
    balances.insert(TokenId::from("0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2".parse().unwrap()), U256::from(0));
    let _market = Market::new(&pools, balances);

    subscribe_to_sync().await?;

    let bot = Arc::new(Bot::new(context));
    start_bot(bot).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Batches) => {
            // Run the same behavior as default for now
            run_default_behavior().await?;
        }
        None => {
            // Default behavior when no subcommand is provided
            run_default_behavior().await?;
        }
    }

    Ok(())
}

async fn start_bot(bot: Arc<Bot>) {
    match bot.start().await {
        Ok(()) => println!("Bot started"),
        Err(e) => println!("Error starting bot: {e}"),
    }
}
