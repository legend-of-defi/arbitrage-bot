mod cycle;
mod market;
mod pool;
mod swap_side;
mod test_helpers;
mod token;
mod types;
mod swap_quote;
// Maps each token to a list of possible swaps from that token
// #[allow(dead_code)]
// type AdjacencyList = HashMap<TokenAddress, Vec<Swap>>;

// /// Builds an adjacency list from a list of pools
// /// Used to find 3-swap cycles
// #[allow(dead_code)]
// fn build_adjacency(pools: &HashSet<Pool>) -> AdjacencyList {
//     let mut adj = HashMap::new();

//     for pool in pools {
//         // Add forward swap: token0 -> token1
//         adj.entry(pool.token0.clone())
//             .or_insert_with(Vec::new)
//             .push(Swap {
//                 pool: pool.clone(),
//                 to: pool.token1.clone(),
//                 rate: (pool.reserve1 as f64 / pool.reserve0 as f64) * FEE_MULTIPLIER,
//             });

//         // Add reverse swap: token1 -> token0
//         adj.entry(pool.token1.clone())
//             .or_insert_with(Vec::new)
//             .push(Swap {
//                 pool: pool.clone(),
//                 to: pool.token0.clone(),
//                 rate: (pool.reserve0 as f64 / pool.reserve1 as f64) * FEE_MULTIPLIER,
//             });
//     }
//     adj
// }

// /// Finds all 3-swap cycles in the pool network starting from a given token
// #[allow(dead_code)]
// fn node_triangular_cycles(adj: &AdjacencyList, start_token: &TokenAddress) -> Vec<Cycle> {
//     let mut cycles = Vec::new();

//     // Get possible first swaps
//     let Some(first_swaps) = adj.get(start_token) else {
//         return cycles;
//     };

//     for swap1 in first_swaps {
//         // Get possible second swaps
//         let Some(second_swaps) = adj.get(&swap1.to) else {
//             continue;
//         };

//         for swap2 in second_swaps {
//             // Skip if using same pool
//             if swap2.pool == swap1.pool {
//                 continue;
//             }

//             // Get possible final swaps
//             let Some(final_swaps) = adj.get(&swap2.to) else {
//                 continue;
//             };

//             for swap3 in final_swaps {
//                 // Skip if using same pools
//                 if swap3.pool == swap1.pool || swap3.pool == swap2.pool {
//                     continue;
//                 }

//                 // Check if cycle completes
//                 if swap3.to == *start_token {
//                     cycles.push(Cycle {
//                         swaps: vec![swap1.clone(), swap2.clone(), swap3.clone()],
//                         rate: swap1.rate * swap2.rate * swap3.rate,
//                     });
//                 }
//             }
//         }
//     }

//     cycles
// }

// /// Finds all 3-swap cycles in the pool network
// #[allow(dead_code)]
// pub fn triangular_cycles(pools: HashSet<Pool>) -> HashSet<Cycle> {
//     let adj = build_adjacency(&pools);
//     let mut cycles = HashSet::new();

//     // Start from each token
//     for start_token in adj.keys() {
//         for mut cycle in node_triangular_cycles(&adj, start_token) {
//             // Rotate to start with lowest token and pool combination
//             let min_idx = cycle
//                 .swaps
//                 .iter()
//                 .enumerate()
//                 .min_by_key(|(_, swap)| {
//                     let from = if swap.to == swap.pool.token1 {
//                         &swap.pool.token0
//                     } else {
//                         &swap.pool.token1
//                     };
//                     (from, &swap.pool.address)
//                 })
//                 .map(|(i, _)| i)
//                 .unwrap_or(0);
//             cycle.swaps.rotate_left(min_idx);

//             cycles.insert(cycle);
//         }
//     }

//     cycles
// }

// #[cfg(test)]
// pub mod tests {
//     use super::*;

//     #[test]
//     fn test_reciprocal_cycle() {
//         let pools = helpers::generate_graph(HashSet::from([("P1", "A", "B", 100, 200)]));
//         let cycles = reciprocal_cycles(pools);
//         assert_eq!(cycles.len(), 1);
//         let cycle = cycles.iter().next().unwrap();
//         assert_eq!(
//             helpers::list_cycle(cycle),
//             "P1: A->B @1.9940, P1: B->A @0.4985"
//         );
//     }

//     #[test]
//     fn test_two_reciprocal_cycles_in_different_pools() {
//         let pools = helpers::generate_graph(HashSet::from([
//             ("P1", "A", "B", 100, 200),
//             ("P2", "A", "B", 100, 200),
//         ]));
//         let cycles = reciprocal_cycles(pools);

//         assert_eq!(cycles.len(), 4);

//         // Check all cycles
//         let cycles_str: HashSet<_> = cycles.iter().map(helpers::list_cycle).collect();
//         let expected: HashSet<_> = HashSet::from([
//             "P1: A->B @1.9940, P1: B->A @0.4985".to_string(),
//             "P2: A->B @1.9940, P2: B->A @0.4985".to_string(),
//             "P1: A->B @1.9940, P2: B->A @0.4985".to_string(),
//             "P2: A->B @1.9940, P1: B->A @0.4985".to_string(),
//         ]);
//         assert_eq!(cycles_str, expected);
//     }

//     #[test]
//     fn test_triangular_cycles() {
//         let pools = helpers::generate_graph(
//             vec![
//                 ("P1", "A", "B", 1000, 2000),
//                 ("P2", "B", "C", 1000, 1000),
//                 ("P3", "C", "A", 1000, 400),
//             ]
//             .into_iter()
//             .collect(),
//         );
//         let cycles = triangular_cycles(pools);

//         assert_eq!(cycles.len(), 2); // 2 cycles in the opposite direction

//         let cycles_str: HashSet<_> = cycles.iter().map(helpers::list_cycle).collect();

//         let expected: HashSet<_> = HashSet::from([
//             "P1: A->B @1.9940, P2: B->C @0.9970, P3: C->A @0.3988".to_string(),
//             "P3: A->C @2.4925, P2: C->B @0.9970, P1: B->A @0.4985".to_string(),
//         ])
//         .into_iter()
//         .collect();

//         assert_eq!(cycles_str, expected);
//     }

//     pub mod helpers {
//         use super::*;

//         // Helper function to generate a graph from a list of pools
//         pub fn generate_graph(pools: HashSet<(&str, &str, &str, u64, u64)>) -> HashSet<Pool> {
//             pools
//                 .iter()
//                 .map(|(address, token0, token1, reserve0, reserve1)| Pool {
//                     address: address.to_string(),
//                     token0: token0.to_string(),
//                     token1: token1.to_string(),
//                     reserve0: *reserve0,
//                     reserve1: *reserve1,
//                 })
//                 .collect()
//         }

//         /// Formats a cycle as a string for easy debugging
//         /// Example: "P1: A->B @1.9940, P1: B->A @0.4985"
//         pub fn list_cycle(cycle: &Cycle) -> String {
//             cycle
//                 .swaps
//                 .iter()
//                 .map(|swap| {
//                     let from = if swap.to == swap.pool.token1 {
//                         &swap.pool.token0
//                     } else {
//                         &swap.pool.token1
//                     };
//                     format!(
//                         "{}: {}->{} @{:.4}",
//                         swap.pool.address, from, swap.to, swap.rate
//                     )
//                 })
//                 .collect::<Vec<_>>()
//                 .join(", ")
//         }
//     }
// }
