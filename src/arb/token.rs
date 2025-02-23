/// A token is what we are trading
/// Here, mostly for type safety.
use alloy::primitives::Address;
use core::fmt::{self, Debug};
use std::fmt::Display;

/// Globally unique identifier for a token to distinguish between different chains
#[derive(Clone, Default, PartialEq, Eq, Hash, Ord, PartialOrd, Debug)]
pub struct TokenId(String);

impl From<&str> for TokenId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<Address> for TokenId {
    fn from(addr: Address) -> Self {
        Self(format!("{addr:?}"))
    }
}

impl Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Token {
    pub id: TokenId,
}

impl Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl Token {
    pub const fn new(id: TokenId) -> Self {
        Self { id }
    }
}
