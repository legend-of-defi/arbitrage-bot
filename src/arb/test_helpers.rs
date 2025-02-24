use crate::arb::pool::Pool;
use alloy::primitives::U256;

use super::cycle::Cycle;
use super::pool::PoolId;
use super::swap_quote::SwapQuote;
use super::swap_side::{Direction, SwapId};
use super::token::{Token, TokenId};
use super::{market::Market, swap_side::SwapSide};

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
pub fn swap(pool_id: &str, direction: Direction, token0: &str, token1: &str, reserve0: u64, reserve1: u64) -> SwapSide {
    SwapSide::new(
        SwapId {
            pool: PoolId::from(pool_id),
            direction,
        },
        TokenId::from(token0),
        TokenId::from(token1),
        U256::from(reserve0),
        U256::from(reserve1),
    )
}

pub fn swap_quote(id: &str, token0: &str, token1: &str, reserve0: u64, reserve1: u64, amount_in: u64) -> SwapQuote {
    SwapQuote::new(
        &swap(id, Direction::ZeroForOne, token0, token1, reserve0, reserve1),
        U256::from(amount_in),
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
pub fn swap_by_index(market: &Market, index: usize) -> &SwapSide {
    &market.swap_vec[index]
}

#[allow(dead_code)]
pub fn cycle(swaps: &[(&str, Direction, &str, &str, u64, u64)]) -> Cycle {
    let swaps = swaps
        .iter()
        .map(|(id, direction, token0, token1, reserve0, reserve1)| {
            swap(id, direction.clone(), token0, token1, *reserve0, *reserve1)
        })
        .collect();
    Cycle::new(swaps).unwrap()
}
