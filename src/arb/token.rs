/// A token is what we are trading
/// Here, mostly for type safety.
use alloy::primitives::Address;
use core::fmt::{self, Debug};
use eyre::Result;
use std::fmt::Display;

/// Globally unique identifier for a token to distinguish between different chains
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TokenId(pub Address);

impl TryFrom<&str> for TokenId {
    type Error = eyre::Error;

    fn try_from(s: &str) -> Result<Self> {
        // Parse the string as an Address
        Address::parse_checksummed(s, None)
            .map(Self)
            .map_err(|e| eyre::eyre!("Invalid token address: {e}"))
    }
}

/// Custom Debug implementation to truncate trailing zeros.
/// This is useful for testing where we deterministically generate the addresses from short strings
/// and pad them with zeros to ensure they are 40 characters long and pass the checksum test.
/// However, we don't want to print the full 40 character hex string when debugging.
/// There is is '10 zeros test' in case we have some real address that have trailing zeros.
/// We consider the odds of 10+ trailing zeros to be so low that we can safely truncate.
impl Debug for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = format!("{}", self.0);
        let hex = hex.trim_start_matches("0x").to_uppercase();
        let zeros = hex.chars().rev().take_while(|&c| c == '0').count();
        if zeros > 10 {
            let trimmed = hex.trim_end_matches('0');
            write!(f, "{trimmed}")
        } else {
            write!(f, "{hex}")
        }
    }
}

impl TryFrom<String> for TokenId {
    type Error = eyre::Error;

    fn try_from(s: String) -> Result<Self> {
        Self::try_from(s.as_str())
    }
}

impl From<Address> for TokenId {
    fn from(address: Address) -> Self {
        Self(address)
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
        write!(f, "{:?}", self.id)
    }
}

impl Token {
    pub const fn new(id: TokenId) -> Self {
        Self { id }
    }
}
