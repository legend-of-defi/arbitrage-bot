use std::hash::Hash;

/// Type alias for a pool address, represented as a string.
pub type PoolAddress = String;

/// Type alias for a token address, represented as a string.
pub type TokenAddress = String;

/// Pool as it comes from the database or Sync events
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Pool {
    /// The address of the pool
    pub address: PoolAddress,
    /// The address of the first token in the pool
    pub token0: TokenAddress,
    /// The address of the second token in the pool
    pub token1: TokenAddress,
    /// The reserve amount of the first token
    pub reserve0: u64,
    /// The reserve amount of the second token
    pub reserve1: u64,
}
