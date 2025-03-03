use alloy::primitives::U256;

use super::swap::Swap;

/// A quote for a swap: the amount of tokens we get out of the swap given an amount of tokens we put in.
///
/// This is simply the implementation of the Uniswap v2 formula. This is returned by the `Cycle`
/// optimizer. We need complete quotes for each swap in a cycle (both amount in and amount out).
#[derive(Debug, Clone)]
pub struct SwapQuote {
    amount_in: U256,
    amount_out: U256,
}

impl SwapQuote {
    /// Creates a new swap quote for the given swap and amount in.
    ///
    /// # Panics
    ///
    /// Panics if the swap does not have reserves.
    pub fn new(swap: &Swap, amount_in: U256) -> Self {
        assert!(
            swap.has_reserves(),
            "Swap must have reserves to calculate amount out"
        );
        let amount_out = Self::calculated_amount_out(swap, amount_in);

        Self {
            amount_in,
            amount_out,
        }
    }

    /// f64 is a lot, also this function is used in logs only
    #[allow(clippy::cast_precision_loss)]
    pub fn rate(&self) -> f64 {
        let amount_out_f64 = self.amount_out.as_limbs()[0] as f64;
        let amount_in_f64 = self.amount_in.as_limbs()[0] as f64;
        amount_out_f64 / amount_in_f64
    }

    pub const fn amount_in(&self) -> U256 {
        self.amount_in
    }

    pub const fn amount_out(&self) -> U256 {
        self.amount_out
    }

    /// The amount of tokens we get out of the swap given an amount of tokens we put in
    /// Uses the rate which already includes the fee calculation
    #[allow(clippy::cast_precision_loss)]
    fn calculated_amount_out(swap: &Swap, amount_in: U256) -> U256 {
        assert!(
            swap.has_reserves(),
            "Swap must have reserves to calculate amount out"
        );

        let fee_numerator = U256::from(997);
        let fee_denominator = U256::from(1000);

        let amount_in_with_fee = amount_in * fee_numerator;
        let numerator = amount_in_with_fee * swap.reserve_out();
        let denominator = (swap.reserve_in() * fee_denominator) + amount_in_with_fee;

        numerator / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arb::test_helpers::*;

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
            let swap_quote = swap_quote("F1", "A", "B", *reserve0, *reserve1, *amount_in);
            assert_eq!(swap_quote.amount_out(), U256::from(*expected));
        }
    }
}
