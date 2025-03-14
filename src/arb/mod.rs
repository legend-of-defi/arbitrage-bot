/*!
 * # Arbitrage Module
 *
 * This module contains the core logic for detecting and executing arbitrage opportunities
 * across decentralized exchanges. It implements graph-based algorithms to find profitable
 * trading cycles and provides mechanisms to execute these trades.
 *
 * ## Key Components
 *
 * - `cycle`: Defines the `Cycle` struct representing a sequence of swaps forming a trading cycle
 * - `cycle_quote`: Provides quote calculation for cycles to determine profitability
 * - `pool`: Represents liquidity pools where tokens can be exchanged
 * - `portfolio`: Manages token holdings and balances
 * - `swap`: Defines individual swap operations between tokens
 * - `swap_quote`: Calculates expected outputs for individual swaps
 * - `token`: Token identification and metadata
 * - `world`: Graph representation of the trading environment
 * - `world_update`: Mechanisms to update the world state based on new information
 */

/// Core cycle detection and representation
mod cycle;
/// Cycle profitability calculation
mod cycle_quote;
/// Liquidity pool representation and operations
pub mod pool;
/// Token portfolio management
mod portfolio;
/// Individual swap operations
mod swap;
/// Swap quote calculation
pub mod swap_quote;
/// Helpers for testing
mod test_helpers;
/// Token identification and metadata
pub mod token;
/// Common types used across the arbitrage module
mod types;
/// Graph representation of the trading environment
pub mod world;
/// World state update mechanisms
mod world_update;
