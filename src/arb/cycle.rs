/// Cycle is a Vec<Swap> that forms a cycle (first and last token are the same)
/// It is primarily used to calculate its profitability exploitability, best amount in, etc.
use std::{
    cmp::min,
    fmt::Debug,
    hash::{Hash, Hasher},
};

use alloy::primitives::U256;
use eyre::{bail, Result};
use log::{debug, error};

use crate::utils::signer::Order;

use super::swap_quote::SwapQuote;
use super::swap_side::SwapSide;

/// A cycle of swaps that starts and ends at the same token
#[derive(Clone)]
#[allow(dead_code)]
pub struct Cycle {
    /// Sequence of swap ids forming the cycle
    pub swap_sides: Vec<SwapSide>,

    /// The swap rate of the cycle (a product of all swap rates in the cycle)
    pub log_rate: i64,

    /// The orders to be sent to the signer
    pub orders: Option<Vec<Order>>,

    pub best_swap_quotes: Option<Vec<SwapQuote>>,

    /// The optimal amount of tokens to input into the cycle to maximize profit
    pub best_amount_in: Option<U256>,

    /// Maximum profit that can be made from the cycle
    pub max_profit: Option<U256>,

    /// Maximum profit margin
    pub max_profit_margin: Option<f64>,
}

impl Debug for Cycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cycle({})",
            self.swap_sides
                .iter()
                .map(|s| format!("{s:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
impl PartialEq for Cycle {
    fn eq(&self, other: &Self) -> bool {
        self.swap_sides == other.swap_sides
    }
}

impl Eq for Cycle {}

impl Hash for Cycle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for swap in &self.swap_sides {
            swap.hash(state);
        }
    }
}

impl Cycle {
    #[allow(dead_code)]
    pub fn new(swaps: Vec<SwapSide>) -> Result<Self> {
        let log_rate = swaps.iter().map(|swap| swap.log_rate).sum();
        let cycle = Self {
            swap_sides: swaps,
            log_rate,
            orders: None,
            best_swap_quotes: None,
            best_amount_in: None,
            max_profit: None,
            max_profit_margin: None,
        };
        cycle.validate_swaps()?;
        Ok(cycle)
    }

    /// The optimal `amount_in` to get the maximum `amount_out`
    /// This is using binary search to find the maximum `amount_out`
    #[allow(dead_code)]
    pub fn optimize(&mut self, our_balance: U256) {
        if !self.is_profitable() {
            debug!("Cycle is not profitable");
            return;
        }

        // Higher precision means more iterations
        let precision = U256::from(1000);

        // Increment in derivative calculation. Too small of a delta can cause
        // the binary search to take into an infinite loop (f(x+dx) - f(x) = 0)
        let delta = U256::from(1000);

        // This should really be gas cost, but not worth optimizing
        let mut amount_in_left = U256::from(0);

        let mut amount_in_right = min(self.swap_sides[0].reserve0, our_balance);

        let mut best_amount_in = U256::ZERO;
        let mut best_profit = U256::ZERO;
        let mut best_swap_quotes = None;

        let mut count = 0;
        // Arbitrary limit to prevent infinite loop
        let max_count = 100;
        while amount_in_right - amount_in_left > precision {
            count += 1;
            if count > max_count {
                error!(
                    "Cycle optimization failed to converge after {} iterations",
                    count
                );
                return;
            }
            let amount_in = min(
                (amount_in_left + amount_in_right) / U256::from(2),
                our_balance - delta,
            );
            let amount_in_delta = amount_in + delta;

            let quotes = self.quotes(amount_in);
            let amount_out = quotes.last().unwrap().amount_out;
            let quotes_delta = self.quotes(amount_in_delta);
            let amount_out_delta = quotes_delta.last().unwrap().amount_out;

            let profit = amount_out.saturating_sub(amount_in);
            let profit_delta = amount_out_delta.saturating_sub(amount_in_delta);

            if profit_delta > profit {
                // Rising profit curve
                amount_in_left = amount_in;
            } else {
                // Falling profit curve
                amount_in_right = amount_in;
            }

            // Track best profit seen
            if profit > best_profit {
                best_profit = profit;
                best_amount_in = amount_in;
                best_swap_quotes = Some(quotes);
            }

            if profit_delta > best_profit {
                best_profit = profit_delta;
                best_amount_in = amount_in_delta;
                best_swap_quotes = Some(quotes_delta);
            }
        }

        // May need to use a different precision for the best amount in.
        // Something like the equivalent of $0.01.
        if best_amount_in > U256::ZERO {
            self.best_amount_in = Some(best_amount_in);
            self.max_profit = Some(best_profit);
            self.max_profit_margin =
                Some(f64::from(best_profit) * 100.0 / f64::from(best_amount_in) / 100.0);
            self.best_swap_quotes = best_swap_quotes;
        } else {
            debug!("Cycle has no profitable amount in");
        }
    }

    fn validate_swaps(&self) -> Result<()> {
        if self.swap_sides.len() < 2 {
            bail!("Cycle must have at least 2 swaps");
        }

        for i in 0..self.swap_sides.len() {
            // Check for duplicates
            if self.swap_sides[i] == self.swap_sides[(i + 1) % self.swap_sides.len()] {
                bail!("Cycle contains duplicate swaps");
            }

            // Check token matching
            let next = (i + 1) % self.swap_sides.len();
            if self.swap_sides[i].token1 != self.swap_sides[next].token0 {
                bail!(
                    "Swap {} token1 ({}) does not match swap {} token0 ({})",
                    i,
                    self.swap_sides[i].token1,
                    next,
                    self.swap_sides[next].token0
                );
            }
        }
        Ok(())
    }

    /// Whether the cycle is profitable
    /// This is based merely on pool price. Gas and slippage are not considered.
    #[allow(dead_code)]
    pub const fn is_profitable(&self) -> bool {
        self.log_rate.is_positive()
    }

    /// Whether the cycle is exploitable
    /// This is based merely on pool price. Gas and slippage are not considered.
    #[allow(dead_code)]
    pub const fn is_exploitable(&self) -> bool {
        self.best_amount_in.is_some()
    }

    /// Returns a Vec of amounts out for each swap in the cycle, including the final amount
    /// The first element is the input amount, and each subsequent element is the output
    /// amount from that swap
    #[allow(dead_code)]
    fn quotes(&self, amount_in: U256) -> Vec<SwapQuote> {
        let mut swap_quotes = Vec::with_capacity(self.swap_sides.len() + 1);

        self.swap_sides.iter().fold(amount_in, |amount, swap_side| {
            let swap_quote = SwapQuote::new(swap_side, amount);
            swap_quotes.push(swap_quote.clone());
            swap_quote.amount_out
        });

        swap_quotes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arb::swap_side::Direction;
    use crate::arb::test_helpers::*;

    #[test]
    fn test_new_invalid_length() {
        let swaps = vec![swap("P1", Direction::ZeroForOne, "A", "B", 100, 200)];
        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Cycle must have at least 2 swaps"
        );
    }

    #[test]
    fn test_new_invalid_duplicate_swaps() {
        let swaps = vec![
            swap("P1", Direction::ZeroForOne, "A", "B", 100, 200),
            swap("P1", Direction::ZeroForOne, "A", "B", 200, 100),
        ];
        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Cycle contains duplicate swaps"
        );
    }

    #[test]
    fn test_new_invalid_token_mismatch() {
        let swaps = vec![
            swap("P1", Direction::ZeroForOne, "A", "B", 100, 200),
            swap("P1", Direction::ZeroForOne, "C", "B", 200, 100),
        ];
        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Swap 0 token1 (B) does not match swap 1 token0 (C)"
        );
    }
    #[test]

    fn test_log_rate() {
        let swap1 = swap("P1", Direction::ZeroForOne, "A", "B", 100, 200);
        assert_eq!(swap1.log_rate, 301_029);
        let swap2 = swap("P2", Direction::ZeroForOne, "B", "A", 300, 100);
        assert_eq!(swap2.log_rate, -477_121);

        let cycle = Cycle::new(vec![swap1, swap2]).unwrap();
        assert_eq!(cycle.log_rate, 301_029 - 477_121);
    }

    #[test]
    fn test_amount_out_not_exploitable() {
        let cycle = cycle(&[
            ("P1", Direction::ZeroForOne, "A", "B", 100, 200), // 2 rate
            ("P2", Direction::ZeroForOne, "B", "A", 300, 100), // 1/3 rate
        ]);
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
            let quotes = cycle.quotes(U256::from(*amount_in));
            assert_eq!(quotes.len(), 2);
            assert_eq!(quotes[0].amount_in, U256::from(*amount_in));
            assert_eq!(quotes[0].amount_out, U256::from(*intermediate_amount_out));
            assert_eq!(quotes[1].amount_in, U256::from(*intermediate_amount_out));
            assert_eq!(quotes[1].amount_out, U256::from(*final_amount_out));
        }
    }

    #[test]
    fn test_amount_out_exploitable() {
        let cycle = cycle(&[
            ("P1", Direction::ZeroForOne, "A", "B", 100, 200), // 2 rate
            ("P2", Direction::ZeroForOne, "B", "A", 300, 300), // 1 rate
        ]);

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
            let quotes = cycle.quotes(U256::from(*amount_in));
            assert_eq!(quotes.len(), 2);
            assert_eq!(quotes[0].amount_in, U256::from(*amount_in));
            assert_eq!(quotes[0].amount_out, U256::from(*intermediate_amount_out));
            assert_eq!(quotes[1].amount_in, U256::from(*intermediate_amount_out));
            assert_eq!(quotes[1].amount_out, U256::from(*final_amount_out));
        }
    }

    #[test]
    fn test_optimize_not_exploitable() {
        let mut cycle = cycle(&[
            ("P1", Direction::ZeroForOne, "A", "B", 100, 200), // 2 rate
            ("P2", Direction::ZeroForOne, "B", "A", 300, 100), // 1/3 rate
        ]);
        let our_balance = U256::from(100);
        cycle.optimize(our_balance);

        assert_eq!(cycle.best_amount_in, None);
        assert_eq!(cycle.max_profit, None);
        assert!(cycle.best_swap_quotes.is_none());
    }

    #[test]
    fn test_optimize_exploitable() {
        let mut cycle = cycle(&[
            ("P1", Direction::ZeroForOne, "A", "B", 1_000_000, 2_000_000), // 2 rate
            ("P2", Direction::ZeroForOne, "B", "A", 3_000_000, 3_000_000), // 1 rate
        ]);

        for (balance, amount_in, mid_amount, amount_out, profit) in &[
            (50_000, 50_000, 94_965, 91_783, 41_783),
            (100_000, 100_000, 181_322, 170_503, 70_503),
            (200_000, 200_000, 332_499, 298_515, 98_515),
            (300_000, 247_093, 395_316, 348_363, 101_270),
            (400_000, 246_875, 395_036, 348_145, 101_270),
            (500_000, 247_093, 395_316, 348_363, 101_270),
            (600_000, 247_093, 395_316, 348_363, 101_270),
            // 247_093 is all we need in this case
        ] {
            assert!(amount_in <= balance);
            cycle.optimize(U256::from(*balance));

            assert!(cycle.clone().best_swap_quotes.is_some());
            let quotes = cycle.clone().best_swap_quotes.unwrap();
            assert_eq!(quotes.len(), 2);

            assert_eq!(quotes[0].amount_in, U256::from(*amount_in));
            assert_eq!(quotes[0].amount_out, U256::from(*mid_amount));
            assert_eq!(quotes[1].amount_in, U256::from(*mid_amount));
            assert_eq!(quotes[1].amount_out, U256::from(*amount_out));

            assert_eq!(
                cycle.best_amount_in,
                Some(U256::from(*amount_in))
            );
            assert_eq!(cycle.clone().max_profit, Some(U256::from(*profit)));
        }
    }

    #[test]
    fn test_optimize_with_wild_exchange_rate() {
        let mut cycle = cycle(&[
            ("P1", Direction::ZeroForOne, "A", "B", 1_000_000, 2_000_000_000_000_000_000), // 2 rate
            ("P2", Direction::ZeroForOne, "B", "A", 2_000_000_000_000_000_000, 2_000_000), // 1 rate
        ]);
        let our_balance = U256::from(100_000);
        cycle.optimize(our_balance);

        assert!(cycle.best_swap_quotes.is_some());
        let quotes = cycle.best_swap_quotes.unwrap();
        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[0].amount_in, U256::from(100_000));
        assert_eq!(quotes[0].amount_out, U256::from(181_322_178_776_029_826_u64));
        assert_eq!(quotes[1].amount_in, U256::from(181_322_178_776_029_826_u64));
        assert_eq!(quotes[1].amount_out, U256::from(165_792));

        assert_eq!(cycle.best_amount_in, Some(U256::from(100_000)));
        assert_eq!(cycle.max_profit, Some(U256::from(65_792)));
        assert_eq!(cycle.max_profit_margin, Some(0.657_920_000_000_000_1));
    }
}