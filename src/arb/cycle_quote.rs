use alloy::primitives::{I256, U256};

use crate::arb::cycle::Cycle;
use crate::arb::swap_quote::SwapQuote;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CycleQuote {
    /// The quotes for each swap in the cycle
    swap_quotes: Vec<SwapQuote>,
}

impl CycleQuote {
    pub fn new(cycle: &Cycle, amount_in: U256) -> Self {
        let mut swap_quotes = Vec::with_capacity(cycle.swaps.len() + 1);
        cycle.swaps.iter().fold(amount_in, |amount, swap_side| {
            let swap_quote = SwapQuote::new(swap_side, amount);
            swap_quotes.push(swap_quote.clone());
            swap_quote.amount_out()
        });

        Self { swap_quotes }
    }

    pub fn swap_quotes(&self) -> Vec<SwapQuote> {
        self.swap_quotes.clone()
    }

    /// Profit for this cycle quote (given `amount_in`)
    pub fn profit(&self) -> I256 {
        I256::from_raw(self.amount_out()).saturating_sub(I256::from_raw(self.amount_in()))
    }

    /// Profit margin for this cycle quote (given `amount_in`) in basis points (10,000 = 100%)
    #[allow(clippy::cast_possible_truncation)]
    pub fn profit_margin(&self) -> i32 {
        let profit = self.profit();
        let amount_in = self.amount_in();

        if amount_in.is_zero() {
            0
        } else {
            // Calculate (profit * 10_000) / amount_in to get basis points
            let scaled_profit = profit.abs().into_raw().saturating_mul(U256::from(10_000));
            let margin = scaled_profit / amount_in;

            // Convert to i32, capping at i32::MAX if necessary
            let result = if margin > U256::from(i32::MAX as u64) {
                i32::MAX
            } else {
                // SAFETY: we know the margin is less than i32::MAX
                margin.as_limbs()[0] as i32
            };

            // Apply sign based on whether profit is positive or negative
            if profit.is_negative() {
                -result
            } else {
                result
            }
        }
    }

    /// Whether this cycle quote is exploitable (has a positive profit)
    pub fn is_profitable(&self) -> bool {
        self.profit().is_positive()
    }

    pub fn amount_in(&self) -> U256 {
        self.swap_quotes.first().unwrap().amount_in()
    }

    pub fn amount_out(&self) -> U256 {
        self.swap_quotes.last().unwrap().amount_out()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arb::test_helpers::*;

    #[test]
    fn test_quotes_not_exploitable() {
        let cycle = cycle(&[
            ("F1", "A", "B", 100, 200), // 2 rate
            ("F2", "B", "A", 300, 100), // 1/3 rate
        ])
        .unwrap();

        for (amount_in, intermediate_amount_out, final_amount_out) in &[
            //in0, out0/in1, out1, loss
            (10, 18, 5),  // -5
            (20, 33, 9),  // -11
            (30, 46, 13), // -17
            (40, 57, 15), // -25
            (50, 66, 17), // -33
            (60, 74, 19), // -41
            (70, 82, 21), // -49
        ] {
            let cycle_quote = CycleQuote::new(&cycle, U256::from(*amount_in));
            assert_eq!(cycle_quote.swap_quotes.len(), 2);
            assert_eq!(cycle_quote.amount_in(), U256::from(*amount_in));
            assert_eq!(
                cycle_quote.swap_quotes[0].amount_out(),
                U256::from(*intermediate_amount_out)
            );
            assert_eq!(
                cycle_quote.swap_quotes[1].amount_in(),
                U256::from(*intermediate_amount_out)
            );
            assert_eq!(
                cycle_quote.swap_quotes[1].amount_out(),
                U256::from(*final_amount_out)
            );
        }
    }

    #[test]
    fn test_quotes_exploitable() {
        let cycle = cycle(&[
            ("F1", "A", "B", 100, 200), // 2 rate
            ("F2", "B", "A", 300, 300), // 1 rate
        ])
        .unwrap();

        for (amount_in, intermediate_amount_out, final_amount_out) in &[
            //in0, out0/in1, out1, profit
            (10, 18, 16), // +6
            (20, 33, 29), // +9 \
            (25, 39, 34), // +9 . best amount in is here
            (30, 46, 39), // +9 /
            (40, 57, 47), // +7
            (50, 66, 53), // +3
            (60, 74, 59), // -1
            (70, 82, 64), // +6
        ] {
            let cycle_quote = CycleQuote::new(&cycle, U256::from(*amount_in));
            assert_eq!(cycle_quote.swap_quotes.len(), 2);
            assert_eq!(cycle_quote.amount_in(), U256::from(*amount_in));
            assert_eq!(
                cycle_quote.swap_quotes[0].amount_out(),
                U256::from(*intermediate_amount_out)
            );
            assert_eq!(
                cycle_quote.swap_quotes[1].amount_in(),
                U256::from(*intermediate_amount_out)
            );
            assert_eq!(
                cycle_quote.swap_quotes[1].amount_out(),
                U256::from(*final_amount_out)
            );
        }
    }
}
