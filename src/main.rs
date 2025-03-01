#![allow(dead_code, unused_variables)]

use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use crate::arb::market::Market;
use crate::arb::token::TokenId;
use crate::bootstrap::{fetch_all_pools, fetch_all_pairs_v2};
use crate::bot::Bot;
use crate::config::Config;
use crate::db_service::PairService;
use crate::notify::SlackNotifier;
use crate::utils::app_context::AppContext;
use crate::utils::db_connect::establish_connection;
use crate::utils::logger::setup_logger;
use crate::utils::providers::create_http_provider;
use alloy::primitives::{address, U256};
use clap::{Parser, Subcommand};
use fly::sync::subscriber::subscribe_to_sync;

mod arb;
mod bootstrap;
mod bot;
mod config;
mod core;
mod db_service;
mod models;
mod notify;
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
    /// Process batches only
    Batches,
    /// Skip batch processing and start the bot
    Start,
    /// Send slack message
    Slack {
        message: String,
    },
    /// Send slack error message
    SlackError {
        message: String,
    },
}

async fn process_batches() -> Result<(), Box<dyn std::error::Error>> {
    let _config = Config::from_env();
    setup_logger().expect("Failed to set up logger");
    println!(
        "Server Started with DATABASE_URL: {}",
        env::var("DATABASE_URL").unwrap()
    );

    let _provider = create_http_provider().await?;
    let _ = fetch_all_pairs_v2(address!("0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6"), 3000).await;

    let mut conn = establish_connection()?;

    // Display all pairs with token information
    let pairs = PairService::read_all_pairs(&mut conn);

    println!("\nFound {} pairs", pairs.len());

    println!("Database connected successfully!");

    let context = AppContext::new().await.expect("Failed to create context");

    let pools = fetch_all_pools(3000).await;
    let num_pairs = pools.len();
    let mut balances = HashMap::with_capacity(num_pairs);

    // Tether Address on base (we can update it later)
    balances.insert(
        TokenId::from("0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2"),
        U256::from(0),
    );
    let _market = Market::new(&pools, balances);

    subscribe_to_sync().await?;

    Ok(())
}

async fn run_default_behavior() -> Result<(), Box<dyn std::error::Error>> {
    // Process batches first
    process_batches().await?;

    // Then start the bot
    let context = AppContext::new().await.expect("Failed to create context");
    let bot = Arc::new(Bot::new(context));
    start_bot(bot).await;

    Ok(())
}

async fn start_bot_only() -> Result<(), Box<dyn std::error::Error>> {
    let _config = Config::from_env();
    setup_logger().expect("Failed to set up logger");
    println!("Starting bot without batch processing...");

    let context = AppContext::new().await.expect("Failed to create context");
    let bot = Arc::new(Bot::new(context));
    start_bot(bot).await;

    Ok(())
}

async fn send_slack_message(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let notifier = SlackNotifier::new()?;
    notifier.send(message).await?;
    Ok(())
}

async fn send_slack_error_message(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let notifier = SlackNotifier::new()?;
    notifier.send_error(message).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Batches) => {
            // Only process batches, don't start the bot
            println!("Processing batches only...");
            process_batches().await?;
        }
        Some(Commands::Start) => {
            // Skip batch processing and just start the bot
            start_bot_only().await?;
        }
        Some(Commands::Slack { message }) => {
            send_slack_message(&message).await?;
        }
        Some(Commands::SlackError { message }) => {
            send_slack_error_message(&message).await?;
        }
        None => {
            // Default behavior: process batches then start the bot
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
