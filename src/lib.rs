/*!
 * # Fly - Blockchain Arbitrage Detection and Execution
 *
 * Fly is a Rust-based system for detecting and executing arbitrage opportunities
 * across decentralized exchanges on Ethereum and other EVM-compatible blockchains.
 *
 * ## Core Features
 *
 * - **Arbitrage Detection**: Identifies profitable trading cycles across multiple pools
 * - **Real-time Monitoring**: Continuously monitors blockchain state for opportunities
 * - **Execution Engine**: Executes trades when profitable opportunities are found
 * - **Risk Management**: Implements safeguards to manage trading risks
 *
 * ## Module Structure
 *
 * - `arb`: Core arbitrage detection and execution logic
 * - `bootstrap`: System initialization and startup procedures
 * - `config`: Configuration management for the system
 * - `db_service`: Database interaction for persistent storage
 * - `models`: Data models for the application
 * - `schemas`: Database schema definitions
 * - `sync`: Blockchain synchronization components
 * - `utils`: Utility functions and helpers
 */

/// Arbitrage detection and execution logic
pub mod arb;
/// System initialization and startup procedures
pub mod bootstrap;
/// Configuration management for the system
pub mod config;
/// Data models for the application
pub mod models;
/// Database schema definitions
pub mod schemas;
/// Blockchain synchronization components
pub mod sync;
/// Utility functions and helpers
pub mod utils;
