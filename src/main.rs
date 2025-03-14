#![doc = "Fly - Blockchain Arbitrage Detection and Execution CLI"]

/// Main entry point for the application
use crate::utils::app_context::AppContext;
use crate::utils::logger::setup_logger;
use clap::{Parser, Subcommand};
use eyre::Result;

mod arb;
/// System initialization and startup procedures
mod bootstrap;
/// Arbitrage detection and execution logic
mod bot;
/// Configuration management for the system
mod config;
/// Data models for the application
mod models;
/// Notification system
mod notify;
/// Database schema definitions
mod schemas;
/// Blockchain synchronization components
mod sync;
/// Utility functions and helpers
mod utils;

/// Command line interface
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// [DEBUG] Sync `Sync` events
    SyncSyncEvents,
    /// [DEBUG] Sync pairs with missing reserves
    SyncReserves,
    /// [DEBUG] Sync pairs tokens
    SyncPairTokens,
    /// [DEBUG] Sync factory pairs
    SyncFactoryPairs,
    /// [DEBUG] Sync factories
    SyncFactories,
    /// [DEBUG] Sync USD values for pairs
    SyncUsd,
    /// [DEBUG] Sync `PairCreated` events
    SyncPairCreatedEvents,
    /// [DEBUG] Sync exchange rates
    SyncExchangeRates,
    /// [DEBUG] Sync WETH price from Moralis API
    SyncWeth,
    /// Start the bot
    Start,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger()?;

    let ctx = AppContext::new().await?;

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::SyncSyncEvents) => {
            sync::events(&ctx).await?;
        }
        Some(Commands::SyncReserves) => {
            sync::reserves(&ctx).await?;
        }
        Some(Commands::SyncPairTokens) => {
            sync::pair_tokens(&ctx).await?;
        }
        Some(Commands::SyncFactoryPairs) => {
            sync::factory_pairs(&ctx).await?;
        }
        Some(Commands::SyncFactories) => {
            sync::factories(&ctx).await?;
        }
        Some(Commands::SyncUsd) => {
            sync::usd(&ctx).await?;
        }
        Some(Commands::SyncPairCreatedEvents) => {
            sync::pair_created_events(&ctx).await?;
        }
        Some(Commands::SyncExchangeRates) => {
            sync::exchange_rates(&ctx).await?;
        }
        Some(Commands::SyncWeth) => {
            sync::weth(&ctx).await?;
        }
        Some(Commands::Start) => {
            bot::start(ctx).await?;
        }
        None => {
            log::error!("No command provided");
        }
    }

    Ok(())
}
