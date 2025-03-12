use std::collections::HashMap;
/// Cycle is a Vec<Swap> that forms a cycle (first and last token are the same)
/// It is primarily used to calculate its profitability exploitability, best amounts in, etc.
use std::{
    cell::RefCell,
    fmt::Debug,
    hash::{Hash, Hasher},
};

use alloy::primitives::U256;
use eyre::{bail, Error, Result};
use itertools::Itertools;
use log::error;

use super::cycle_quote::CycleQuote;
use super::swap::Swap;

/// A cycle of swaps that starts and ends at the same token
#[derive(Clone)]
pub struct Cycle {
    /// Sequence of swap sides forming the cycle
    pub swaps: Vec<Swap>,

    /// Cached best quote for this cycle
    /// This is future functionality.
    #[allow(dead_code)]
    best_quote: RefCell<Option<CycleQuote>>,
}

impl PartialOrd for Cycle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cycle {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // SAFETY: we know the cycle has at least one swap because this is enforced by the
        // constructor
        #[allow(clippy::unwrap_used)]
        self.swaps
            .first()
            .unwrap()
            .cmp(other.swaps.first().unwrap())
    }
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
        if self.swaps.len() != other.swaps.len() {
            return false;
        }
        for i in 0..self.swaps.len() {
            if self.swaps[i] != other.swaps[i] {
                return false;
            }
        }
        true
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
    /// Creates a new cycle from a vector of swap sides
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cycle has fewer than 2 swaps
    /// - The cycle contains duplicate swaps
    /// - The tokens don't match between consecutive swaps
    pub fn new(mut swaps: Vec<Swap>) -> Result<Self> {
        Self::validate_swaps(&swaps)?;
        Self::normalize_swaps(&mut swaps);
        let cycle = Self {
            swaps,
            best_quote: RefCell::new(None),
        };
        Ok(cycle)
    }

    /// Normalizes the swaps by rotating them so the smallest swap is first
    /// This is used for equality comparison and hashing
    fn normalize_swaps(swaps: &mut [Swap]) {
        if swaps.is_empty() {
            return;
        }

        // Find index of smallest swap
        let mut min_idx = 0;
        for i in 1..swaps.len() {
            if swaps[i] < swaps[min_idx] {
                min_idx = i;
            }
        }

        // No need to rotate if smallest is already first
        if min_idx == 0 {
            return;
        }

        // Rotate swaps so smallest is first
        swaps.rotate_left(min_idx);
    }

    /// The cycle is quotable if all swaps have reserves
    /// This is future functionality.
    #[allow(dead_code)]
    pub fn has_all_reserves(&self) -> bool {
        self.swaps.iter().all(super::swap::Swap::has_reserves)
    }

    /// Returns a list of swaps in the cycle that have no reserves.
    ///
    /// This method is useful for identifying which swaps in the cycle
    /// are preventing it from being quotable.
    ///
    /// # Returns
    ///
    /// A vector of swaps that have no reserves
    ///
    /// This is future functionality.
    #[allow(dead_code)]
    pub fn swaps_with_no_reserves(&self) -> Vec<Swap> {
        self.swaps
            .iter()
            .filter(|swap| swap.has_no_reserves())
            .cloned()
            .collect()
    }

    /// The swap rate of the cycle (a product of all swap rates in the cycle)
    ///
    /// This is future functionality.
    #[allow(dead_code)]
    fn log_rate(&self) -> i64 {
        assert!(
            self.has_all_reserves(),
            "All swaps must have reserves to calculate log rate"
        );
        self.swaps.iter().map(super::swap::Swap::log_rate).sum()
    }

    /// The optimal `amount_in` to get the maximum `amount_out`
    /// This is using binary search to find the maximum `amount_out`
    /// Memoized for efficiency since this is an expensive calculation
    ///
    /// This is future functionality.
    #[allow(dead_code)]
    pub fn best_quote(&self) -> Result<CycleQuote, Error> {
        // Check if we already have a cached result
        if let Some(cached) = self.best_quote.borrow().as_ref() {
            return Ok(cached.clone());
        }

        // Increment in derivative calculation. Too small of a delta can cause
        // the binary search to take into an infinite loop (f(x+dx) - f(x) = 0)
        // Maybe make it adjustable?
        let delta = U256::from(100);

        // This should really be gas cost, but not worth optimizing
        let mut amount_in_left = U256::from(0);

        // Maximum amount in we can use. In theory, this should be U256::MAX, but not in practice.
        // The larger the amount in the larger the slippage. We are setting the max amount in to the
        // first swap's reserve0. This is arbitrary, but probably still higher than any realistic
        // amount in. This results in 50% slippage at the max amount in. There has to be some
        // really crazy arbitrage to get anywhere near this.
        let mut amount_in_right = self.swaps[0].reserve_in();

        let mut best_quote = CycleQuote::new(self, U256::from(0));

        let precision = U256::from(1);

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
                bail!(
                    "Cycle optimization failed to converge after {} iterations",
                    count
                );
            }
            let amount_in = (amount_in_left + amount_in_right) / U256::from(2);
            let amount_in_delta = amount_in + delta;

            let quote = self.quote(amount_in);
            let quote_delta = self.quote(amount_in_delta);
            // dbg!(&quote, &quote_delta);

            if quote_delta.profit() > quote.profit() {
                // Rising profit curve
                best_quote = quote_delta;
                amount_in_left = amount_in;
            } else {
                // Falling profit curve
                best_quote = quote;
                amount_in_right = amount_in;
            }
        }

        // We are down to the `precision` from the zero - it's the zero.
        if best_quote.amount_in() == precision {
            best_quote = CycleQuote::new(self, U256::from(0));
        }

        // Cache the result
        *self.best_quote.borrow_mut() = Some(best_quote);

        // SAFETY: self.best_quote (even if it's very bad) is always Some
        #[allow(clippy::unwrap_used)]
        Ok(self.best_quote.borrow().as_ref().unwrap().clone())
    }

    /// Validates a sequence of swaps to ensure they form a valid cycle.
    ///
    /// This function checks that:
    /// - The cycle has at least 2 swaps
    /// - The first and last tokens in the cycle are the same
    /// - Each swap's output token matches the next swap's input token
    ///
    /// # Arguments
    ///
    /// * `swaps` - A vector of swaps to validate
    ///
    /// # Returns
    ///
    /// Ok(()) if the swaps form a valid cycle, or an error describing why they don't
    ///
    /// # Errors
    ///
    /// Returns an error if the swaps don't form a valid cycle
    fn validate_swaps(swaps: &Vec<Swap>) -> Result<()> {
        if swaps.len() < 2 {
            bail!("Cycle must have at least 2 swaps");
        }

        // First check token matching
        for i in 0..swaps.len() {
            let next = (i + 1) % swaps.len();
            let current_out_token = swaps[i].token_out();
            let next_in_token = swaps[next].token_in();
            if current_out_token != next_in_token {
                bail!(
                    "Swap {:#?} output token ({:#?}) does not match swap {:#?} input token ({:#?})",
                    i,
                    current_out_token,
                    next,
                    next_in_token
                );
            }
        }

        // Check for duplicate tokens (vertices)
        let mut token_counts = HashMap::new();
        for swap in swaps {
            // Count token0 as input token for current swap
            *token_counts.entry(swap.token_in()).or_insert(0) += 1;
            // Count token1 as output token for current swap and input for next swap
            *token_counts.entry(swap.token_out()).or_insert(0) += 1;

            // SAFETY: we know the it has value because we just initialized it 2 lines above
            #[allow(clippy::unwrap_used)]
            if token_counts.get(&swap.token_in()).unwrap() > &2
                || token_counts.get(&swap.token_out()).unwrap() > &2
            {
                bail!("Cycle contains duplicate tokens");
            }
        }

        // Check for duplicate swaps (edges)
        let mut seen_swaps = HashMap::new();
        for swap in swaps {
            seen_swaps.insert(swap, true);
        }
        if seen_swaps.len() < swaps.len() {
            bail!("Cycle contains duplicate swaps");
        }

        // Check for reciprocal swaps
        for combo in swaps.iter().combinations(2) {
            if combo[0].is_reciprocal(combo[1]) {
                bail!("Cycle contains reciprocal swaps");
            }
        }

        Ok(())
    }

    /// Whether the cycle has a positive rate
    /// This is based merely on pool price. Gas and slippage are not considered.
    ///
    /// This is future functionality.
    #[allow(dead_code)]
    pub fn is_positive(&self) -> bool {
        assert!(
            self.has_all_reserves(),
            "Cycle must be quotable (all swaps must have reserves)"
        );
        self.log_rate().is_positive()
    }

    /// Returns a Vec of amounts out for each swap in the cycle, including the final amount
    /// The first element is the input amount, and each subsequent element is the output
    /// amount from that swap
    ///
    /// This is future functionality.
    #[allow(dead_code)]
    pub fn quote(&self, amount_in: U256) -> CycleQuote {
        CycleQuote::new(self, amount_in)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::hash::DefaultHasher;

    use alloy::primitives::I256;

    use super::*;
    use crate::arb::test_helpers::*;

    #[test]
    fn test_new_valid_cycle() {
        let cycle = cycle(&[
            ("F3", "A", "B", 300, 120),
            ("F2", "B", "C", 200, 300),
            ("F1", "C", "A", 100, 200),
        ]);

        assert!(cycle.is_ok(), "Cycle should be valid: {:?}", cycle.err());
    }

    #[test]
    fn test_new_invalid_length() {
        let swaps = vec![swap("F1", "A", "B", 100, 200)];
        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Cycle must have at least 2 swaps"
        );
    }

    #[test]
    fn test_new_invalid_reciprocal_swaps() {
        let swaps = vec![
            swap("F1", "A", "B", 100, 200),
            swap("F1", "B", "A", 200, 100),
        ];
        let cycle = Cycle::new(swaps);

        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Cycle contains reciprocal swaps"
        );
    }

    #[test]
    fn test_new_invalid_duplicate_swaps() {
        // A->B->C->A->B->A
        let swaps = vec![
            swap("F1", "A", "B", 100, 200),
            swap("F2", "B", "C", 100, 200),
            swap("F3", "C", "A", 100, 200),
            swap("F1", "A", "B", 200, 100),
            swap("F2", "B", "A", 200, 100),
        ];
        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Cycle contains duplicate tokens"
        );
    }

    #[test]
    fn test_new_invalid_token_mismatch() {
        // Create two swaps where the output token of the first doesn't match the input token of the second
        let swaps = vec![
            swap("F1", "A", "B", 100, 200),
            swap("F2", "C", "D", 200, 100),
        ];

        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Swap 0 output token (B) does not match swap 1 input token (C)"
        );
    }

    #[test]
    fn test_new_invalid_non_simple_cycle() {
        // A->B->C->B->A - B is repeated
        let swaps = vec![
            swap("F1", "A", "B", 100, 200),
            swap("F2", "B", "C", 200, 100),
            swap("F3", "C", "B", 100, 200),
            swap("F4", "B", "A", 200, 100),
        ];
        let cycle = Cycle::new(swaps);
        assert_eq!(
            cycle.err().unwrap().to_string(),
            "Cycle contains duplicate tokens"
        );
    }

    #[test]
    fn test_log_rate() {
        let swap1 = swap("F1", "A", "B", 100, 200);
        assert_eq!(swap1.log_rate(), 299_725);
        let swap2 = swap("F2", "B", "A", 300, 100);
        assert_eq!(swap2.log_rate(), -478_426);

        let cycle = Cycle::new(vec![swap1, swap2]).unwrap();
        assert_eq!(cycle.log_rate(), 299_725 - 478_426);
    }

    #[test]
    fn test_best_quote_not_exploitable() {
        let cycle = cycle(&[
            ("F1", "A", "B", 100, 200), // 2 rate
            ("F2", "B", "A", 300, 100), // 1/3 rate
        ]);
        let best_quote = cycle.unwrap().best_quote().unwrap();

        assert_eq!(best_quote.amount_in(), U256::from(0));
        assert_eq!(best_quote.amount_out(), U256::from(0));
        assert_eq!(best_quote.profit(), I256::ZERO);
        assert_eq!(best_quote.profit_margin(), 0);
    }

    #[test]
    fn test_best_quote_exploitable() {
        let cycle_instance = cycle(&[
            ("F1", "A", "B", 1_000_000, 2_000_000), // 2 rate
            ("F2", "B", "A", 3_000_000, 3_000_000), // 1 rate
        ])
        .unwrap();

        // Ensure the cycle is profitable for testing
        assert!(
            cycle_instance.is_positive(),
            "Cycle should be profitable for this test"
        );

        let amount_in = 248_054;
        let mid_amount = 396_549;
        let amount_out = 349_323;
        let profit = 101_269;

        let cycle_clone = cycle_instance;
        let best_quote = cycle_clone.best_quote().unwrap();

        assert!(
            best_quote.swap_quotes().len() == 2,
            "best_swap_quotes should be Some after optimize"
        );
        let quotes = best_quote.swap_quotes();
        assert_eq!(quotes.len(), 2);

        assert_eq!(quotes[0].amount_in(), U256::from(amount_in));
        assert_eq!(quotes[0].amount_out(), U256::from(mid_amount));
        assert_eq!(quotes[1].amount_in(), U256::from(mid_amount));
        assert_eq!(quotes[1].amount_out(), U256::from(amount_out));

        assert_eq!(best_quote.amount_in(), U256::from(amount_in));
        assert_eq!(best_quote.profit(), I256::from_raw(U256::from(profit)));
    }

    #[test]
    fn test_best_quote_with_wild_exchange_rate() {
        let cycle_instance = cycle(&[
            ("F1", "A", "B", 1_000_000, 2_000_000_000_000_000_000), // 2e12 rate
            ("F2", "B", "A", 2_000_000_000_000_000_000, 2_000_000), // 1e12 rate
        ])
        .unwrap();

        // Ensure the cycle is profitable for testing
        assert!(
            cycle_instance.is_positive(),
            "Cycle should be profitable for this test"
        );

        let best_quote = cycle_instance.best_quote().unwrap();

        assert!(
            best_quote.swap_quotes().len() == 2,
            "best_swap_quotes should be Some after optimize"
        );
        let quotes = best_quote.swap_quotes();
        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[0].amount_in(), U256::from(204_322));
        assert_eq!(
            quotes[0].amount_out(),
            U256::from(338_468_896_130_258_668_u64)
        );
        assert_eq!(
            quotes[1].amount_in(),
            U256::from(338_468_896_130_258_668_u64)
        );
        assert_eq!(quotes[1].amount_out(), U256::from(288_736));

        assert_eq!(best_quote.amount_in(), U256::from(204_322));
        assert_eq!(best_quote.profit(), I256::from_raw(U256::from(84_414)));
        assert_eq!(best_quote.profit_margin(), 4131);
    }

    fn hash(cycle: &Cycle) -> u64 {
        let mut hasher = DefaultHasher::new();
        cycle.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_equality_and_hash() {
        // Note that for equality purposes we only care about tokens and their order:
        // starting token and reserves are not considered
        let cycle1 = cycle(&[
            ("F1", "A", "B", 100, 200),
            ("F2", "B", "C", 300, 100),
            ("F3", "C", "A", 100, 200),
        ])
        .unwrap();

        // Reflexive
        assert_eq!(cycle1, cycle1);
        // Check hash
        assert_eq!(hash(&cycle1), hash(&cycle1));

        // Same order as cycle1 but with a different starting swap
        let cycle2 = cycle(&[
            ("F2", "B", "C", 30, 10),
            ("F3", "C", "A", 10, 20),
            ("F1", "A", "B", 10, 20),
        ])
        .unwrap();

        // Symmetric
        assert_eq!(cycle1, cycle2);
        assert_eq!(cycle2, cycle1);
        assert_eq!(hash(&cycle1), hash(&cycle2));

        // Another rotation of cycle1
        let cycle3 = cycle(&[
            ("F3", "C", "A", 10, 20),
            ("F1", "A", "B", 10, 20),
            ("F2", "B", "C", 30, 10),
        ])
        .unwrap();

        // Transitive
        assert_eq!(cycle1, cycle3);
        assert_eq!(cycle3, cycle1);
        assert_eq!(hash(&cycle1), hash(&cycle3));
    }

    #[test]
    fn test_inequality() {
        let cycle1 = cycle(&[
            ("F1", "A", "B", 100, 200),
            ("F2", "B", "C", 300, 100),
            ("F3", "C", "A", 100, 200),
        ])
        .unwrap();

        let cycle2 = cycle(&[
            ("F1", "B", "A", 100, 200),
            ("F2", "A", "C", 300, 100),
            ("F3", "C", "B", 100, 200),
        ])
        .unwrap();

        assert_ne!(cycle1, cycle2);
    }
}
