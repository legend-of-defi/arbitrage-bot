#![allow(dead_code, unused_variables)]
use std::env;
use std::sync::Arc;

use crate::bootstrap::read_all_pairs_v2;
use crate::bot::Bot;
use crate::config::Config;
use crate::db_service::PairService;
use crate::utils::app_context::AppContext;
use crate::utils::db_connect::establish_connection;
use crate::utils::logger::setup_logger;
use crate::utils::providers::create_http_provider;
use alloy::primitives::address;
use clap::{Parser, Subcommand};
use mev_eth::bootstrap::read_all_reserves;

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
    let pairs =
        read_all_pairs_v2(address!("0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6"), 3000).await;

    println!("pairs: {pairs:?}");

    let mut conn = establish_connection()?;

    // Display all pairs with token information
    let pairs = PairService::read_all_pairs(&mut conn);

    println!("\nFound {} pairs", pairs.len());

    for pair in pairs {
        if let Some((pair, token0, token1)) = PairService::read_pair_with_tokens(&mut conn, pair.id)
        {
            println!(
                "Pair: {} - Token0: {} ({}), Token1: {} ({})",
                pair.address,
                token0.symbol.unwrap_or_else(|| "Unknown".to_string()),
                token0.address,
                token1.symbol.unwrap_or_else(|| "Unknown".to_string()),
                token1.address,
            );
        }
    }

    println!("Database connected successfully!");

    let mut context = AppContext::new().await.expect("Failed to create context");

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

    let all_reserves = read_all_reserves(3000).await;

    println!("\nFound {all_reserves:?} reserves");

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
