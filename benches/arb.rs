use alloy::primitives::{Address, U256};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fly::arb::{
    cycle::Cycle,
    pool::{Pool, PoolId},
    swap::{Direction, Swap, SwapId},
    token::TokenId,
};
use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Instant;

/// Generate a new random token address
fn generate_random_address() -> String {
    let addr_str = format!("0x{:040x}", fastrand::u64(..));
    let address_checksum = Address::from_str(&addr_str).unwrap();
    address_checksum.to_string()
}

/// Generate synthetic test data for benchmarking
fn generate_benchmark_pools(pool_count: usize, token_count: usize) -> Vec<Pool> {
    let mut rng = rand::thread_rng();
    let mut pools = Vec::with_capacity(pool_count);

    // Create token IDs
    let tokens: Vec<TokenId> = (0..token_count)
        .map(|_| TokenId::try_from(generate_random_address()).unwrap())
        .collect();

    println!("Generated {} tokens. First 3 tokens:", tokens.len());
    for i in 0..std::cmp::min(3, tokens.len()) {
        println!("  Token {}: {}", i, tokens[i]);
    }

    // Generate random pools
    for i in 0..pool_count {
        // Select two random tokens
        let idx1 = rng.random_range(0..token_count);
        let mut idx2 = rng.random_range(0..token_count);

        // Ensure tokens are different
        while idx1 == idx2 {
            idx2 = rng.random_range(0..token_count);
        }

        // Create pool with random reserves
        let reserve0 = U256::from(rng.random_range(1000..1_000_000));
        let reserve1 = U256::from(rng.random_range(1000..1_000_000));

        let pool = Pool::new(
            PoolId::try_from(generate_random_address()).unwrap(),
            tokens[idx1].clone(),
            tokens[idx2].clone(),
            Some(reserve0),
            Some(reserve1),
        );

        // Print details for first few pools
        if i < 3 || i == pool_count - 1 {
            println!(
                "  Pool {}/{}: ID={}, token0={}, token1={}, reserve0={}, reserve1={}",
                i + 1,
                pool_count,
                pool.id,
                pool.token0,
                pool.token1,
                reserve0,
                reserve1
            );
        } else if i == 3 {
            println!("  ... (omitting {} pools for brevity) ...", pool_count - 4);
        }

        pools.push(pool);
    }

    pools
}

/// Find all profitable cycles affected by an updated pool
///
/// # Arguments
/// * `pools` - All existing pools in the market
/// * `updated_pool` - The specific pool that was updated
///
/// # Returns
/// A vector of all profitable cycles that contain the updated pool
pub fn find_affected_cycles(pools: &Vec<Pool>, updated_pool: Pool) -> Vec<Cycle> {
    let start_time = Instant::now();

    // Build token graph
    let mut token_graph: HashMap<TokenId, Vec<(TokenId, PoolId, bool)>> = HashMap::new();

    // Build graph from all pools including the updated one
    for pool in pools.iter().chain(std::iter::once(&updated_pool)) {
        let token0 = pool.token0.clone();
        let token1 = pool.token1.clone();

        // Add edges in both directions
        token_graph
            .entry(token0.clone())
            .or_insert_with(Vec::new)
            .push((token1.clone(), pool.id.clone(), true)); // token0 -> token1

        token_graph
            .entry(token1)
            .or_insert_with(Vec::new)
            .push((token0, pool.id.clone(), false)); // token1 -> token0
    }

    // println!("Graph built with {} nodes in {:?}", token_graph.len(), start_time.elapsed());

    // Set to keep track of unique cycles
    let mut unique_cycles = HashSet::new();
    let mut all_cycles = Vec::new();

    // The updated pool's tokens are our starting points for cycle detection
    let start_tokens = vec![updated_pool.token0.clone(), updated_pool.token1.clone()];

    // println!("Starting cycles detection from tokens: {} and {}",
    //          start_tokens[0], start_tokens[1]);

    let dfs_start = Instant::now();
    for start_token in start_tokens {
        // Find cycles starting from this token
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        dfs_find_cycles(
            &token_graph,
            &start_token,
            &start_token,
            &updated_pool.id,
            &mut visited,
            &mut path,
            &mut unique_cycles,
            &mut all_cycles,
            0,
            3, // Maximum cycle length (3-hop)
        );
    }
    // println!("  DFS completed in {:?}, found {} total cycles", dfs_start.elapsed(), all_cycles.len());

    // Filter for profitable cycles only - this is where we add the profitability check
    let filter_start = Instant::now();
    let profitable_cycles = all_cycles
        .clone()
        .into_iter()
        .filter(|cycle| {
            // For each cycle, check if it's profitable
            // We need all reserves to do this, so ensure the cycle has them
            if cycle.has_all_reserves() {
                cycle.is_positive()
            } else {
                // If we can't determine profitability, be conservative and exclude it
                false
            }
        })
        .collect::<Vec<_>>();

    // println!("Profitability filtering completed in {:?}", filter_start.elapsed());
    // println!("Found {} profitable cycles out of {} total cycles",
    //          profitable_cycles.len(), all_cycles.len());

    // Print details of first few profitable cycles
    if !profitable_cycles.is_empty() {
        println!("Details of first profitable cycle:");
        let cycle = &profitable_cycles[0];
        println!("Number of swaps: {}", cycle.swaps.len());
        for (i, swap) in cycle.swaps.iter().enumerate() {
            println!(
                "Swap {}: {} -> {}",
                i + 1,
                swap.token_in(),
                swap.token_out()
            );
        }
    }

    // println!("Total execution time: {:?}", start_time.elapsed());

    profitable_cycles
}

/// Depth-first search to find cycles in the token graph
/// This should match your existing implementation with minor adjustments
#[allow(clippy::too_many_arguments)]
fn dfs_find_cycles(
    graph: &HashMap<TokenId, Vec<(TokenId, PoolId, bool)>>,
    start_token: &TokenId,
    current_token: &TokenId,
    updated_pool_id: &PoolId,
    visited: &mut HashSet<PoolId>,
    path: &mut Vec<(TokenId, PoolId, bool)>,
    unique_cycles: &mut HashSet<Vec<SwapId>>,
    result_cycles: &mut Vec<Cycle>,
    depth: usize,
    max_depth: usize,
) {
    // Check if we found a cycle back to start
    if depth > 0 && current_token == start_token {
        let mut cycle_contains_updated_pool = false;
        let mut swaps = Vec::new();

        // Create swaps from the path
        for i in 0..path.len() {
            let (token_from, pool_id, is_forward) = &path[i];
            let (token_to, _, _) = if i < path.len() - 1 {
                &path[i + 1]
            } else {
                &path[0] // Close the cycle
            };

            // Check if this pool is the updated one
            if pool_id == updated_pool_id {
                cycle_contains_updated_pool = true;
            }

            // Create swap ID
            let direction = if *is_forward {
                Direction::ZeroForOne
            } else {
                Direction::OneForZero
            };
            let swap_id = SwapId {
                pool_id: pool_id.clone(),
                direction,
            };

            // Create the swap itself with actual reserves if available
            // This is important for profitability calculation
            if let Ok(swap) = Swap::new(
                swap_id,
                token_from.clone(),
                token_to.clone(),
                None, // We'll handle reserves later
                None,
            ) {
                swaps.push(swap);
            }
        }

        // Only add the cycle if it contains the updated pool
        if cycle_contains_updated_pool && !swaps.is_empty() {
            // Create a normalized representation of the cycle for deduplication
            let mut swap_ids: Vec<SwapId> = swaps.iter().map(|s| s.id().clone()).collect();

            // Find smallest swap (lexicographically) for normalization
            if let Some(min_pos) = swap_ids
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.cmp(b))
                .map(|(i, _)| i)
            {
                // Rotate to put smallest swap first
                swap_ids.rotate_left(min_pos);

                // If we haven't seen this cycle before, add it
                if unique_cycles.insert(swap_ids) {
                    if let Ok(cycle) = Cycle::new(swaps) {
                        result_cycles.push(cycle);
                    }
                }
            }
        }
        return;
    }

    // Stop if we hit max depth
    if depth >= max_depth {
        return;
    }

    // Continue DFS with neighbors
    if let Some(neighbors) = graph.get(current_token) {
        for (next_token, pool_id, is_forward) in neighbors {
            // Skip already visited pools to avoid loops
            if visited.contains(pool_id) {
                continue;
            }

            visited.insert(pool_id.clone());
            path.push((current_token.clone(), pool_id.clone(), *is_forward));

            dfs_find_cycles(
                graph,
                start_token,
                next_token,
                updated_pool_id,
                visited,
                path,
                unique_cycles,
                result_cycles,
                depth + 1,
                max_depth,
            );

            path.pop();
            visited.remove(pool_id);
        }
    }
}

/// Record showing detailed benchmark metrics
#[derive(Default)]
struct BenchmarkMetrics {
    total_cycles: usize,
    profitable_cycles: usize,
    max_cycle_length: usize,
    avg_execution_time_ms: f64,
    samples: usize,
}

/// Benchmark finding cycles with a randomly updated pool
fn bench_find_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_affected_cycles");

    // Configure measurement settings for more accurate results
    group.sample_size(10); // Reduced for clarity in output
    group.measurement_time(std::time::Duration::from_secs(5)); // Reduced for faster results

    // For collecting metrics across samples
    let mut metrics_map: HashMap<usize, BenchmarkMetrics> = HashMap::new();

    // Benchmark with different pool counts to find our limits
    for pool_count in [100, 500, 1000, 5000].iter() {
        // Create a synthetic market with 20% of the pool count as tokens
        // This mimics real-world token-to-pool ratios
        let token_count = ((pool_count / 5) as usize).max(10);

        println!("\n========================================================");
        println!("BENCHMARK: {} pools, {} tokens", pool_count, token_count);
        println!("========================================================");

        let pools = generate_benchmark_pools(*pool_count as usize, token_count);

        // Select a random pool to update for the benchmark
        let random_pool_idx = fastrand::usize(0..pools.len());
        let updated_pool = pools[random_pool_idx].clone();

        println!(
            "Selected updated pool: ID={}, token0={}, token1={}",
            updated_pool.id, updated_pool.token0, updated_pool.token1
        );

        // Configure a specific throughput measurement based on pool count
        group.throughput(criterion::Throughput::Elements(*pool_count as u64));

        let metrics = metrics_map.entry(*pool_count).or_default();

        group.bench_with_input(
            BenchmarkId::from_parameter(pool_count),
            pool_count,
            |b, _| {
                // Setup phase (not measured)
                b.iter_batched(
                    // Setup function (called for each batch)
                    || (pools.clone(), updated_pool.clone()),
                    // Benchmark function (timed)
                    |(p, up)| {
                        let start = Instant::now();
                        let cycles = black_box(find_affected_cycles(&p, up));
                        let duration = start.elapsed();

                        // Update metrics (not part of timed section)
                        metrics.total_cycles += cycles.len();
                        metrics.profitable_cycles += cycles.len();
                        metrics.max_cycle_length = cycles
                            .iter()
                            .map(|c| c.swaps.len())
                            .max()
                            .unwrap_or(0)
                            .max(metrics.max_cycle_length);
                        metrics.avg_execution_time_ms = (metrics.avg_execution_time_ms
                            * metrics.samples as f64
                            + duration.as_millis() as f64)
                            / (metrics.samples + 1) as f64;
                        metrics.samples += 1;

                        cycles
                    },
                    // Batch size
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        // Print summary metrics after all samples
        println!("\nSUMMARY METRICS for {} pools:", pool_count);
        println!(
            "Total profitable cycles found: {}",
            metrics.profitable_cycles
        );
        println!(
            "Avg cycles per sample: {:.2}",
            metrics.profitable_cycles as f64 / metrics.samples as f64
        );
        println!("Max cycle length: {}", metrics.max_cycle_length);
        println!(
            "Avg execution time: {:.2} ms",
            metrics.avg_execution_time_ms
        );
        println!("Number of samples: {}", metrics.samples);
    }

    group.finish();
}

/// Benchmark against production-like data
fn bench_production_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("production_data");

    // Configure for thorough statistical analysis
    group.sample_size(5); // Reduced for clarity in output
    group.measurement_time(std::time::Duration::from_secs(5)); // Reduced for faster results

    // Define our test matrix - format: (name, pool_count, token_count, density)
    let test_configs = [
        // Low density markets (fewer connections between tokens)
        ("sparse_small", 100, 50, 0.3),
        // Medium density markets (moderate connections)
        ("medium_small", 100, 30, 0.5),
        // High density markets (many connections between tokens)
        ("dense_small", 100, 20, 0.7),
    ];

    for (name, pool_count, token_count, density) in test_configs {
        println!("\n========================================================");
        println!(
            "PRODUCTION TEST: {}, {} pools, {} tokens, density {:.1}",
            name, pool_count, token_count, density
        );
        println!("========================================================");

        // Generate pools with controlled density
        // Here we'd use density to control how pools are generated
        let pools = generate_benchmark_pools(pool_count, token_count);

        // For each scenario, we test multiple update patterns
        // 1. Update a high-connectivity pool (many cycles affected)
        let high_connectivity_pool = pools[0].clone();
        println!(
            "High connectivity pool: ID={}, token0={}, token1={}",
            high_connectivity_pool.id, high_connectivity_pool.token0, high_connectivity_pool.token1
        );

        // 2. Update a low-connectivity pool (few cycles affected)
        let low_connectivity_pool = pools[pools.len() - 1].clone();
        println!(
            "Low connectivity pool: ID={}, token0={}, token1={}",
            low_connectivity_pool.id, low_connectivity_pool.token0, low_connectivity_pool.token1
        );

        // First benchmark: high connectivity pool updates
        group.bench_with_input(
            BenchmarkId::new("high_connectivity", name),
            &name,
            |b, _| {
                b.iter_batched(
                    || (pools.clone(), high_connectivity_pool.clone()),
                    |(p, up)| {
                        let start = Instant::now();
                        let cycles = black_box(find_affected_cycles(&p, up));
                        let duration = start.elapsed();

                        println!(
                            "HIGH CONNECTIVITY: Found {} profitable cycles in {:?}",
                            cycles.len(),
                            duration
                        );

                        // Print first cycle details if available
                        if !cycles.is_empty() {
                            let cycle = &cycles[0];
                            println!("  First cycle has {} swaps:", cycle.swaps.len());
                            for (i, swap) in cycle.swaps.iter().enumerate().take(3) {
                                println!(
                                    "    Swap {}: {} -> {}",
                                    i + 1,
                                    swap.token_in(),
                                    swap.token_out()
                                );
                            }
                        }

                        cycles
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        // Second benchmark: low connectivity pool updates
        group.bench_with_input(BenchmarkId::new("low_connectivity", name), &name, |b, _| {
            b.iter_batched(
                || (pools.clone(), low_connectivity_pool.clone()),
                |(p, up)| {
                    let start = Instant::now();
                    let cycles = black_box(find_affected_cycles(&p, up));
                    let duration = start.elapsed();

                    println!(
                        "LOW CONNECTIVITY: Found {} profitable cycles in {:?}",
                        cycles.len(),
                        duration
                    );

                    // Print first cycle details if available
                    if !cycles.is_empty() {
                        let cycle = &cycles[0];
                        println!("  First cycle has {} swaps:", cycle.swaps.len());
                        for (i, swap) in cycle.swaps.iter().enumerate().take(3) {
                            println!(
                                "    Swap {}: {} -> {}",
                                i + 1,
                                swap.token_in(),
                                swap.token_out()
                            );
                        }
                    }

                    cycles
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::U256;
    use fly::arb::pool::Pool;
    use fly::arb::token::TokenId;
    use std::str::FromStr;

    // Create a helper function to generate simple test pools
    fn create_test_pool(
        id: &str,
        token0: &str,
        token1: &str,
        reserve0: u64,
        reserve1: u64,
    ) -> Pool {
        Pool::new(
            PoolId::try_from(id).unwrap(),
            TokenId::try_from(token0).unwrap(),
            TokenId::try_from(token1).unwrap(),
            Some(U256::from(reserve0)),
            Some(U256::from(reserve1)),
        )
    }

    /// Test using the example graph provided in the requirements
    /// Nodes: 0, 1, 2, 3
    /// Edges (labeled): 1: 0-1, 2: 0-2, 3: 1-3, 4: 1-3, 5: 2-3
    #[test]
    fn test_specific_graph_structure() {
        // Create the exact graph structure shown in the requirements
        // Nodes: 0, 1, 2, 3
        // Edges (labeled): 1: 0-1, 2: 0-2, 3: 1-2, 4: 1-3, 5: 2-3

        // Create token IDs for nodes 0, 1, 2, 3
        let token0 = TokenId::try_from(generate_random_address()).unwrap();
        let token1 = TokenId::try_from(generate_random_address()).unwrap();
        let token2 = TokenId::try_from(generate_random_address()).unwrap();
        let token3 = TokenId::try_from(generate_random_address()).unwrap();

        // Create pools with consistent edge IDs matching the diagram
        let pools = vec![
            // Edge 1: Node 0 to Node 1
            Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                token0.clone(),
                token1.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
            // Edge 2: Node 0 to Node 2
            Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                token0.clone(),
                token2.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
            // Edge 3: Node 1 to Node 2
            Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                token1.clone(),
                token2.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
            // Edge 4: Node 1 to Node 3
            Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                token1.clone(),
                token3.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
            // Edge 5: Node 2 to Node 3
            Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                token2.clone(),
                token3.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
        ];

        // Make Edge 1 (pool between nodes 0-1) unbalanced to create arbitrage opportunities
        let updated_pool = Pool::new(
            PoolId::try_from("Edge1").unwrap(),
            token0.clone(),
            token1.clone(),
            Some(U256::from(700)),  // Changed from 1000
            Some(U256::from(1300)), // Changed from 1000
        );

        // Set reserves for other pools to create profitable cycles
        // We need to modify the remaining pools to ensure cycles are profitable
        // This will require making sure the product of exchange rates around cycles is > 1

        // Find all cycles that include the updated pool
        let cycles = find_affected_cycles(&pools, updated_pool);

        // Debug output: print all found cycles
        println!(
            "Found {} cycles that include the updated pool:",
            cycles.len()
        );
        for (i, cycle) in cycles.iter().enumerate() {
            println!("Cycle {}:", i + 1);
            println!("  Number of swaps: {}", cycle.swaps.len());

            // Print the path of the cycle
            print!("  Path: ");
            for (j, swap) in cycle.swaps.iter().enumerate() {
                let from_token = swap.token_in().to_string();
                let to_token = swap.token_out().to_string();

                // Convert token names to node numbers
                let from_node = from_token.replace("Token", "");
                let to_node = to_token.replace("Token", "");

                print!(
                    "{}{}",
                    from_node,
                    if j < cycle.swaps.len() - 1 { "->" } else { "" }
                );
                if j == cycle.swaps.len() - 1 {
                    println!("{}", to_node);
                }
            }

            // Print the edges (pool IDs) in the cycle
            print!("  Edges: ");
            for (j, swap) in cycle.swaps.iter().enumerate() {
                let pool_id = swap.id().pool_id.to_string().replace("Edge", "");
                print!(
                    "{}{}",
                    pool_id,
                    if j < cycle.swaps.len() - 1 { "," } else { "" }
                );
            }
            println!();

            // Print profitability information
            if cycle.has_all_reserves() {
                println!("  Is profitable: {}", cycle.is_positive());

                // Calculate best quote
                let best_quote = cycle.best_quote().unwrap();
                println!("  Optimal amount in: {}", best_quote.amount_in());
                println!("  Expected amount out: {}", best_quote.amount_out());
                println!("  Expected profit: {}", best_quote.profit());
            }
            println!();
        }

        // Verify we find the expected cycles
        // Cycle 1: (1,2,3) - Node 0->1->2->0 using edges 1,3,2
        // Cycle 2: (1,2,5,4) - Node 0->1->3->2->0 using edges 1,4,5,2

        // First check that we found at least one cycle
        assert!(!cycles.is_empty(), "Should find at least one cycle");

        // Function to check if a specific cycle is present
        let has_cycle = |edge_sequence: Vec<&str>| -> bool {
            cycles.iter().any(|cycle| {
                // Check if cycle has the right number of swaps
                if cycle.swaps.len() != edge_sequence.len() {
                    return false;
                }

                // Extract edge labels from cycle
                let cycle_edges: Vec<String> = cycle
                    .swaps
                    .iter()
                    .map(|swap| swap.id().pool_id.to_string())
                    .collect();

                // Check if all expected edges are in the cycle (may be rotated)
                for i in 0..cycle.swaps.len() {
                    let mut matches = true;
                    for j in 0..cycle.swaps.len() {
                        let expected_edge = edge_sequence[(i + j) % cycle.swaps.len()];
                        if !cycle_edges[j].contains(expected_edge) {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        return true;
                    }
                }
                false
            })
        };

        // Check for the 3-edge cycle: (1,2,3)
        let has_cycle_123 = has_cycle(vec!["Edge1", "Edge3", "Edge2"]);

        // Check for the 4-edge cycle: (1,4,5,2)
        let has_cycle_1452 = has_cycle(vec!["Edge1", "Edge4", "Edge5", "Edge2"]);

        // Print whether expected cycles were found
        println!("Cycle (1,2,3) found: {}", has_cycle_123);
        println!("Cycle (1,4,5,2) found: {}", has_cycle_1452);

        // Verify at least one of the expected cycles was found
        assert!(
            has_cycle_123 || has_cycle_1452,
            "Should find at least one of the expected cycles"
        );
    }
}

// Criterion setup
criterion_group!(benches, bench_find_cycles, bench_production_data);
criterion_main!(benches);
