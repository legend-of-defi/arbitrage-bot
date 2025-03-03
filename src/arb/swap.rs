/// A swap side represents one of the two swaps sides in a pool: the `ZeroForOne` or `OneForZero`
/// Used to calculate its `log_rate` and an `amount_out` given an `amount_in`
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use alloy::primitives::U256;
use eyre::{bail, Error};

use super::pool::{Pool, PoolId};
use super::token::TokenId;

/// The direction of a swap
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
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

impl Debug for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroForOne => write!(f, "0>1"),
            Self::OneForZero => write!(f, "1>0"),
        }
    }
}

/// A unique identifier for a swap between two tokens
/// Defines the direction of the swap in a pool
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SwapId {
    pub pool_id: PoolId,
    pub direction: Direction,
}

impl Debug for SwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {:?}", self.pool_id, self.direction)
    }
}

impl Display for SwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.pool_id, self.direction)
    }
}

/// A single swap between two tokens in a pool in one direction or the other.
/// This is mostly to codify the direction of the swap. It also knows the reserves and swap log rate.
/// Notably, this does not include swap amounts. That is handled by the `SwapQuote` struct.
#[derive(Clone, Eq)]
pub struct Swap {
    pub id: SwapId,
    pub token_in: TokenId,
    pub token_out: TokenId,
    reserve_in: Option<U256>,
    reserve_out: Option<U256>,
    log_rate: Option<i64>,
}

/// We compare `SwapSide`s by their `token0`, `token1`, and `id` only. Note, that reserves
/// (and thus `log_rate`) are not part of the comparison.
/// This is because when we match them in the market, we do not care about the reserves.
/// We have updated reserves for a swap so we need to find the original one and update it.
impl PartialEq for Swap {
    fn eq(&self, other: &Self) -> bool {
        self.token_in == other.token_in && self.token_out == other.token_out && self.id == other.id
    }
}

impl PartialOrd for Swap {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Swap {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.token_in
            .cmp(&other.token_in)
            .then(self.token_out.cmp(&other.token_out))
            .then(self.id.cmp(&other.id))
    }
}

impl Debug for Swap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            // Swap(pool, 1000 WETH / 100 USDC @ 10)
            "Swap({:?}, {:?} {:?} / {:?} {:?} @ {:?})",
            self.id,
            self.reserve_in,
            self.token_in,
            self.reserve_out,
            self.token_out,
            self.log_rate()
        )
    }
}

impl Display for Swap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Swap({}, {}{}/{}{} @{})",
            self.id,
            self.reserve_in
                .map_or("None".to_string(), |r| r.to_string()),
            self.token_in,
            self.reserve_out
                .map_or("None".to_string(), |r| r.to_string()),
            self.token_out,
            self.log_rate.map_or("None".to_string(), |r| r.to_string())
        )
    }
}

impl Hash for Swap {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.token_in.hash(state);
        self.token_out.hash(state);
    }
}

impl Swap {
    pub fn new(
        id: SwapId,
        token_in: TokenId,
        token_out: TokenId,
        reserve_in: Option<U256>,
        reserve_out: Option<U256>,
    ) -> Result<Self, Error> {
        if token_in == token_out {
            bail!("Swap token0 and token1 must be different");
        }

        assert!(
            reserve_in.is_none() && reserve_out.is_none()
                || reserve_in.is_some() && reserve_out.is_some(),
            "Reserves must be both None or both Some"
        );

        let log_rate = match (reserve_in, reserve_out) {
            (Some(reserve_in), Some(reserve_out)) => {
                let log_rate = Self::calculated_log_rate(reserve_in, reserve_out);
                Some(log_rate)
            }
            _ => None,
        };

        Ok(Self {
            id,
            token_in,
            token_out,
            reserve_in,
            reserve_out,
            log_rate,
        })
    }

    const fn assert_reserves(&self) {
        assert!(self.has_reserves(), "Swap must have reserves");
    }

    pub const fn log_rate(&self) -> i64 {
        self.assert_reserves();
        self.log_rate.unwrap()
    }

    pub const fn reserve_in(&self) -> U256 {
        self.assert_reserves();
        self.reserve_in.unwrap()
    }

    pub const fn reserve_out(&self) -> U256 {
        self.assert_reserves();
        self.reserve_out.unwrap()
    }

    pub const fn has_reserves(&self) -> bool {
        self.reserve_in.is_some() && self.reserve_out.is_some()
    }

    pub const fn has_no_reserves(&self) -> bool {
        self.reserve_in.is_none() || self.reserve_out.is_none()
    }

    /// Create a new swap side for the forward direction: token0 -> token1
    pub fn forward(pool: &Pool) -> Self {
        let token_in = pool.token0;
        let token_out = pool.token1;
        let reserve_in = pool.reserve0;
        let reserve_out = pool.reserve1;
        let swap_id = SwapId {
            pool_id: pool.id.clone(),
            direction: Direction::ZeroForOne,
        };
        Self::new(swap_id, token_in, token_out, reserve_in, reserve_out).unwrap()
    }

    /// Create a new swap side for the reverse direction: token1 -> token0
    pub fn reverse(pool: &Pool) -> Self {
        let token_in = pool.token1;
        let token_out = pool.token0;
        let reserve_in = pool.reserve1;
        let reserve_out = pool.reserve0;
        let swap_id = SwapId {
            pool_id: pool.id.clone(),
            direction: Direction::OneForZero,
        };
        Self::new(swap_id, token_in, token_out, reserve_in, reserve_out).unwrap()
    }

    /// Returns true if the swap side is the `OneForZero` direction
    pub fn is_one_for_zero(&self) -> bool {
        self.id.direction == Direction::OneForZero
    }

    /// Returns true if the swap side is the `ZeroForOne` direction
    pub fn is_zero_for_one(&self) -> bool {
        self.id.direction == Direction::ZeroForOne
    }

    /// Returns true if the swap side is the reciprocal of the other swap side,
    /// i.e. it has the same pool but opposite direction. This is used to avoid trivial (within the
    /// same pool) cycles that are not interesting.
    pub fn is_reciprocal(&self, other: &Self) -> bool {
        self.id.pool_id == other.id.pool_id && self.id.direction.is_opposite(&other.id.direction)
    }

    /// Estimated gas cost of the swap in WETH
    /// This is a rough estimate and should not be relied on really.
    /// This is based on 150k gas for our contract overhead.
    /// TODO: review, maybe use gas price oracle?
    pub const fn estimated_gas_cost_in_weth() -> f64 {
        0.0001
    }

    /// Calculate the log rate of a swap for faster computation
    /// We replace rate multiplication with log addition
    /// Takes into account the swap fee (default 0.3%)
    #[allow(clippy::cast_possible_truncation)]
    fn calculated_log_rate(reserve0: U256, reserve1: U256) -> i64 {
        const SCALE: f64 = 1_000_000.0;
        // Apply fee factor (0.997 for 0.3% fee)
        const FEE_FACTOR: f64 = 0.997;

        // Calculate log rate with fee adjustment
        ((reserve1.approx_log10() - reserve0.approx_log10() + FEE_FACTOR.log10()) * SCALE) as i64
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use alloy::primitives::U256;

    use crate::arb::pool::PoolId;
    use crate::arb::swap::{Direction, Swap, SwapId};
    use crate::arb::test_helpers::*;
    use crate::arb::token::TokenId;

    #[test]
    fn test_same_tokens() {
        let swap = Swap::new(
            SwapId {
                pool_id: PoolId::from(address_from_str("F1")),
                direction: Direction::ZeroForOne,
            },
            TokenId::from(address_from_str("A")),
            TokenId::from(address_from_str("A")),
            Some(U256::from(100)),
            Some(U256::from(200)),
        );
        assert_eq!(
            swap.err().unwrap().to_string(),
            "Swap token0 and token1 must be different"
        );
    }

    #[test]
    fn test_log_rate() {
        for (reserve_in, reserve_out, expected) in &[
            // reserve_in,      reserve_out,        expected
            (100, 100, -1_304),
            (100, 200, 299_725),
            (200, 100, -302_334),
        ] {
            let test_swap = swap("F1", "A", "B", *reserve_in, *reserve_out);
            assert_eq!(test_swap.log_rate, Some(*expected));
        }
    }

    #[test]
    fn test_equality_and_hash() {
        let swap1 = swap("F1", "A", "B", 100, 200);
        let swap2 = swap("F1", "A", "B", 120, 230);

        assert_eq!(swap1, swap1); // reflexive

        // Compute hash for swap1
        let mut hasher1 = DefaultHasher::new();
        swap1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        // Compute hash for swap2
        let mut hasher2 = DefaultHasher::new();
        swap2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash1); // hash is consistent
        assert_eq!(swap1, swap2); // reflexive even with different reserves
        assert_eq!(hash2, hash1); // hash is symmetric

        let swap3 = swap("F1", "B", "A", 100, 200);
        assert_ne!(swap1, swap3);

        // Compute hash for swap3
        let mut hasher3 = DefaultHasher::new();
        swap3.hash(&mut hasher3);
        let hash3 = hasher3.finish();

        assert_ne!(hash1, hash3); // hash is different for different directions
    }
}
