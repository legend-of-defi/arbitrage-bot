use alloy::primitives::U256;

use super::swap_side::SwapSide;

/// A quote for a swap: the amount of tokens we get out of the swap given an amount of tokens we put in
///
#[derive(Debug, Clone)]
pub struct SwapQuote {
    pub swap_side: SwapSide,
    pub amount_in: U256,
    pub amount_out: U256,
}

impl SwapQuote {
    pub fn new(swap: &SwapSide, amount_in: U256) -> Self {
        let reserve0 = swap.reserve0;
        let reserve1 = swap.reserve1;
        let amount_out = Self::amount_out(reserve0, reserve1, amount_in);

        Self {
            swap_side: swap.clone(),
            amount_in,
            amount_out,
        }
    }

    /// The amount of tokens we get out of the swap given an amount of tokens we put in
    pub fn amount_out(reserve0: U256, reserve1: U256, amount_in: U256) -> U256 {
        let fee_numerator = U256::from(997);
        let fee_denominator = U256::from(1000);

        let amount_in_with_fee = amount_in * fee_numerator;
        let numerator = amount_in_with_fee * reserve1;
        let denominator = (reserve0 * fee_denominator) + amount_in_with_fee;

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
            let swap_quote = swap_quote("P1", "A", "B", *reserve0, *reserve1, *amount_in);
            assert_eq!(swap_quote.amount_out, U256::from(*expected));
        }
    }
}
