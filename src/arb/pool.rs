/// Market expects Pools to be this.
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use alloy::primitives::{Address, U256};
use eyre::Result;

use super::token::TokenId;

/// A unique identifier for a pool
/// This is just an Address for now, but, in the future, it will also include a chain id
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PoolId(Address);

impl Debug for PoolId {
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

impl TryFrom<&str> for PoolId {
    type Error = eyre::Error;

    fn try_from(s: &str) -> Result<Self> {
        // Parse the string as an Address
        Address::parse_checksummed(s, None)
            .map(Self)
            .map_err(|e| eyre::eyre!("Invalid pool address: {e}"))
    }
}

impl TryFrom<String> for PoolId {
    type Error = eyre::Error;

    fn try_from(s: String) -> Result<Self> {
        Self::try_from(s.as_str())
    }
}

impl From<Address> for PoolId {
    fn from(addr: Address) -> Self {
        Self(addr)
    }
}

impl Display for PoolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pool as it comes from the database or Sync events
#[derive(Debug, Clone, Eq)]
pub struct Pool {
    pub id: PoolId,
    pub token0: TokenId,
    pub token1: TokenId,
    pub reserve0: Option<U256>,
    pub reserve1: Option<U256>,
}

/// Two pools are equal if they have the same address
/// This is for `HashSet` operations
impl PartialEq for Pool {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Hash the pool by its address
/// This is for `HashSet` operations
impl Hash for Pool {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Pool {
    pub const fn new(
        id: PoolId,
        token0: TokenId,
        token1: TokenId,
        reserve0: Option<U256>,
        reserve1: Option<U256>,
    ) -> Self {
        Self {
            id,
            token0,
            token1,
            reserve0,
            reserve1,
        }
    }
}
