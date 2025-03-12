#![allow(dead_code)]
use alloy::primitives::U256;
use std::collections::HashMap;

use super::token::TokenId;

/// Represents a portfolio of token holdings.
///
/// A portfolio tracks the balances of various tokens identified by their `TokenId`.
/// It provides methods to query and manage these balances.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Portfolio {
    /// Map of token IDs to their respective balances
    pub holdings: HashMap<TokenId, U256>,
}

impl Portfolio {
    /// Creates a new portfolio with the given token holdings.
    ///
    /// # Arguments
    ///
    /// * `holdings` - A map of token IDs to their respective balances
    ///
    /// # Returns
    ///
    /// A new Portfolio instance
    pub const fn new(holdings: HashMap<TokenId, U256>) -> Self {
        Self { holdings }
    }

    /// Returns the balance of a specific token in the portfolio.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the token to query
    ///
    /// # Returns
    ///
    /// The token balance as a U256 value if the token exists in the portfolio,
    /// or None if the token is not in the portfolio
    pub fn balance(&self, token_id: &TokenId) -> Option<U256> {
        self.holdings.get(token_id).copied()
    }
}
