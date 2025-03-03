/// This is the core of the arbitrage detection logic
///
/// Usage:
/// Call this once at startup with all pools and balances
/// let mut market = `Market::new(pools`, balances);
///
/// Call this once per block with new pools
/// `market.update(new_pools`, `new_balances`) -> Vec<Cycle>
///
/// Returns a list of cycles that are profitable and exploitable, meaning they include at least
/// one of supported tokens in our balances.
use std::collections::{HashMap, HashSet};

use super::{
    cycle::Cycle,
    pool::Pool,
    swap::{Swap, SwapId},
    token::{Token, TokenId},
    world_update::WorldUpdate,
};

pub type TokenIndex = usize;
pub type SwapIndex = usize;

#[derive(Debug, Clone, Default)]
pub struct World {
    /// Tokens indexed by `TokenIndex`
    pub token_vec: Vec<Token>,

    /// `TokenId` to `TokenIndex` mapping
    pub token_map: HashMap<TokenId, TokenIndex>,

    /// Swaps indexed by `SwapIndex`
    pub swap_vec: Vec<Swap>,

    /// `SwapId` to `SwapIndex` mapping
    pub swap_map: HashMap<SwapId, SwapIndex>,

    /// Adjacency list of `TokenId` (Vertex) to a list of `SwapId` (outgoing edges)
    pub graph: Vec<Vec<SwapIndex>>,

    /// All cycles
    pub cycle_vec: Vec<Cycle>,
}

impl World {
    /// Create a new market from a set of pools loaded from the database
    /// Called at startup
    pub fn new(pools: &HashSet<Pool>) -> Self {
        // Build token_vec with deduplication
        let mut token_set = HashSet::new();
        for pool in pools {
            token_set.insert(pool.token0);
            token_set.insert(pool.token1);
        }
        let mut token_vec: Vec<_> = token_set.into_iter().map(Token::new).collect();
        token_vec.sort();

        let num_tokens = token_vec.len();
        let num_swaps = pools.len() * 2; // Each pool has 2 swaps (forward/reverse)

        // Build token_map with capacity
        let mut token_map = HashMap::with_capacity(num_tokens);
        for (token_id, token) in token_vec.iter().enumerate() {
            token_map.insert(token.id, token_id);
        }

        // Build swap_vec with capacity
        let mut swap_vec = Vec::with_capacity(num_swaps);
        for pool in pools {
            swap_vec.push(Swap::forward(pool));
            swap_vec.push(Swap::reverse(pool));
        }
        swap_vec.sort(); // Sort before building graph

        // Build swap_map with capacity
        let mut swap_map = HashMap::with_capacity(num_swaps);
        for (swap_id, swap) in swap_vec.iter().enumerate() {
            swap_map.insert(swap.id.clone(), swap_id);
        }

        // Build graph with capacity - adjacency list of tokens to swaps
        let mut graph = Vec::with_capacity(num_tokens);
        for _ in 0..num_tokens {
            graph.push(Vec::with_capacity(num_swaps / num_tokens)); // Average swaps per token
        }

        // Add edges (swaps) from each token to its swaps
        for (swap_id, swap) in swap_vec.iter().enumerate() {
            let token_index = token_map[&swap.token_in];
            graph[token_index].push(swap_id); // Add outgoing edges based on input token
        }

        let mut market = Self {
            token_vec,
            token_map,
            swap_vec,
            swap_map,
            graph,
            cycle_vec: Vec::new(),
        };

        // Find all cycles once during initialization
        market.cycle_vec = market.cycle_vec();

        market
    }

    /// Update the market with new pool reserves and balances and return affected cycles
    /// Call this once per block with new pools and balances
    pub fn update(&mut self, pools: &HashSet<Pool>) -> WorldUpdate {
        let updated_swaps = self.update_swaps(pools.clone());
        let updated_cycles = self.update_cycles(&updated_swaps);
        WorldUpdate::new(updated_cycles)
    }

    // Update the swaps in the market and return the updated swaps
    fn update_swaps(&mut self, updated_pools: HashSet<Pool>) -> Vec<Swap> {
        let mut updated_swaps = Vec::with_capacity(updated_pools.len() * 2);

        for pool in updated_pools {
            let forward = Swap::forward(&pool);
            if let Some(&swap_id) = self.swap_map.get(&forward.id) {
                self.swap_vec[swap_id] = forward.clone();
                updated_swaps.push(forward);
            }

            let reverse = Swap::reverse(&pool);
            if let Some(&swap_id) = self.swap_map.get(&reverse.id) {
                self.swap_vec[swap_id] = reverse.clone();
                updated_swaps.push(reverse);
            }
        }

        updated_swaps
    }

    // Update the cycles in the market and return the updated cycles
    fn update_cycles(&self, updated_swaps: &[Swap]) -> Vec<Cycle> {
        // Filter all_cycles to only include cycles with at least one updated swap
        let updated_set: HashSet<Swap> = updated_swaps.iter().cloned().collect();

        self.cycle_vec
            .iter()
            .filter(|cycle| {
                cycle.swaps.iter().any(|swap| {
                    if let Some(&swap_id) = self.swap_map.get(&swap.id) {
                        updated_set.contains(&self.swap_vec[swap_id])
                    } else {
                        false
                    }
                })
            })
            .cloned()
            .collect()
    }

    fn cycle_vec(&self) -> Vec<Cycle> {
        // Even though Cycle itself is mutable, the way we calculate hash is immutable
        #[allow(clippy::mutable_key_type)]
        let mut cycles: HashSet<Cycle> = HashSet::new();

        // For each token, find cycles starting from that token
        for token_idx in 0..self.token_vec.len() {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            self.dfs_find_cycles(
                token_idx,
                token_idx,
                &mut visited,
                &mut path,
                &mut cycles,
                0,
                3, // max cycle depth
            );
        }
        let mut cycles_vec = cycles.into_iter().collect::<Vec<_>>();
        cycles_vec.sort_by(std::cmp::Ord::cmp);
        cycles_vec
    }

    /// Find all cycles in the graph using DFS
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::mutable_key_type)]
    fn dfs_find_cycles(
        &self,
        start_token: TokenIndex,
        current_token: TokenIndex,
        visited: &mut HashSet<SwapIndex>,
        path: &mut Vec<Swap>,
        cycles: &mut HashSet<Cycle>,
        depth: usize,
        max_depth: usize,
    ) {
        // Check if we found a cycle back to start
        if depth > 0 && current_token == start_token {
            // Create a new cycle with the current path
            if let Ok(cycle) = Cycle::new(path.clone()) {
                cycles.insert(cycle);
            }
            return;
        }

        // Stop if we hit max depth
        if depth >= max_depth {
            return;
        }

        // Try each outgoing swap from current token
        for &swap_id in &self.graph[current_token] {
            if visited.contains(&swap_id) {
                continue;
            }

            let swap = &self.swap_vec[swap_id];

            // Only consider swaps where current token is the input token
            if swap.token_in != self.token_vec[current_token].id {
                continue;
            }

            let output_token = swap.token_out;
            if !self.token_map.contains_key(&output_token) {
                continue;
            }

            let next_token = self.token_map[&output_token];

            visited.insert(swap_id);
            path.push(swap.clone());

            self.dfs_find_cycles(
                start_token,
                next_token,
                visited,
                path,
                cycles,
                depth + 1,
                max_depth,
            );

            path.pop();
            visited.remove(&swap_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::map::HashMap;

    use crate::arb::pool::PoolId;
    use crate::arb::swap::Direction;
    use crate::arb::test_helpers::*;

    #[test]
    fn test_new_no_arbitrage() {
        // One pool P1 with A/B and 100/200 reserves, A 100 is our base token reserve
        let market = world(&[("F1", "A", "B", 100, 200)], &[("A", 100)]);

        assert_eq!(market.token_vec, vec![token("A"), token("B")]);

        assert_eq!(
            market.token_map,
            HashMap::from([(token("B").id, 1), (token("A").id, 0),])
        );

        assert_eq!(
            market.swap_vec,
            vec![
                swap("F1", "A", "B", 100, 200),
                swap("F1", "B", "A", 200, 100),
            ]
        );

        assert_eq!(
            market.swap_map,
            HashMap::from([
                (
                    SwapId {
                        pool_id: PoolId::from(address_from_str("F1")),
                        direction: Direction::ZeroForOne,
                    },
                    0
                ),
                (
                    SwapId {
                        pool_id: PoolId::from(address_from_str("F1")),
                        direction: Direction::OneForZero,
                    },
                    1
                ),
            ])
        );

        assert_eq!(market.graph, vec![vec![0], vec![1],]);
    }

    #[test]
    fn test_new_with_arbitrage() {
        // Create a triangular arbitrage opportunity with 3 pools:
        // Pool1: A/B with 100/200 reserves
        // Pool2: B/C with 200/300 reserves
        // Pool3: A/C with 120/300 reserves
        // A->B->C->A
        // We have balance of 100 token A
        let market = world(
            &[
                ("F1", "A", "B", 100, 200),
                ("F2", "B", "C", 200, 300),
                ("F3", "A", "C", 120, 300),
            ],
            &[("A", 100)],
        );

        assert_eq!(market.token_vec, vec![token("A"), token("B"), token("C")]);

        assert_eq!(
            market.token_map,
            HashMap::from([(token("A").id, 0), (token("B").id, 1), (token("C").id, 2),])
        );

        assert_eq!(
            market.swap_vec,
            vec![
                swap("F1", "A", "B", 100, 200), // 0
                swap("F3", "A", "C", 120, 300), // 4
                swap("F1", "B", "A", 200, 100), // 1
                swap("F2", "B", "C", 200, 300), // 2
                swap("F3", "C", "A", 300, 120), // 5
                swap("F2", "C", "B", 300, 200), // 3
            ]
        );

        // Token A (0) has swaps A->B and A->C
        // Token B (1) has swaps B->A and B->C
        // Token C (2) has swaps C->A and C->B
        assert_eq!(
            market.graph,
            vec![
                vec![0, 1], // Token A's swaps: 0: A->B, 5: C->A
                vec![2, 3], // Token B's swaps: 2: B->C, 3: C->B
                vec![4, 5], // Token C's swaps: 4: C->A, 1: B->A
            ]
        );

        let cycle1 = cycle(&[
            ("F1", "A", "B", 100, 200),
            ("F2", "B", "C", 200, 300),
            ("F3", "C", "A", 300, 120),
        ])
        .unwrap();
        let cycle2 = cycle(&[
            ("F3", "A", "C", 120, 300),
            ("F2", "C", "B", 300, 200),
            ("F1", "B", "A", 200, 100),
        ])
        .unwrap();
    }

    #[test]
    fn test_new_cycle() {
        let world = world(
            &[
                ("F1", "A", "B", 100, 200), // Pool1: A->B
                ("F2", "A", "B", 300, 100), // Pool2: B->A
            ],
            &[("A", 100)],
        );

        assert_eq!(world.token_vec, vec![token("A"), token("B")]);

        assert_eq!(
            world.token_map,
            HashMap::from([(token("B").id, 1), (token("A").id, 0),])
        );

        assert_eq!(
            world.swap_vec,
            vec![
                swap("F1", "A", "B", 100, 200), // 0 Forward: A->B in Pool1
                swap("F2", "A", "B", 100, 300), // 3 Reverse: B->A in Pool2
                swap("F1", "B", "A", 200, 100), // 2 Reverse: B->A in Pool1
                swap("F2", "B", "A", 300, 100), // 1 Forward: A->B in Pool2
            ]
        );

        // Token A (0) has swaps A->B (0,1)
        // Token B (1) has swaps B->A (2,3)
        assert_eq!(
            world.graph,
            vec![
                vec![0, 1], // Token A's swaps
                vec![2, 3], // Token B's swaps
            ]
        );
    }

    #[test]
    fn test_our_tokens() {
        let world = world(
            &[("F1", "A", "B", 100, 200), ("F2", "B", "C", 300, 100)],
            &[("A", 100), ("C", 200)], // We have balances in A and C
        );

        assert_eq!(world.token_vec, vec![token("A"), token("B"), token("C")]);
    }

    #[test]
    fn test_update_swaps() {
        let original_pool = pool("F1", "A", "B", 100, 200);
        let mut world = World::new(&HashSet::from([original_pool]));

        let updated_pool = pool("F1", "A", "B", 100, 300);

        let updated_swaps = world.update_swaps(HashSet::from([updated_pool]));
        assert_eq!(
            updated_swaps,
            vec![
                swap("F1", "A", "B", 100, 300),
                swap("F1", "B", "A", 300, 100)
            ]
        );

        assert_eq!(
            world.swap_vec,
            vec![
                swap("F1", "A", "B", 100, 300),
                swap("F1", "B", "A", 300, 100),
            ]
        );
    }

    #[test]
    fn test_find_cycles() {
        let world = world(
            &[("F1", "A", "B", 100, 200), ("F2", "A", "B", 100, 300)],
            &[("A", 100)],
        );

        // The test expects to find both cycles
        assert_eq!(
            world.cycle_vec,
            vec![
                cycle(&[("F1", "A", "B", 100, 200), ("F2", "B", "A", 300, 100),]).unwrap(),
                cycle(&[("F2", "A", "B", 100, 300), ("F1", "B", "A", 200, 100),]).unwrap(),
            ]
        );
    }

    // #[test]
    // fn test_profitable_but_not_exploitable_cycles() {
    //     let market = market(
    //         &[("F1", "A", "B", 100, 200), ("F2", "A", "B", 100, 300)],
    //         &[("A", 1)],
    //     );

    //     let cycles = market.update_cycles(vec![0, 1]);
    //     assert_eq!(cycles.len(), 2);

    //     let profitable_cycles = market.profitable_updated_cycles();
    //     assert_eq!(profitable_cycles.len(), 1);
    //     let profitable_cycle = &profitable_cycles[0];
    //     assert!(profitable_cycle.max_profit.is_none());
    //     assert!(profitable_cycle.max_profit_margin.is_none());
    //     assert!(profitable_cycle.best_amount_in.is_none());
    //     assert!(profitable_cycle.best_swap_quotes.is_none());
    //     assert_eq!(profitable_cycle.swap_sides.len(), 2);
    //     assert_eq!(profitable_cycle.log_rate, 176_092);

    //     let exploitable_cycles = market.exploitable_updated_cycle_quotes();
    //     assert_eq!(exploitable_cycles.len(), 0);
    // }

    // #[test]
    // fn test_exploitable_cycles() {
    //     let market = market(
    //         &[
    //             ("F1", "A", "B", 100_000, 200_000_000_000_000),
    //             ("F2", "A", "B", 105_000, 200_000_000_000_000),
    //         ],
    //         &[("A", 100_000)],
    //     );
    //     assert_eq!(market.updated_profitable_cycles.len(), 1);
    //     let profitable_cycle = &market.updated_profitable_cycles[0];
    //     assert_eq!(
    //         profitable_cycle.max_profit,
    //         Some(I256::from_raw(U256::from(21)))
    //     );
    //     assert_eq!(
    //         profitable_cycle.max_profit_margin,
    //         Some(0.026_888_604_353_393_086)
    //     );
    //     assert_eq!(profitable_cycle.best_amount_in, Some(U256::from(781)));
    //     assert_eq!(profitable_cycle.log_rate, 21_189);
    //     assert_eq!(profitable_cycle.swap_sides.len(), 2);

    //     let exploitable_cycles = market.exploitable_updated_cycle_quotes();
    //     assert_eq!(exploitable_cycles.len(), 1);
    //     let cycle = &exploitable_cycles[0];
    //     assert_eq!(cycle.max_profit, Some(I256::from_raw(U256::from(21))));
    //     assert_eq!(cycle.max_profit_margin, Some(0.026_888_604_353_393_086));
    //     assert_eq!(cycle.best_amount_in, Some(U256::from(781)));
    //     assert_eq!(cycle.log_rate, 21_189);
    //     assert_eq!(cycle.swap_sides.len(), 2);
    //     assert_eq!(
    //         cycle.swap_sides,
    //         vec![
    //             swap("F1", "A", "B", 100_000, 200_000_000_000_000),
    //             swap("F2", "B", "A", 300_000_000_000_000, 105_000),
    //         ]
    //     );
    // }
}
