/// A swap side represents one of the two swaps sides in a pool: the `ZeroForOne` or `OneForZero`
/// Used to calculate its `log_rate` and an `amount_out` given an `amount_in`
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use alloy::primitives::U256;
use eyre::{bail, Error};

use super::pool::{Pool, PoolId};
use super::token::TokenId;

/// The direction of a swap in a liquidity pool.
///
/// In a standard liquidity pool with two tokens (token0 and token1),
/// a swap can go in either direction: from token0 to token1 or from token1 to token0.
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Direction {
    /// Swap from token0 to token1 in the pool
    ZeroForOne,
    /// Swap from token1 to token0 in the pool
    OneForZero,
}

impl Direction {
    /// Checks if this direction is the opposite of another direction.
    ///
    /// # Arguments
    ///
    /// * `other` - The other direction to compare with
    ///
    /// # Returns
    ///
    /// `true` if the directions are opposite, `false` otherwise
    #[must_use]
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
    /// The identifier of the liquidity pool where the swap occurs
    pub pool_id: PoolId,
    /// The direction of the swap (`ZeroForOne` or `OneForZero`)
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
    /// Unique identifier for this swap, containing pool ID and direction
    id: SwapId,
    /// The token being swapped in (source token)
    token_in: TokenId,
    /// The token being swapped out (destination token)
    token_out: TokenId,
    /// The available reserve of the input token in the pool
    reserve_in: Option<U256>,
    /// The available reserve of the output token in the pool
    reserve_out: Option<U256>,
    /// The logarithmic exchange rate for this swap
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
    /// Creates a new swap with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for this swap
    /// * `token_in` - The token being swapped in
    /// * `token_out` - The token being swapped out
    /// * `reserve_in` - The available reserve of the input token
    /// * `reserve_out` - The available reserve of the output token
    ///
    /// # Returns
    ///
    /// A new Swap instance if successful
    ///
    /// # Errors
    ///
    /// Returns an error if the input and output tokens are the same,
    /// or if the reserves are inconsistent (one is Some and the other is None)
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

        if !(reserve_in.is_none() && reserve_out.is_none()
            || reserve_in.is_some() && reserve_out.is_some())
        {
            bail!("Reserves must be both None or both Some");
        }

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

    /// Returns the unique identifier for this swap.
    ///
    /// # Returns
    ///
    /// The unique identifier for this swap
    #[must_use]
    pub fn id(&self) -> SwapId {
        self.id.clone()
    }

    /// Returns the token being swapped in.
    ///
    /// # Returns
    ///
    /// The token being swapped in
    #[must_use]
    pub const fn token_in(&self) -> TokenId {
        self.token_in
    }

    /// Returns the token being swapped out.
    ///
    /// # Returns
    ///
    /// The token being swapped out
    #[must_use]
    pub const fn token_out(&self) -> TokenId {
        self.token_out
    }

    /// Returns the logarithmic exchange rate for this swap.
    ///
    /// # Returns
    ///
    /// The logarithmic exchange rate as an i64
    ///
    /// # Panics
    ///
    /// Panics if the `log_rate` is None, which occurs when the swap doesn't have reserves
    #[must_use]
    pub const fn log_rate(&self) -> i64 {
        // TODO: use typestates to ensure this is never called on a swap without reserves
        #[allow(clippy::unwrap_used)]
        self.log_rate.unwrap()
    }

    /// Returns the reserve of the input token.
    ///
    /// # Returns
    ///
    /// The reserve of the input token as a U256
    ///
    /// # Panics
    ///
    /// Panics if the `reserve_in` is None, which occurs when the swap doesn't have reserves
    #[must_use]
    pub const fn reserve_in(&self) -> U256 {
        // TODO: use typestates to ensure this is never called on a swap without reserves
        #[allow(clippy::unwrap_used)]
        self.reserve_in.unwrap()
    }

    /// Returns the reserve of the output token.
    ///
    /// # Returns
    ///
    /// The reserve of the output token as a U256
    ///
    /// # Panics
    ///
    /// Panics if the `reserve_out` is None, which occurs when the swap doesn't have reserves
    #[must_use]
    pub const fn reserve_out(&self) -> U256 {
        // TODO: use typestates to ensure this is never called on a swap without reserves
        #[allow(clippy::unwrap_used)]
        self.reserve_out.unwrap()
    }

    /// Checks if this swap has reserves for both input and output tokens.
    ///
    /// # Returns
    ///
    /// `true` if both reserves are present, `false` otherwise
    #[must_use]
    pub const fn has_reserves(&self) -> bool {
        self.reserve_in.is_some() && self.reserve_out.is_some()
    }

    /// Checks if this swap is missing reserves for either input or output tokens.
    ///
    /// # Returns
    ///
    /// `true` if either reserve is missing, `false` if both are present
    #[must_use]
    pub const fn has_no_reserves(&self) -> bool {
        self.reserve_in.is_none() || self.reserve_out.is_none()
    }

    /// Create a new swap side for the forward direction: token0 -> token1
    ///
    /// # Panics
    ///
    /// Panics if the created swap is invalid, which should never happen when creating from a valid pool
    #[must_use]
    pub fn forward(pool: &Pool) -> Self {
        let token_in = pool.token0;
        let token_out = pool.token1;
        let reserve_in = pool.reserve0;
        let reserve_out = pool.reserve1;
        let swap_id = SwapId {
            pool_id: pool.id.clone(),
            direction: Direction::ZeroForOne,
        };
        // SAFETY: we know the is valid because we are creating it from a pool which is valid
        #[allow(clippy::unwrap_used)]
        Self::new(swap_id, token_in, token_out, reserve_in, reserve_out).unwrap()
    }

    /// Create a new swap side for the reverse direction: token1 -> token0
    ///
    /// # Panics
    ///
    /// Panics if the created swap is invalid, which should never happen when creating from a valid pool
    #[must_use]
    pub fn reverse(pool: &Pool) -> Self {
        let token_in = pool.token1;
        let token_out = pool.token0;
        let reserve_in = pool.reserve1;
        let reserve_out = pool.reserve0;
        let swap_id = SwapId {
            pool_id: pool.id.clone(),
            direction: Direction::OneForZero,
        };
        // SAFETY: we know the is valid because we are creating it from a pool which is valid
        #[allow(clippy::unwrap_used)]
        Self::new(swap_id, token_in, token_out, reserve_in, reserve_out).unwrap()
    }

    /// Returns true if the swap side is the reciprocal of the other swap side,
    /// i.e. it has the same pool but opposite direction. This is used to avoid trivial (within the
    /// same pool) cycles that are not interesting.
    #[must_use]
    pub fn is_reciprocal(&self, other: &Self) -> bool {
        self.id.pool_id == other.id.pool_id && self.id.direction.is_opposite(&other.id.direction)
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
#[allow(clippy::unwrap_used)]
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
