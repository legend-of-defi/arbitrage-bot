use crate::arb::pool::Pool;
use alloy::primitives::U256;

use super::cycle::Cycle;
use super::pool::PoolId;
use super::swap::SwapId;
use super::token::{Token, TokenId};
use super::{market::Market, swap::Swap};

#[allow(dead_code)]
pub fn market(pool_args: &[(&str, &str, &str, u64, u64)], balances: &[(&str, u128)]) -> Market {
    let pools = &pool_args
        .iter()
        .map(|(id, token0, token1, reserve0, reserve1)| {
            pool(id, token0, token1, *reserve0, *reserve1)
        })
        .collect();

    let balances = balances
        .iter()
        .map(|(token, balance)| (TokenId::from(*token), U256::from(*balance)))
        .collect();

    Market::new(pools, balances)
}

#[allow(dead_code)]
pub fn token(id: &str) -> Token {
    Token::new(TokenId::from(id))
}

#[allow(dead_code)]
pub fn swap(id: &str, token0: &str, token1: &str, reserve0: u64, reserve1: u64) -> Swap {
    Swap::new(
        SwapId::from(id),
        PoolId::from(id),
        TokenId::from(token0),
        TokenId::from(token1),
        U256::from(reserve0),
        U256::from(reserve1),
    )
}

#[allow(dead_code)]
pub fn pool(symbol: &str, token0: &str, token1: &str, reserve0: u64, reserve1: u64) -> Pool {
    Pool::new(
        PoolId::from(symbol),
        TokenId::from(token0),
        TokenId::from(token1),
        U256::from(reserve0),
        U256::from(reserve1),
    )
}

#[allow(dead_code)]
pub fn swap_by_index(market: &Market, index: usize) -> &Swap {
    &market.swap_vec[index]
}

#[allow(dead_code)]
pub fn cycle(swaps: &[(&str, &str, &str, u64, u64)]) -> Cycle {
    let swaps = swaps
        .iter()
        .map(|(id, token0, token1, reserve0, reserve1)| {
            swap(id, token0, token1, *reserve0, *reserve1)
        })
        .collect();
    Cycle::new(swaps).unwrap()
}
