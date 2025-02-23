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

use super::swap::Swap;

/// A cycle of swaps that starts and ends at the same token
#[derive(Clone)]
#[allow(dead_code)]
pub struct Cycle {
    /// Sequence of swap ids forming the cycle
    pub swaps: Vec<Swap>,

    /// The swap rate of the cycle (a product of all swap rates in the cycle)
    pub log_rate: i64,

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
            self.swaps
                .iter()
                .map(|s| format!("{s:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
impl PartialEq for Cycle {
    fn eq(&self, other: &Self) -> bool {
        self.swaps == other.swaps
    }
}

impl Eq for Cycle {}

impl Hash for Cycle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for swap in &self.swaps {
            swap.hash(state);
        }
    }
}

impl Cycle {
    #[allow(dead_code)]
    pub fn new(swaps: Vec<Swap>) -> Result<Self> {
        let log_rate = swaps.iter().map(|swap| swap.log_rate).sum();
        let cycle = Self {
            swaps,
            log_rate,
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

        let mut amount_in_right = min(self.swaps[0].reserve0, our_balance);

        let mut best_amount_in = U256::ZERO;
        let mut best_profit = U256::ZERO;

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

            let amount_out = self.amount_out(amount_in);
            let amount_out_delta = self.amount_out(amount_in_delta);

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
            }

            if profit_delta > best_profit {
                best_profit = profit_delta;
                best_amount_in = amount_in_delta;
            }
        }

        // May need to use a different precision for the best amount in.
        // Something like the equivalent of $0.01.
        if best_amount_in > U256::ZERO {
            self.best_amount_in = Some(best_amount_in);
            self.max_profit = Some(best_profit);
            self.max_profit_margin =
                Some(f64::from(best_profit) * 100.0 / f64::from(best_amount_in) / 100.0);
        } else {
            debug!("Cycle has no profitable amount in");
        }
    }

    fn validate_swaps(&self) -> Result<()> {
        if self.swaps.len() < 2 {
            bail!("Cycle must have at least 2 swaps");
        }

        for i in 0..self.swaps.len() {
            // Check for duplicates
            if self.swaps[i] == self.swaps[(i + 1) % self.swaps.len()] {
                bail!("Cycle contains duplicate swaps");
            }

            // Check token matching
            let next = (i + 1) % self.swaps.len();
            if self.swaps[i].token1 != self.swaps[next].token0 {
                bail!(
                    "Swap {} token1 ({}) does not match swap {} token0 ({})",
                    i,
                    self.swaps[i].token1,
                    next,
                    self.swaps[next].token0
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

    /// The `amount_out` we get from this cycle when we start with `amount_in`
    #[allow(dead_code)]
    fn amount_out(&self, amount_in: U256) -> U256 {
        self.swaps
            .iter()
            .fold(amount_in, |amount, swap| swap.amount_out(amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arb::test_helpers::*;

    #[test]
    fn test_new_invalid_length() {
        let swaps = vec![swap("P1", "A", "B", 100, 200)];
        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Cycle must have at least 2 swaps"
        );
    }

    #[test]
    fn test_new_invalid_duplicate_swaps() {
        let swaps = vec![
            swap("P1", "A", "B", 100, 200),
            swap("P1", "A", "B", 200, 100),
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
            swap("P1", "A", "B", 100, 200),
            swap("P1", "C", "B", 200, 100),
        ];
        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Swap 0 token1 (B) does not match swap 1 token0 (C)"
        );
    }
    #[test]

    fn test_log_rate() {
        let swap1 = swap("P1", "A", "B", 100, 200);
        assert_eq!(swap1.log_rate, 301_029);
        let swap2 = swap("P2", "B", "A", 300, 100);
        assert_eq!(swap2.log_rate, -477_121);

        let cycle = Cycle::new(vec![swap1, swap2]).unwrap();
        assert_eq!(cycle.log_rate, 301_029 - 477_121);
    }

    #[test]
    fn test_amount_out_not_exploitable() {
        let cycle = cycle(&[
            ("P1", "A", "B", 100, 200), // 2 rate
            ("P2", "B", "A", 300, 100), // 1/3 rate
        ]);
        for (amount_in, expected_amount_out) in &[
            //in, out, loss
            (10, 5),  // -5
            (20, 9),  // -11
            (30, 13), // -17
            (40, 15), // -25
            (50, 17), // -33
            (60, 19), // -41
            (70, 21), // -49
        ] {
            assert_eq!(
                cycle.amount_out(U256::from(*amount_in)),
                U256::from(*expected_amount_out)
            );
        }
    }

    #[test]
    fn test_amount_out_exploitable() {
        let cycle = cycle(&[
            ("P1", "A", "B", 100, 200), // 2 rate
            ("P2", "B", "A", 300, 300), // 1 rate
        ]);

        for (amount_in, expected_amount_out) in &[
            //in, out,  profit
            (10, 16), // +6
            (20, 29), // +9 \
            (25, 34), // +9 . best amount in is here
            (30, 39), // +9 /
            (40, 47), // +7
            (50, 53), // +3
            (60, 59), // -1
            (70, 64), // +6
        ] {
            assert_eq!(
                cycle.amount_out(U256::from(*amount_in)),
                U256::from(*expected_amount_out)
            );
        }
    }

    #[test]
    fn test_optimize_not_exploitable() {
        let mut cycle = cycle(&[
            ("P1", "A", "B", 100, 200), // 2 rate
            ("P2", "B", "A", 300, 100), // 1/3 rate
        ]);
        let our_balance = U256::from(100);
        cycle.optimize(our_balance);

        assert_eq!(cycle.best_amount_in, None);
        assert_eq!(cycle.max_profit, None);
    }

    #[test]
    fn test_optimize_exploitable() {
        let mut cycle = cycle(&[
            ("P1", "A", "B", 1_000_000, 2_000_000), // 2 rate
            ("P2", "B", "A", 3_000_000, 3_000_000), // 1 rate
        ]);

        for (our_balance, expected_optimal_amount_in, expected_profit) in &[
            // our balance, optimal amount in, profit
            (50_000, 50_000, 41_783),
            (100_000, 100_000, 70_503),
            (200_000, 200_000, 98_515),
            (300_000, 247_093, 101_270),
            (400_000, 246_875, 101_270),
            (500_000, 247_093, 101_270),
            (600_000, 247_093, 101_270),
            // somewhere between 247_093 is all we need in this case
        ] {
            dbg!(our_balance, expected_optimal_amount_in, expected_profit);
            assert!(expected_optimal_amount_in <= our_balance);
            cycle.optimize(U256::from(*our_balance));

            assert_eq!(
                cycle.best_amount_in,
                Some(U256::from(*expected_optimal_amount_in))
            );
            assert_eq!(cycle.max_profit, Some(U256::from(*expected_profit)));
        }
    }

    #[test]
    fn test_optimize_with_wild_exchange_rate() {
        let mut cycle = cycle(&[
            ("P1", "A", "B", 1_000_000, 2_000_000_000_000_000_000), // 2 rate
            ("P2", "B", "A", 2_000_000_000_000_000_000, 2_000_000), // 1 rate
        ]);
        let our_balance = U256::from(100_000);
        cycle.optimize(our_balance);

        assert_eq!(cycle.best_amount_in, Some(U256::from(100_000)));
        assert_eq!(cycle.max_profit, Some(U256::from(65_792)));
        assert_eq!(cycle.max_profit_margin, Some(0.657_920_000_000_000_1));
    }

    #[test]
    fn test_slippage_vs_size() {
        // Set up pools with equal reserves
        let reserve_size = 1_000_000;
        let base_amount = 1_000;

        println!("\nSlippage analysis:");
        println!("Swap size (% of reserves) | Slippage %");
        println!("-----------------------------------------");

        for multiplier in [1, 5, 10, 20, 30, 40, 50] {
            let amount_in = base_amount * multiplier;
            let percent_of_reserves = (amount_in as f64 / reserve_size as f64) * 100.0;

            let cycle = cycle(&[
                ("P1", "A", "B", reserve_size, reserve_size),
                ("P2", "B", "A", reserve_size, reserve_size),
            ]);

            // Calculate expected amount without slippage (using spot price)
            let spot_price = 1.0; // Since reserves are equal
            let expected_without_slippage = amount_in as f64 * spot_price;

            // Get actual amount out
            let actual_amount_out = cycle.amount_out(U256::from(amount_in));

            // Calculate slippage percentage
            let slippage_percent = ((expected_without_slippage
                - actual_amount_out.to::<u64>() as f64)
                / expected_without_slippage)
                * 100.0;

            println!(
                "{percent_of_reserves:>20.2}% | {slippage_percent:>10.2}%"
            );
        }
    }
}
