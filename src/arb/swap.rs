/// A swap is a single swap between two tokens in a pool
/// So, there are two swaps per pool.
/// Used to calculate its `log_rate` and an `amount_out` given an `amount_in`
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use alloy::primitives::U256;

use super::pool::{Pool, PoolId};
use super::token::TokenId;
/// A unique identifier for a swap between two tokens
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SwapId(String);

impl From<&str> for SwapId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Display for SwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single swap between two tokens in a pool
#[derive(Clone, Eq)]
pub struct Swap {
    pub id: SwapId,
    pub pool: PoolId,
    pub token0: TokenId,
    pub token1: TokenId,
    pub reserve0: U256,
    pub reserve1: U256,
    pub log_rate: i64,
}

impl PartialEq for Swap {
    fn eq(&self, other: &Self) -> bool {
        self.token0 == other.token0 && self.token1 == other.token1 && self.id == other.id
    }
}

impl PartialOrd for Swap {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Swap {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.token0
            .cmp(&other.token0)
            .then(self.token1.cmp(&other.token1))
            .then(self.id.cmp(&other.id))
    }
}

impl Debug for Swap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            // Swap(pool, 1000WETH / 100USDC @ 10)
            "Swap({}, {} {} / {} {} @ {})",
            self.id, self.reserve0, self.token0, self.reserve1, self.token1, self.log_rate
        )
    }
}

impl Display for Swap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Swap({}, {}{}/{}{} @{})",
            self.id, self.reserve0, self.token0, self.reserve1, self.token1, self.log_rate
        )
    }
}

impl Hash for Swap {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.token0.hash(state);
        self.token1.hash(state);
    }
}

impl Swap {
    #[allow(dead_code)]
    pub fn new(
        id: SwapId,
        pool: PoolId,
        token0: TokenId,
        token1: TokenId,
        reserve0: U256,
        reserve1: U256,
    ) -> Self {
        let log_rate = Self::log_rate(reserve0, reserve1);

        Self {
            id,
            pool,
            token0,
            token1,
            reserve0,
            reserve1,
            log_rate,
        }
    }

    pub fn forward(pool: &Pool) -> Self {
        let token0 = pool.token0.clone();
        let token1 = pool.token1.clone();
        let reserve0 = pool.reserve0;
        let reserve1 = pool.reserve1;
        let swap_id = SwapId::from(format!("{}-fwd", pool.id).as_str());
        Self::new(swap_id, pool.id.clone(), token0, token1, reserve0, reserve1)
    }

    pub fn reverse(pool: &Pool) -> Self {
        let token0 = pool.token1.clone();
        let token1 = pool.token0.clone();
        let reserve0 = pool.reserve1;
        let reserve1 = pool.reserve0;
        let swap_id = SwapId::from(format!("{}-rev", pool.id).as_str());
        Self::new(swap_id, pool.id.clone(), token0, token1, reserve0, reserve1)
    }

    #[allow(dead_code)]
    pub fn is_reverse(&self, other: &Self) -> bool {
        self.token0 == other.token1 && self.token1 == other.token0 && self.pool == other.pool
    }

    /// Estimated gas cost of the swap in WETH
    /// This is a rough estimate and should not be relied on
    /// This is base on average Uniswap v2 core swap gas cost of 40k-50k
    /// doubled to take into account our contract overhead
    /// TODO: review
    #[allow(dead_code)]
    const fn estimated_gas_cost_in_weth() -> f64 {
        0.0001
    }

    /// Calculate the log rate of a swap for faster computation
    /// We replace rate multiplication with log addition
    #[allow(clippy::cast_possible_truncation)]
    pub fn log_rate(reserve0: U256, reserve1: U256) -> i64 {
        const SCALE: f64 = 1_000_000.0;
        ((reserve1.approx_log10() - reserve0.approx_log10()) * SCALE) as i64
    }

    /// The amount of tokens we get out of the swap given an amount of tokens we put in
    pub fn amount_out(&self, amount_in: U256) -> U256 {
        let fee_numerator = U256::from(997);
        let fee_denominator = U256::from(1000);

        let amount_in_with_fee = amount_in * fee_numerator;
        let numerator = amount_in_with_fee * self.reserve1;
        let denominator = (self.reserve0 * fee_denominator) + amount_in_with_fee;

        numerator / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arb::test_helpers::*;

    #[test]
    fn test_log_rate() {
        for (reserve0, reserve1, expected) in &[
            // reserve0,      reserve1,        expected
            (100, 100, 0),
            // ln(2) = 0.693147
            (100, 200, 301_029),
            // ln(1/2) = -0.693147
            (200, 100, -301_029),
        ] {
            let test_swap = swap("P1", "A", "B", *reserve0, *reserve1);
            assert_eq!(test_swap.log_rate, *expected);
        }
    }

    #[test]
    fn test_amount_out() {
        for (reserve0, reserve1, amount_in, expected) in &[
            (
                1_000_000_000, // reserve0
                1_000_000_000, // reserve1
                100,           // amount_in
                99,            // expected - some slippage
            ),
            (
                1_000_000_000, // reserve0
                1_000_000_000, // reserve1
                10_000_000,    // amount_in
                9_871_580,     // expected - more slippage
            ),
            (
                1_000,
                1_000,
                1_000_000_000,
                999, // the max amount out no matter the amount_in
            ),
        ] {
            let test_swap = swap("P1", "A", "B", *reserve0, *reserve1);
            assert_eq!(
                test_swap.amount_out(U256::from(*amount_in)),
                U256::from(*expected)
            );
        }
    }
}
