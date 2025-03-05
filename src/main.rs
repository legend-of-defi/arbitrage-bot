#![allow(dead_code, unused_variables)]

use std::env;
use std::sync::Arc;

use crate::arb::world::World;
use crate::bootstrap::{fetch_all_pairs_v2_by_factory, fetch_all_pools};
use crate::bot::Bot;
use crate::db_service::PairService;
use crate::notify::SlackNotifier;
use crate::sync::subscriber::subscribe_to_sync;
use crate::utils::app_context::AppContext;
use crate::utils::logger::setup_logger;
use alloy::primitives::address;
use clap::{Parser, Subcommand};
use eyre::{Error, Result};
use log::info;
use fly::bootstrap::start_pool_monitoring;

mod arb;
mod bootstrap;
mod bot;
mod config;
mod core;
mod db_service;
mod models;
mod notify;
mod schemas;
mod sync;
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
    Slack { message: String },
    /// Send slack error message
    SlackError { message: String },
}

async fn run_default_behavior(mut ctx: AppContext) -> Result<(), Error> {
    info!(
        "Server Started with DATABASE_URL: {}",
        env::var("DATABASE_URL")?
    );

    let uniswap_v2_factory_base = address!("0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6");
    fetch_all_pairs_v2_by_factory(&mut ctx, uniswap_v2_factory_base, 3000).await?;

    // Display all pairs with token information
    let pairs = PairService::read_all_pairs(&mut ctx.pg_connection);

    println!("\nFound {} pairs", pairs.len());

    println!("Database connected successfully!");

    let pools = fetch_all_pools(&mut ctx, 3000_usize).await?;
    let num_pairs = pools.len();

    let _world = World::new(&pools);

    start_pool_monitoring(&mut ctx, 300)?;

    subscribe_to_sync(&ctx).await?;

    let bot = Arc::new(Bot::new(ctx));
    start_bot(bot).await;

    Ok(())
}

async fn start_bot_only(ctx: AppContext) -> Result<(), Error> {
    println!("Starting bot without batch processing...");

    let bot = Arc::new(Bot::new(ctx));
    start_bot(bot).await;

    Ok(())
}

async fn send_slack_message(message: &str) -> Result<(), Error> {
    let notifier = SlackNotifier::new()?;
    notifier.send(message).await?;
    Ok(())
}

async fn send_slack_error_message(message: &str) -> Result<(), Error> {
    let notifier = SlackNotifier::new()?;
    notifier.send_error(message).await?;
    Ok(())
}

async fn process_batches(mut ctx: AppContext) -> Result<(), Error> {
    let uniswap_v2_factory_base = address!("0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6");
    fetch_all_pairs_v2_by_factory(&mut ctx, uniswap_v2_factory_base, 750).await?;
    println!("Batch processing completed successfully");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_logger().expect("Failed to set up logger");

    let ctx = AppContext::new().await?;

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Batches) => {
            // Only process batches, don't start the bot
            println!("Processing batches only...");
            process_batches(ctx).await?;
        }
        Some(Commands::Start) => {
            // Skip batch processing and just start the bot
            start_bot_only(ctx).await?;
        }
        Some(Commands::Slack { message }) => {
            send_slack_message(&message).await?;
        }
        Some(Commands::SlackError { message }) => {
            send_slack_error_message(&message).await?;
        }
        none => {
            // Default behavior when no subcommand is provided
            run_default_behavior(ctx).await?;
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
