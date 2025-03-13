//! # Arbitrage Module
//!
//! This module contains core arbitrage detection and execution logic.
//! It provides tools for identifying profitable trading opportunities
//! across multiple pools and calculating optimal execution strategies.

/// Core cycle detection and analysis
pub mod cycle;
/// Quote generation for cycles
mod cycle_quote;
/// Pool data structures and operations
pub mod pool;
/// Portfolio management
mod portfolio;
/// Swap execution and modeling
pub mod swap;
/// Swap quote generation
pub mod swap_quote;
/// Test helpers and utilities
mod test_helpers;
/// Token data structures and utilities
pub mod token;
/// Common type definitions
pub mod types;
/// Market world state
pub mod world;
/// World state update mechanisms
pub mod world_update;
