use super::UniswapQuery;
use crate::models::token::NewToken;
use alloy::primitives::{Address, U256};

/// Pair information
#[derive(Debug)]
pub struct PairInfo {
    /// Address of the pair
    #[allow(dead_code)]
    pub address: Address,
    /// Token 0
    pub token0: NewToken,
    /// Token 1
    pub token1: NewToken,
}

impl From<UniswapQuery::PairInfo> for PairInfo {
    fn from(pair: UniswapQuery::PairInfo) -> Self {
        let token0 = NewToken::new(
            pair.token0.tokenAddress,
            Some(pair.token0.symbol),
            Some(pair.token0.name),
            i32::from(pair.token0.decimals),
            true,
            None,
            None,
        );

        let token1 = NewToken::new(
            pair.token1.tokenAddress,
            Some(pair.token1.symbol),
            Some(pair.token1.name),
            i32::from(pair.token1.decimals),
            true,
            None,
            None,
        );

        Self {
            address: pair.pairAddress,
            token0,
            token1,
        }
    }
}

/// Reserves information
#[derive(Debug)]
pub struct Reserves {
    /// Reserve 0
    pub reserve0: U256,
    /// Reserve 1
    pub reserve1: U256,
    /// Block timestamp last
    #[allow(dead_code)]
    pub block_timestamp_last: U256,
}

impl From<[U256; 3]> for Reserves {
    fn from(reserves: [U256; 3]) -> Self {
        Self {
            reserve0: reserves[0],
            reserve1: reserves[1],
            block_timestamp_last: reserves[2],
        }
    }
}
