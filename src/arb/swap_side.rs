/// A swap side represents one of the two swaps sides in a pool: the `ZeroForOne` or `OneForZero`
/// Used to calculate its `log_rate` and an `amount_out` given an `amount_in`
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use alloy::primitives::U256;

use super::pool::{Pool, PoolId};
use super::token::TokenId;

/// The direction of a swap
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Direction {
    ZeroForOne,
    OneForZero,
}

impl Direction {
    pub fn is_opposite(&self, other: &Self) -> bool {
        self == &Self::OneForZero && other == &Self::ZeroForOne
            || self == &Self::ZeroForOne && other == &Self::OneForZero
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// A unique identifier for a swap between two tokens
/// Defines the direction of the swap in a pool
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SwapId {
    pub pool: PoolId,
    pub direction: Direction,
}

impl Display for SwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.pool, self.direction)
    }
}

/// A single swap between two tokens in a pool in one direction or the other
#[derive(Clone, Eq)]
pub struct SwapSide {
    pub id: SwapId,
    pub token0: TokenId,
    pub token1: TokenId,
    pub reserve0: U256,
    pub reserve1: U256,
    pub log_rate: i64,
}

impl PartialEq for SwapSide {
    fn eq(&self, other: &Self) -> bool {
        self.token0 == other.token0 && self.token1 == other.token1 && self.id == other.id
    }
}

impl PartialOrd for SwapSide {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SwapSide {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.token0
            .cmp(&other.token0)
            .then(self.token1.cmp(&other.token1))
            .then(self.id.cmp(&other.id))
    }
}

impl Debug for SwapSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            // Swap(pool, 1000 WETH / 100 USDC @ 10)
            "Swap({}, {} {} / {} {} @ {})",
            self.id, self.reserve0, self.token0, self.reserve1, self.token1, self.log_rate
        )
    }
}

impl Display for SwapSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Swap({}, {}{}/{}{} @{})",
            self.id, self.reserve0, self.token0, self.reserve1, self.token1, self.log_rate
        )
    }
}

impl Hash for SwapSide {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.token0.hash(state);
        self.token1.hash(state);
    }
}

impl SwapSide {
    #[allow(dead_code)]
    pub fn new(
        id: SwapId,
        token0: TokenId,
        token1: TokenId,
        reserve0: U256,
        reserve1: U256,
    ) -> Self {
        let log_rate = Self::log_rate(reserve0, reserve1);

        Self {
            id,
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
        let swap_id = SwapId {
            pool: pool.id.clone(),
            direction: Direction::ZeroForOne,
        };
        Self::new(swap_id, token0, token1, reserve0, reserve1)
    }

    pub fn reverse(pool: &Pool) -> Self {
        let token0 = pool.token1.clone();
        let token1 = pool.token0.clone();
        let reserve0 = pool.reserve1;
        let reserve1 = pool.reserve0;
        let swap_id = SwapId {
            pool: pool.id.clone(),
            direction: Direction::OneForZero,
        };
        Self::new(swap_id, token0, token1, reserve0, reserve1)
    }

    /// Returns true if the swap side is the `OneForZero` direction
    #[allow(dead_code)]
    pub fn is_one_for_zero(&self) -> bool {
        self.id.direction == Direction::OneForZero
    }

    /// Returns true if the swap side is the `ZeroForOne` direction
    #[allow(dead_code)]
    pub fn is_zero_for_one(&self) -> bool {
        self.id.direction == Direction::ZeroForOne
    }

    /// Returns true if the swap side is the reciprocal of the other swap side,
    /// i.e. it has the same pool but opposite direction
    #[allow(dead_code)]
    pub fn is_reciprocal(&self, other: &Self) -> bool {
        self.id.pool == other.id.pool && self.id.direction.is_opposite(&other.id.direction)
    }

    /// Estimated gas cost of the swap in WETH
    /// This is a rough estimate and should not be relied on
    /// This is based on average Uniswap v2 core swap gas cost of 40k-50k
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
}

#[cfg(test)]
mod tests {
    use crate::arb::swap_side::Direction;
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
            let test_swap = swap("P1", Direction::ZeroForOne, "A", "B", *reserve0, *reserve1);
            assert_eq!(test_swap.log_rate, *expected);
        }
    }
}
