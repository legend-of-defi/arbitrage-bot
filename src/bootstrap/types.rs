use super::UniswapQuery;
use crate::models::token::NewToken;
use alloy::primitives::{Address, U256};

#[derive(Debug)]
pub struct TokenInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: i32,
}

#[derive(Debug)]
pub struct PairInfo {
    pub address: Address,
    pub token0: NewToken,
    pub token1: NewToken,
}

impl From<UniswapQuery::PairInfo> for PairInfo {
    fn from(pair: UniswapQuery::PairInfo) -> Self {
        let token0 = NewToken::new(
            pair.token0.tokenAddress,
            Some(pair.token0.symbol),
            Some(pair.token0.name),
            i32::from(pair.token0.decimals),
            None,
            None,
            None,
        );

        let token1 = NewToken::new(
            pair.token1.tokenAddress,
            Some(pair.token1.symbol),
            Some(pair.token1.name),
            i32::from(pair.token1.decimals),
            None,
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

#[derive(Debug)]
pub struct Reserves {
    pub reserve0: U256,
    pub reserve1: U256,
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
