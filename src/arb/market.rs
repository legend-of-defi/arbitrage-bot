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

use alloy::primitives::U256;

use super::{
    cycle::Cycle,
    pool::Pool,
    swap::{Swap, SwapId},
    token::{Token, TokenId},
};

pub type TokenIndex = usize;
pub type SwapIndex = usize;

#[derive(Debug, Clone, Default)]
pub struct Market {
    /// Our balances indexed by Token Address
    #[allow(dead_code)]
    pub balances: HashMap<TokenId, U256>,

    /// Our tokens indexed by `TokenId`
    #[allow(dead_code)]
    pub our_token_vec: Vec<TokenIndex>,

    /// Tokens indexed by `TokenId`
    #[allow(dead_code)]
    pub token_vec: Vec<Token>,

    /// Address to `TokenId`
    #[allow(dead_code)]
    pub token_map: HashMap<TokenId, TokenIndex>,

    /// Swaps indexed by `SwapId`
    #[allow(dead_code)]
    pub swap_vec: Vec<Swap>,

    /// `SwapKey` to `SwapId`
    #[allow(dead_code)]
    pub swap_map: HashMap<SwapId, SwapIndex>,

    // Adjacency list of TokenId (Vertex) to a list of SwapId (Edges)
    #[allow(dead_code)]
    pub graph: Vec<Vec<SwapIndex>>,
}

impl Market {
    /// Create a new market from a set of pools loaded from the database
    /// Called at startup
    #[allow(dead_code)]
    pub fn new(pools: &HashSet<Pool>, balances: HashMap<TokenId, U256>) -> Self {
        // Build token_vec with deduplication
        let mut token_set = HashSet::new();
        for pool in pools {
            token_set.insert(pool.token0.clone());
            token_set.insert(pool.token1.clone());
        }
        let mut token_vec: Vec<_> = token_set.into_iter().map(Token::new).collect();
        token_vec.sort();

        let num_tokens = token_vec.len();
        let num_swaps = pools.len() * 2; // Each pool has 2 swaps (forward/reverse)

        // Build token_map with capacity
        let mut token_map = HashMap::with_capacity(num_tokens);
        for (token_id, token) in token_vec.iter().enumerate() {
            token_map.insert(token.id.clone(), token_id);
        }

        // Build our_token_vec with capacity
        let mut our_token_vec = Vec::with_capacity(balances.len());
        for address in balances.keys() {
            if let Some(&token_id) = token_map.get(address) {
                our_token_vec.push(token_id);
            }
        }
        our_token_vec.sort_unstable();

        // Build swap_vec with capacity
        let mut swap_vec = Vec::with_capacity(num_swaps);
        for pool in pools {
            swap_vec.push(Swap::new(
                SwapId::from(format!("{}-fwd", pool.id).as_str()),
                pool.id.clone(),
                pool.token0.clone(),
                pool.token1.clone(),
                pool.reserve0,
                pool.reserve1,
            ));
            swap_vec.push(Swap::new(
                SwapId::from(format!("{}-rev", pool.id).as_str()),
                pool.id.clone(),
                pool.token1.clone(),
                pool.token0.clone(),
                pool.reserve1,
                pool.reserve0,
            ));
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
            graph[token_map[&swap.token0]].push(swap_id); // Only add outgoing edges
        }

        Self {
            balances,
            our_token_vec,
            token_vec,
            token_map,
            swap_vec,
            swap_map,
            graph,
        }
    }

    /// Update the market with new pool reserves and balances and return affected cycles
    /// Call this once per block with new pools and balances
    #[allow(dead_code)]
    pub fn update(
        &mut self,
        new_pools: HashSet<Pool>,
        new_balances: HashMap<TokenId, U256>,
    ) -> Vec<Cycle> {
        // Update balances
        for (token_id, balance) in new_balances {
            self.balances.insert(token_id, balance);
        }

        let updated_swaps = self.update_swaps(new_pools);
        let cycles = self.find_cycles(updated_swaps);
        self.exploitable_cycles(cycles)
    }

    #[allow(dead_code)]
    fn update_swaps(&mut self, updated_pools: HashSet<Pool>) -> Vec<SwapIndex> {
        let mut updated_swaps = Vec::new();

        for pool in updated_pools {
            let forward = Swap::forward(&pool);
            if let Some(&swap_id) = self.swap_map.get(&forward.id) {
                self.swap_vec[swap_id] = forward;
                updated_swaps.push(swap_id);
            }

            let reverse = Swap::reverse(&pool);
            if let Some(&swap_id) = self.swap_map.get(&reverse.id) {
                self.swap_vec[swap_id] = reverse;
                updated_swaps.push(swap_id);
            }
        }

        updated_swaps
    }

    /// Find cycles that include at least one updated swap and at least one of our tokens
    #[allow(dead_code)]
    fn find_cycles(&self, updated_swaps: Vec<SwapIndex>) -> Vec<Cycle> {
        let mut cycles = Vec::new();
        let mut unique_cycles = HashSet::new();
        let updated_set: HashSet<_> = updated_swaps.into_iter().collect();

        // For each token we own find cycles up to depth 3
        for &start_token in &self.our_token_vec {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            self.dfs_cycles(
                start_token,
                start_token,
                &updated_set,
                &mut visited,
                &mut path,
                &mut cycles,
                0,
                3,
            );
        }

        // Convert paths to cycles and deduplicate
        cycles
            .iter()
            .filter_map(|path| {
                let swaps = path
                    .iter()
                    .map(|swap_id| self.swap_vec[*swap_id].clone())
                    .collect();
                Cycle::new(swaps).ok()
            })
            .filter(|cycle| unique_cycles.insert(cycle.clone()))
            .collect()
    }

    #[allow(dead_code)]
    fn profitable_cycles(cycles: Vec<Cycle>) -> Vec<Cycle> {
        cycles
            .into_iter()
            .filter(super::cycle::Cycle::is_profitable)
            .collect()
    }

    #[allow(dead_code)]
    fn exploitable_cycles(&self, cycles: Vec<Cycle>) -> Vec<Cycle> {
        let mut exploitable_cycles = Vec::new();

        // Optimize and filter exploitable cycles
        for cycle in Self::profitable_cycles(cycles) {
            let mut cycle = cycle;
            cycle.optimize(self.balances[&cycle.swaps[0].token0]);

            if cycle.is_exploitable() {
                exploitable_cycles.push(cycle);
            }
        }

        // Sort by max_profit in descending order
        exploitable_cycles.sort_by(|a, b| b.max_profit.cmp(&a.max_profit));

        exploitable_cycles
    }

    #[allow(clippy::too_many_arguments)]
    fn dfs_cycles(
        &self,
        start_token: TokenIndex,
        current_token: TokenIndex,
        updated_swaps: &HashSet<SwapIndex>,
        visited: &mut HashSet<SwapIndex>,
        path: &mut Vec<SwapIndex>,
        cycles: &mut Vec<Vec<SwapIndex>>,
        depth: usize,
        max_depth: usize,
    ) {
        // Check if we found a cycle back to start
        if depth > 0 && current_token == start_token {
            // Only include cycles that contain at least one updated swap
            if path.iter().any(|swap_id| updated_swaps.contains(swap_id)) {
                cycles.push(path.clone());
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

            // Skip if this swap is reciprocal of previous swap
            if let Some(&prev_id) = path.last() {
                let prev_swap = &self.swap_vec[prev_id];
                if swap.is_reverse(prev_swap) {
                    continue;
                }
            }

            visited.insert(swap_id);
            path.push(swap_id);

            self.dfs_cycles(
                start_token,
                self.token_map[&swap.token1],
                updated_swaps,
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

    use crate::arb::test_helpers::*;

    #[test]
    fn test_new_trivial() {
        // One pool P1 with A/B and 100/200 reserves, A 100 is our base token reserve
        let market = market(&[("P1", "A", "B", 100, 200)], &[("A", 100)]);

        assert_eq!(market.token_vec, vec![token("A"), token("B")]);

        assert_eq!(
            market.token_map,
            HashMap::from([(token("B").id, 1), (token("A").id, 0),])
        );

        assert_eq!(
            market.swap_vec,
            vec![
                swap("P1-fwd", "A", "B", 100, 200),
                swap("P1-rev", "B", "A", 200, 100),
            ]
        );

        assert_eq!(
            market.swap_map,
            HashMap::from([
                (market.swap_vec[0].id.clone(), 0),
                (market.swap_vec[1].id.clone(), 1),
            ])
        );

        assert_eq!(market.graph, vec![vec![0], vec![1],]);
    }

    #[test]
    fn test_new_cycle() {
        let market = market(
            &[
                ("Pool1", "A", "B", 100, 200), // Pool1: A->B
                ("Pool2", "B", "A", 300, 100), // Pool2: B->A
            ],
            &[("A", 100)],
        );

        assert_eq!(market.token_vec, vec![token("A"), token("B")]);

        assert_eq!(
            market.token_map,
            HashMap::from([(token("B").id, 1), (token("A").id, 0),])
        );

        assert_eq!(
            market.swap_vec,
            vec![
                swap("Pool1-fwd", "A", "B", 100, 200), // 0 Forward: A->B in Pool1
                swap("Pool2-rev", "A", "B", 100, 300), // 1 Forward: A->B in Pool2
                swap("Pool1-rev", "B", "A", 200, 100), // 3 Reverse: B->A in Pool1
                swap("Pool2-fwd", "B", "A", 300, 100), // 2 Reverse: B->A in Pool2
            ]
        );

        // Token A (0) has swaps A->B (0,1)
        // Token B (1) has swaps B->A (2,3)
        assert_eq!(
            market.graph,
            vec![
                vec![0, 1], // Token A's swaps
                vec![2, 3], // Token B's swaps
            ]
        );
    }

    #[test]
    fn test_our_tokens() {
        let market = market(
            &[("Pool1", "A", "B", 100, 200), ("Pool2", "B", "C", 300, 100)],
            &[("A", 100), ("C", 200)], // We have balances in A and C
        );

        assert_eq!(market.token_vec, vec![token("A"), token("B"), token("C")]);

        // our_token_vec should only contain tokens we have balances for
        assert_eq!(market.our_token_vec, vec![0, 2]); // TokenIds for A and C
    }

    #[test]
    fn test_update_swaps() {
        let original_pool = pool("Pool1", "A", "B", 100, 200);
        let balances = HashMap::from([(token("A").id, U256::from(100))]);

        let mut market = Market::new(&HashSet::from([original_pool]), balances);

        let updated_pool = pool("Pool1", "A", "B", 100, 300);

        let updated_swaps = market.update_swaps(HashSet::from([updated_pool]));
        assert_eq!(updated_swaps, vec![0, 1]);

        assert_eq!(
            market.swap_vec,
            vec![
                swap("Pool1-fwd", "A", "B", 100, 300),
                swap("Pool1-rev", "B", "A", 300, 100),
            ]
        );
    }

    #[test]
    fn test_find_cycles() {
        let market = market(
            &[("Pool1", "A", "B", 100, 200), ("Pool2", "B", "A", 300, 100)],
            &[("A", 100)],
        );

        let cycles = market.find_cycles(vec![0, 1]);
        assert_eq!(
            cycles,
            vec![
                cycle(&[
                    ("Pool1-fwd", "A", "B", 100, 200),
                    ("Pool2-fwd", "B", "A", 300, 100),
                ]),
                cycle(&[
                    ("Pool2-rev", "A", "B", 100, 300),
                    ("Pool1-rev", "B", "A", 200, 100),
                ]),
            ]
        );
    }

    #[test]
    fn test_profitable_but_not_exploitable_cycles() {
        let market = market(
            &[("Pool1", "A", "B", 100, 200), ("Pool2", "B", "A", 300, 100)],
            &[("A", 100)],
        );

        let cycles = market.find_cycles(vec![0, 1]);
        assert_eq!(cycles.len(), 2);

        let profitable_cycles = Market::profitable_cycles(cycles.clone());
        assert_eq!(profitable_cycles.len(), 1);
        dbg!(&profitable_cycles[0]);

        let exploitable_cycles = market.exploitable_cycles(cycles);
        assert_eq!(exploitable_cycles.len(), 0);
    }

    #[test]
    fn test_exploitable_cycles() {
        let market = market(
            &[
                ("Pool1", "A", "B", 100_000, 200_000_000_000_000),
                ("Pool2", "B", "A", 300_000_000_000_000, 100_000),
            ],
            &[("A", 100_000)],
        );

        let cycles = market.find_cycles(vec![0, 1]);
        let exploitable_cycles = market.exploitable_cycles(cycles);
        assert_eq!(exploitable_cycles.len(), 1);
        let cycle = &exploitable_cycles[0];
        assert_eq!(cycle.max_profit, Some(U256::from(1_964)));
        assert_eq!(cycle.max_profit_margin, Some(0.222_877_893_781_207_45));
        assert_eq!(cycle.best_amount_in, Some(U256::from(8_812)));
        assert_eq!(cycle.log_rate, 176_092);
        assert_eq!(cycle.swaps.len(), 2);
        assert_eq!(
            cycle.swaps,
            vec![
                swap("Pool2-rev", "A", "B", 100_000, 300_000_000_000_000),
                swap("Pool1-rev", "B", "A", 200_000_000_000_000, 100_000),
            ]
        );
    }
}
