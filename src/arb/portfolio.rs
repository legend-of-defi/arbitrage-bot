#![allow(dead_code)]
use alloy::primitives::U256;
use std::collections::HashMap;

use super::token::TokenId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Portfolio {
    pub holdings: HashMap<TokenId, U256>,
}

impl Portfolio {
    pub const fn new(holdings: HashMap<TokenId, U256>) -> Self {
        Self { holdings }
    }

    pub fn balance(&self, token_id: &TokenId) -> Option<U256> {
        self.holdings.get(token_id).copied()
    }
}
