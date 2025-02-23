use std::hash::Hash;

#[allow(dead_code)]
pub type PoolAddress = String;

#[allow(dead_code)]
pub type TokenAddress = String;

/// Pool as it comes from the database or Sync events
#[derive(Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct Pool {
    pub address: PoolAddress,
    pub token0: TokenAddress,
    pub token1: TokenAddress,
    pub reserve0: u64,
    pub reserve1: u64,
}
