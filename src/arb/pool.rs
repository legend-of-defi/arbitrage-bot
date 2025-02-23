/// Market expects Pools to be this.
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use alloy::primitives::{Address, U256};

use super::token::TokenId;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PoolId(String);

impl From<&str> for PoolId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<Address> for PoolId {
    fn from(addr: Address) -> Self {
        Self(format!("{addr:?}"))
    }
}

impl Display for PoolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pool as it comes from the database or Sync events
#[derive(Debug, Clone, Eq)]
#[allow(dead_code)]
pub struct Pool {
    pub id: PoolId,
    pub token0: TokenId,
    pub token1: TokenId,
    pub reserve0: U256,
    pub reserve1: U256,
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
    #[allow(dead_code)]
    pub const fn new(
        id: PoolId,
        token0: TokenId,
        token1: TokenId,
        reserve0: U256,
        reserve1: U256,
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
