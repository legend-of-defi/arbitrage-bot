//! # Arbitrage Detection Benchmarks
//!
//! This module provides benchmarks for the cycle detection and arbitrage opportunity
//! identification algorithms. It tests performance across various market conditions:
//!
//! - Different market sizes (number of pools and tokens)
//! - Various levels of connectivity between tokens
//! - Performance impact of pool location in the graph
//! - Efficiency of cycle detection with different arbitrage opportunities
//!
//! The benchmarks use synthetic data designed to simulate real-world market conditions
//! while providing controlled test cases for reproducible performance measurement.

#![allow(missing_docs)]

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

/// Helper function to get log_rate from a pool
fn get_log_rate(pool: &Pool) -> i64 {
    // Create a temporary swap to get the log_rate
    let swap_id = SwapId {
        pool_id: pool.id.clone(),
        direction: Direction::ZeroForOne,
    };

    match Swap::new(
        swap_id,
        pool.token0.clone(),
        pool.token1.clone(),
        pool.reserve0,
        pool.reserve1,
    ) {
        Ok(swap) => swap.log_rate(),
        Err(_) => 0,
    }
}

/// Generate synthetic test data for benchmarking
fn generate_benchmark_pools(pool_count: usize, token_count: usize) -> (Vec<Pool>, Pool) {
    let mut rng = rand::rng();
    let mut pools = Vec::with_capacity(pool_count);

    // Create token IDs
    let tokens: Vec<TokenId> = (0..token_count)
        .map(|_| TokenId::try_from(generate_random_address()).unwrap())
        .collect();

    println!("Generated {} tokens. First 3 tokens:", tokens.len());
    for i in 0..std::cmp::min(3, tokens.len()) {
        println!("Token {}: {}", i, tokens[i]);
    }

    // Generate random pools
    for _i in 0..pool_count {
        // Select two random tokens
        let idx1 = rng.random_range(0..token_count);
        let mut idx2 = rng.random_range(0..token_count);

        while idx1 == idx2 {
            idx2 = rng.random_range(0..token_count);
        }

        // Create pool with potentially imbalanced reserves
        let reserve0 = U256::from(rng.random_range(1000..10_000_000));
        let reserve1 = if rng.random_bool(0.3) {
            // 30% chance of imbalanced pool
            U256::from(rng.random_range(
                1000_000_000_000_000_000_000_u128..1000_000_000_000_000_000_000_000_000_000_u128,
            )) // Much smaller reserve
        } else {
            U256::from(rng.random_range(
                1000_000_000_000_000_000_000_u128..1000_000_000_000_000_000_000_000_000_000_u128,
            ))
        };

        let pool = Pool::new(
            PoolId::try_from(generate_random_address()).unwrap(),
            tokens[idx1].clone(),
            tokens[idx2].clone(),
            Some(reserve0),
            Some(reserve1),
        );

        pools.push(pool);
    }

    // Create a few circular arbitrage opportunities and prepare an updated pool
    let token_a = tokens[0].clone();
    let token_b = tokens[1].clone();
    let token_c = tokens[2].clone();
    let updated_pool;

    if pool_count >= 10 && token_count >= 4 {
        // Create a simple 3-token cycle (A->B->C->A) with profitable arbitrage
        if pools.len() >= 3 {
            // Create three connected pools with initially balanced reserves
            pools[0] = Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                token_a.clone(),
                token_b.clone(),
                Some(U256::from(1_000_000_000_000_000_u128)),
                Some(U256::from(3_000_000_000_000_000_000_u128)),
            );

            pools[1] = Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                token_b.clone(),
                token_c.clone(),
                Some(U256::from(1_000_000_000_000_000_u128)),
                Some(U256::from(3_000_000_000_000_000_000_u128)),
            );

            pools[2] = Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                token_c.clone(),
                token_a.clone(),
                Some(U256::from(1_000_000_000_000_000_u128)),
                Some(U256::from(3_000_000_000_000_000_000_u128)),
            );

            // Create an updated version of the C-A pool (pools[2]) with significant imbalance
            updated_pool = Pool::new(
                pools[2].id.clone(),
                token_c.clone(),
                token_a.clone(),
                Some(U256::from(1_000_000_000_000_000_u128)), // Imbalanced reserve
                Some(U256::from(7_000_000_000_000_000_000_u128)), // Imbalanced reserve
            );

            println!("Created arbitrage cycle between tokens:");
            println!("A: {}", token_a);
            println!("B: {}", token_b);
            println!("C: {}", token_c);
            println!(
                "Pool IDs: {}, {}, {}",
                pools[0].id, pools[1].id, pools[2].id
            );
            println!("Updated pool ID: {} with imbalanced reserves: token0={}, token1={}, reserve0={}, reserve1={}, log_rate={:.8}", 
                    updated_pool.id,
                    updated_pool.token0,
                    updated_pool.token1,
                    updated_pool.reserve0.unwrap_or_default(),
                    updated_pool.reserve1.unwrap_or_default(),
                    get_log_rate(&updated_pool));
        } else {
            // Fallback if we don't have enough pools
            updated_pool = pools[0].clone();
        }
    } else {
        // Fallback if we don't have enough tokens or pools
        updated_pool = pools[0].clone();
    }

    (pools, updated_pool)
}

/// Find all profitable cycles affected by an updated pool
///
/// # Arguments
/// * `pools` - All existing pools in the market
/// * `updated_pool` - The specific pool that was updated
///
/// # Returns
/// A vector of all profitable cycles that contain the updated pool
pub fn find_affected_cycles(pools: &[Pool], updated_pool: Pool) -> Vec<Cycle> {
    let _start_time = Instant::now();

    // Build token graph
    let mut token_graph: HashMap<TokenId, Vec<(TokenId, PoolId, bool)>> = HashMap::new();

    // Create a pool lookup to store reserves and token mapping
    let mut pool_lookup: HashMap<PoolId, (TokenId, TokenId, Option<U256>, Option<U256>)> =
        HashMap::new();

    // Build graph from all pools including the updated one
    for pool in pools.iter().chain(std::iter::once(&updated_pool)) {
        let token0 = pool.token0.clone();
        let token1 = pool.token1.clone();

        // Store pool information for later reserve lookup
        pool_lookup.insert(
            pool.id.clone(),
            (token0.clone(), token1.clone(), pool.reserve0, pool.reserve1),
        );

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

    // Try both methods and use the results from the one that finds cycles
    let dfs_cycles = find_cycles_dfs(&token_graph, &updated_pool, &pool_lookup);
    let bellman_ford_cycles = find_cycles_bellman_ford(&token_graph, &updated_pool, &pool_lookup);
    
    // Print debug info for both methods
    // println!("DFS found {} cycles", dfs_cycles.len());
    // println!("Bellman-Ford found {} cycles", bellman_ford_cycles.len());
    
    // If both find cycles, prefer Bellman-Ford as it's more likely to find all profitable cycles
    if !bellman_ford_cycles.is_empty() {
        bellman_ford_cycles
    } else {
        dfs_cycles
    }
    // dfs_cycles
}

/// Find cycles using DFS - our original approach but with fixed reserve handling
fn find_cycles_dfs(
    token_graph: &HashMap<TokenId, Vec<(TokenId, PoolId, bool)>>,
    updated_pool: &Pool,
    pool_lookup: &HashMap<PoolId, (TokenId, TokenId, Option<U256>, Option<U256>)>,
) -> Vec<Cycle> {
    // Set to keep track of unique cycles
    let mut unique_cycles = HashSet::new();
    let mut all_cycles = Vec::new();

    // The updated pool's tokens are our starting points for cycle detection
    let start_tokens = vec![updated_pool.token0.clone(), updated_pool.token1.clone()];

    for start_token in start_tokens {
        // Find cycles starting from this token
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        dfs_find_cycles(
            token_graph,
            &start_token,
            &start_token,
            &updated_pool.id,
            &mut visited,
            &mut path,
            &mut unique_cycles,
            &mut all_cycles,
            0,
            3,            // Maximum cycle length (3-hop)
            pool_lookup, 
        );
    }

    // Filter for profitable cycles
    let profitable_cycles = all_cycles
        .iter()
        .filter(|cycle| cycle.has_all_reserves() && cycle.is_positive())
        .cloned()
        .collect::<Vec<_>>();

    // Debug output for profitable cycles
    // println!("DFS found {} cycles, {} are profitable", all_cycles.len(), profitable_cycles.len());
    
    for (i, cycle) in profitable_cycles.iter().enumerate().take(3) {
        // println!("DFS Profitable cycle {}: {} swaps, log_rate: {}",
        //         i + 1, 
        //         cycle.swaps.len(),
        //         cycle.swaps.iter().map(|s| s.log_rate()).sum::<i64>());
    }

    profitable_cycles
}

/// Find profitable cycles using Modified Bellman-Ford algorithm
fn find_cycles_bellman_ford(
    token_graph: &HashMap<TokenId, Vec<(TokenId, PoolId, bool)>>,
    updated_pool: &Pool,
    pool_lookup: &HashMap<PoolId, (TokenId, TokenId, Option<U256>, Option<U256>)>,
) -> Vec<Cycle> {
    // Map tokens to indices for the algorithm
    let mut token_to_index = HashMap::new();
    let mut index_to_token = HashMap::new();
    
    // Collect all unique tokens
    let mut index = 0;
    for token in token_graph.keys() {
        if !token_to_index.contains_key(token) {
            token_to_index.insert(token.clone(), index);
            index_to_token.insert(index, token.clone());
            index += 1;
        }
    }
    
    let num_tokens = token_to_index.len();
    if num_tokens == 0 {
        return Vec::new();
    }
    
    // Store edges as (from_idx, to_idx, weight, swap)
    let mut edges = Vec::new();
    // For faster lookup: (from_idx, to_idx) -> edge_idx
    let mut edge_map: HashMap<(usize, usize), usize> = HashMap::new();
    
    // Build edges with weights
    for (from_token, neighbors) in token_graph {
        let from_idx = *token_to_index.get(from_token).unwrap();
        
        for (to_token, pool_id, is_forward) in neighbors {
            let to_idx = *token_to_index.get(to_token).unwrap();
            
            // Get pool data
            if let Some((token0, token1, reserve0, reserve1)) = pool_lookup.get(pool_id) {
                let swap_id = SwapId {
                    pool_id: pool_id.clone(),
                    direction: if *is_forward { Direction::ZeroForOne } else { Direction::OneForZero },
                };
                
                // Create swap with correct reserves
                if let Ok(swap) = Swap::new(
                    swap_id,
                    from_token.clone(),
                    to_token.clone(),
                    if *is_forward { *reserve0 } else { *reserve1 },
                    if *is_forward { *reserve1 } else { *reserve0 },
                ) {
                    // Get log_rate, use negative for Bellman-Ford
                    if let log_rate = swap.log_rate() {
                        let edge_idx = edges.len();
                        edges.push((from_idx, to_idx, -log_rate, swap));
                        edge_map.insert((from_idx, to_idx), edge_idx);
                    }
                }
            }
        }
    }
    
    // Results storage
    let mut profitable_cycles = Vec::new();
    
    // Only run from the tokens in the updated pool
    let start_tokens = vec![
        *token_to_index.get(&updated_pool.token0).unwrap(),
        *token_to_index.get(&updated_pool.token1).unwrap()
    ];

    for source in start_tokens {
        // Distance and predecessor arrays
        let mut dist = vec![i64::MAX / 2; num_tokens]; // Avoid overflow
        let mut pred = vec![None; num_tokens];
        
        // Initialize source distance
        dist[source] = 0;
        
        // Relax edges |V|-1 times
        for _ in 0..num_tokens {
            let mut updated = false;
            
            for (u, v, weight, _) in &edges {
                // Try to relax edge
                if dist[*v] > dist[*u] + weight {
                    dist[*v] = dist[*u] + weight;
                    pred[*v] = Some(*u);
                    updated = true;
                }
            }
            
            // If no updates occurred, we've converged
            if !updated {
                break;
            }
        }
        
        // Check for negative cycles affecting the updated pool
        for u in 0..num_tokens {
            // Skip if no path to this node
            if dist[u] == i64::MAX / 2 {
                continue;
            }
            
            // Check if this node is part of a negative cycle
            let mut visited = vec![false; num_tokens];
            let mut stack = Vec::new();
            let mut on_stack = vec![false; num_tokens];
            
            // DFS to find cycles
            fn dfs_find_negative_cycle(
                node: usize,
                edges: &[(usize, usize, i64, Swap)],
                edge_map: &HashMap<(usize, usize), usize>,
                dist: &[i64],
                pred: &[Option<usize>],
                visited: &mut [bool],
                on_stack: &mut [bool],
                stack: &mut Vec<usize>,
                updated_pool_id: &PoolId,
                cycles: &mut Vec<Cycle>,
            ) {
                if visited[node] {
                    return;
                }
                
                visited[node] = true;
                stack.push(node);
                on_stack[node] = true;
                
                // Check neighbors for cycles
                if let Some(prev) = pred[node] {
                    if !visited[prev] {
                        dfs_find_negative_cycle(
                            prev, edges, edge_map, dist, pred, visited, on_stack, stack,
                            updated_pool_id, cycles,
                        );
                    } else if on_stack[prev] {
                        // We found a cycle - reconstruct it
                        let mut cycle_swaps = Vec::new();
                        let mut contains_updated_pool = false;
                        
                        // Find position of prev in stack
                        let cycle_start = stack.iter().position(|&x| x == prev).unwrap();
                        
                        // Reconstruct cycle
                        for i in cycle_start..stack.len() - 1 {
                            let from = stack[i];
                            let to = stack[i + 1];
                            
                            if let Some(&edge_idx) = edge_map.get(&(from, to)) {
                                let (_, _, _, swap) = &edges[edge_idx];
                                cycle_swaps.push(swap.clone());
                                
                                // Check if this swap involves the updated pool
                                if swap.id().pool_id == *updated_pool_id {
                                    contains_updated_pool = true;
                                }
                            }
                        }
                        
                        // Add the final edge to close the cycle
                        let from = stack[stack.len() - 1];
                        let to = prev;
                        
                        if let Some(&edge_idx) = edge_map.get(&(from, to)) {
                            let (_, _, _, swap) = &edges[edge_idx];
                            cycle_swaps.push(swap.clone());
                            
                            if swap.id().pool_id == *updated_pool_id {
                                contains_updated_pool = true;
                            }
                        }
                        
                        // If cycle contains updated pool and is profitable
                        if contains_updated_pool && !cycle_swaps.is_empty() {
                            // Create cycle and check profitability
                            if let Ok(cycle) = Cycle::new(cycle_swaps) {
                                if cycle.has_all_reserves() && cycle.is_positive() {
                                    cycles.push(cycle);
                                }
                            }
                        }
                    }
                }
                
                stack.pop();
                on_stack[node] = false;
            }
            
            // Run DFS from this node
            dfs_find_negative_cycle(
                u, &edges, &edge_map, &dist, &pred, &mut visited, &mut on_stack, &mut stack,
                &updated_pool.id, &mut profitable_cycles,
            );
        }
    }
    
    // Deduplicate cycles
    let mut unique_cycles = HashSet::new();
    let mut unique_profitable_cycles = Vec::new();
    
    for cycle in profitable_cycles {
        let mut swap_ids: Vec<SwapId> = cycle.swaps.iter().map(|s| s.id().clone()).collect();
        
        // Normalize by rotating to smallest swap ID
        if let Some(min_pos) = swap_ids.iter().enumerate()
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(i, _)| i) 
        {
            swap_ids.rotate_left(min_pos);
            
            if unique_cycles.insert(swap_ids) {
                unique_profitable_cycles.push(cycle);
            }
        }
    }
    
    // Debug output for profitable cycles
    // println!("Bellman-Ford found {} profitable cycles", unique_profitable_cycles.len());
    
    for (i, cycle) in unique_profitable_cycles.iter().enumerate().take(3) {
        // println!("Bellman-Ford Profitable cycle {}: {} swaps, log_rate: {}",
        //          i + 1, 
        //          cycle.swaps.len(),
        //          cycle.swaps.iter().map(|s| s.log_rate()).sum::<i64>());
    }
    
    unique_profitable_cycles
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
    pool_lookup: &HashMap<PoolId, (TokenId, TokenId, Option<U256>, Option<U256>)>,
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

            // Get the pool's reserve information from our lookup table
            let (reserve_in, reserve_out) =
                if let Some((token0, token1, reserve0, reserve1)) = pool_lookup.get(pool_id) {
                    if *is_forward {
                        // token0 -> token1 direction
                        (*reserve0, *reserve1)
                    } else {
                        // token1 -> token0 direction
                        (*reserve1, *reserve0)
                    }
                } else {
                    (None, None)
                };

            // Create the swap with the reserve information
            if let Ok(swap) = Swap::new(
                swap_id,
                token_from.clone(),
                token_to.clone(),
                reserve_in,  // Use actual reserves
                reserve_out, // Use actual reserves
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
                pool_lookup,
            );

            path.pop();
            visited.remove(pool_id);
        }
    }
}

/// Record showing detailed benchmark metrics
///
/// Tracks various statistics about cycle detection performance:
/// - Total cycles found
/// - Number of profitable cycles
/// - Maximum cycle length encountered
/// - Average execution time in milliseconds
/// - Number of samples processed
#[derive(Default)]
struct BenchmarkMetrics {
    total_cycles: usize,
    profitable_cycles: usize,
    max_cycle_length: usize,
    avg_execution_time_ms: f64,
    samples: usize,
}

/// Benchmark finding cycles with a randomly updated pool
///
/// Tests the performance of cycle detection across different market sizes:
/// - Varies pool counts (100, 500, 1000, 5000)
/// - Uses realistic token-to-pool ratios (20% tokens to pools)
/// - Measures throughput and execution time
/// - Collects detailed metrics about cycle characteristics
fn bench_find_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_affected_cycles");

    // Configure measurement settings for more accurate results
    group.sample_size(10); // Reduced for clarity in output
    group.measurement_time(std::time::Duration::from_secs(5)); // Reduced for faster results

    // For collecting metrics across samples
    let mut metrics_map: HashMap<usize, BenchmarkMetrics> = HashMap::new();

    // Benchmark with different pool counts to find our limits
    for pool_count in [100, 500, 1000, 5000, 100000, 500000].iter() {
        // Create a synthetic market with 20% of the pool count as tokens
        // This mimics real-world token-to-pool ratios
        let token_count = ((pool_count / 5) as usize).max(10);

        println!("\n========================================================");
        println!("BENCHMARK: {} pools, {} tokens", pool_count, token_count);
        println!("========================================================");

        let (pools, updated_pool) = generate_benchmark_pools(*pool_count as usize, token_count);

        println!(
            "Using updated pool: ID={}, token0={}, token1={}, reserve0={}, reserve1={}, log_rate={:.8}",
            updated_pool.id,
            updated_pool.token0,
            updated_pool.token1,
            updated_pool.reserve0.unwrap_or_default(),
            updated_pool.reserve1.unwrap_or_default(),
            get_log_rate(&updated_pool)
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
///
/// Tests cycle detection under different market conditions:
/// - Sparse markets (low connectivity between tokens)
/// - Medium density markets
/// - Dense markets (high connectivity)
///
/// For each scenario, tests both high and low connectivity pool updates
/// to measure performance impact of pool location in the graph.
fn bench_production_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("production_data");

    // Fix: Ensure minimum required samples for Criterion
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    // Define our test matrix - format: (name, pool_count, token_count)
    let test_configs = [
        // Low density markets (fewer connections between tokens)
        ("sparse_small", 100, 50),
        // Medium density markets (moderate connections)
        ("medium_small", 100, 30),
        // High density markets (many connections between tokens)
        ("dense_small", 100, 20),
    ];

    for (name, pool_count, token_count) in test_configs {
        println!("\n========================================================");
        println!(
            "PRODUCTION TEST: {}, {} pools, {} tokens",
            name, pool_count, token_count
        );
        println!("========================================================");

        // Generate pools with controlled density
        let (pools, base_updated_pool) = generate_benchmark_pools(pool_count, token_count);

        // IMPORTANT: Make sure to use the pool that's part of our arbitrage cycle -
        // we now use the provided updated_pool from generate_benchmark_pools

        // Make the pool even more imbalanced for high connectivity test
        let updated_high_connectivity_pool = Pool::new(
            base_updated_pool.id.clone(),
            base_updated_pool.token0.clone(),
            base_updated_pool.token1.clone(),
            Some(U256::from(1500)), // Extreme imbalance
            Some(U256::from(500)),   // Extreme imbalance
        );

        println!(
            "High connectivity pool: ID={}, token0={}, token1={}, reserve0={}, reserve1={}, log_rate={:.8}",
            updated_high_connectivity_pool.id,
            updated_high_connectivity_pool.token0,
            updated_high_connectivity_pool.token1,
            updated_high_connectivity_pool.reserve0.unwrap_or_default(),
            updated_high_connectivity_pool.reserve1.unwrap_or_default(),
            get_log_rate(&updated_high_connectivity_pool)
        );

        // Use a random pool for low connectivity benchmark - select the last pool to ensure it's not the same as our updated pool
        let low_connectivity_pool = if pools.len() > 3 {
            pools[pools.len() - 1].clone()
        } else {
            // Create a new unrelated pool if we don't have enough pools
            let new_token_id = TokenId::try_from(generate_random_address()).unwrap();
            Pool::new(
                PoolId::try_from(generate_random_address()).unwrap(),
                base_updated_pool.token0.clone(),
                new_token_id,
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            )
        };

        // First benchmark: high connectivity pool updates
        group.bench_with_input(
            BenchmarkId::new("high_connectivity", name),
            &name,
            |b, _| {
                b.iter_batched(
                    || (pools.clone(), updated_high_connectivity_pool.clone()),
                    |(p, up)| {
                        let start = Instant::now();
                        let cycles = black_box(find_affected_cycles(&p, up));
                        let _duration = start.elapsed();

                        // Print first cycle details if available
                        // if !cycles.is_empty() {
                        //     let cycle = &cycles[0];
                        //     for (i, swap) in cycle.swaps.iter().enumerate().take(3) {
                        //         println!(
                        //             "Swap {}: {} -> {}",
                        //             i + 1,
                        //             swap.token_in(),
                        //             swap.token_out()
                        //         );
                        //     }
                        // }

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
                    let _duration = start.elapsed();

                    // Print first cycle details if available
                    // if !cycles.is_empty() {
                    //     let cycle = &cycles[0];
                    //     println!("  First cycle has {} swaps:", cycle.swaps.len());
                    //     for (i, swap) in cycle.swaps.iter().enumerate().take(3) {
                    //         println!(
                    //             "    Swap {}: {} -> {}",
                    //             i + 1,
                    //             swap.token_in(),
                    //             swap.token_out()
                    //         );
                    //     }
                    // }

                    cycles
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

/// Benchmark with specifically crafted arbitrage cycle
fn bench_specific_arbitrage_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("specific_arbitrage_cycle");

    // Configure measurement settings
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    println!("\n========================================================");
    println!("BENCHMARK: Specific Arbitrage Cycle Test");
    println!("========================================================");

    // Create 4 tokens - A, B, C, D for testing
    let token_a = TokenId::try_from(generate_random_address()).unwrap();
    let token_b = TokenId::try_from(generate_random_address()).unwrap();
    let token_c = TokenId::try_from(generate_random_address()).unwrap();
    let token_d = TokenId::try_from(generate_random_address()).unwrap();

    println!("Testing with tokens:");
    println!("  A: {}", token_a);
    println!("  B: {}", token_b);
    println!("  C: {}", token_c);
    println!("  D: {}", token_d);

    // Create the triangular arbitrage pools
    let pool_ab = Pool::new(
        PoolId::try_from(generate_random_address()).unwrap(),
        token_a.clone(),
        token_b.clone(),
        Some(U256::from(1_000_000_000_000_000_u128)),
        Some(U256::from(7_000_000_000_000_000_000_u128)),
    );

    let pool_bc = Pool::new(
        PoolId::try_from(generate_random_address()).unwrap(),
        token_b.clone(),
        token_c.clone(),
        Some(U256::from(1_000_000_000_000_000_u128)),
        Some(U256::from(7_000_000_000_000_000_000_u128)),
    );

    let pool_ca = Pool::new(
        PoolId::try_from(generate_random_address()).unwrap(),
        token_c.clone(),
        token_a.clone(),
        Some(U256::from(1_000_000_000_000_000_u128)), // Initially balanced
        Some(U256::from(7_000_000_000_000_000_000_u128)),
    );

    // Create additional paths
    let pool_bd = Pool::new(
        PoolId::try_from(generate_random_address()).unwrap(),
        token_b.clone(),
        token_d.clone(),
        Some(U256::from(1_000_000_000_000_000_u128)),
        Some(U256::from(7_000_000_000_000_000_000_u128)),
    );

    let pool_cd = Pool::new(
        PoolId::try_from(generate_random_address()).unwrap(),
        token_c.clone(),
        token_d.clone(),
        Some(U256::from(1_000_000_000_000_000_u128)),
        Some(U256::from(7_000_000_000_000_000_000_u128)),
    );

    // Aggregate all pools
    let pools = vec![
        pool_ab.clone(),
        pool_bc.clone(),
        pool_ca.clone(),
        pool_bd.clone(),
        pool_cd.clone(),
    ];

    println!("Created arbitrage cycle with pools:");
    println!("  A-B: {}", pool_ab.id);
    println!("  B-C: {}", pool_bc.id);
    println!("  C-A: {}", pool_ca.id);
    println!("  B-D: {}", pool_bd.id);
    println!("  C-D: {}", pool_cd.id);

    // Test each pool as the updated pool
    let updated_pools = [
        // Pool C-A with increased imbalance to trigger arbitrage
        (
            "C-A (Imbalanced)",
            Pool::new(
                pool_ca.id.clone(),
                token_c.clone(),
                token_a.clone(),
                Some(U256::from(1_300_000_000_000_000_u128)), // Imbalanced reserve
                Some(U256::from(6_500_000_000_000_000_000_u128)), // Imbalanced reserve
            ),
        ),
        // Also test with the A-B pool as updated
        (
            "A-B (Imbalanced)",
            Pool::new(
                pool_ab.id.clone(),
                token_a.clone(),
                token_b.clone(),
                Some(U256::from(1_200_000_000_000_000_u128)), // Imbalanced reserve
                Some(U256::from(8_000_000_000_000_000_000_u128)), // Imbalanced reserve
            ),
        ),
        // Also test with the B-C pool as updated
        (
            "B-C (Imbalanced)",
            Pool::new(
                pool_bc.id.clone(),
                token_b.clone(),
                token_c.clone(),
                Some(U256::from(1_200_000_000_000_000_u128)), // Imbalanced reserve
                Some(U256::from(7_500_000_000_000_000_000_u128)), // Imbalanced reserve
            ),
        ),
    ];

    for (name, updated_pool) in updated_pools.iter() {
        println!("\nTesting with {} as updated pool", name);
        println!("  Updated pool details: ID={}, token0={}, token1={}, reserve0={}, reserve1={}, log_rate={:.8}",
                updated_pool.id,
                updated_pool.token0,
                updated_pool.token1,
                updated_pool.reserve0.unwrap_or_default(),
                updated_pool.reserve1.unwrap_or_default(),
                get_log_rate(updated_pool));

        group.bench_with_input(BenchmarkId::from_parameter(name), name, |b, _| {
            b.iter_batched(
                || (pools.clone(), updated_pool.clone()),
                |(p, up)| {
                    let start = Instant::now();
                    let cycles = black_box(find_affected_cycles(&p, up));
                    let duration = start.elapsed();

                    // println!("Found {} cycles in {:?}", cycles.len(), duration);

                    // Print first cycle details if available
                    // if !cycles.is_empty() {
                    //     let cycle = &cycles[0];
                    //     println!("First cycle has {} swaps, is profitable: {}",
                    //              cycle.swaps.len(), cycle.is_positive());

                    //     println!("Cycle path:");
                    //     for (i, swap) in cycle.swaps.iter().enumerate() {
                    //         println!("  Swap {}: {} -> {}",
                    //                  i + 1,
                    //                  swap.token_in(),
                    //                  swap.token_out());
                    //     }
                    // }

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
    /// Edges (labeled): 1: 0-1, 2: 0-2, 3: 1-2, 4: 1-3, 5: 2-3
    #[test]
    fn test_example_graph() {
        // Create tokens for nodes 0, 1, 2, 3 using random addresses
        let token0 = TokenId::try_from(generate_random_address()).unwrap();
        let token1 = TokenId::try_from(generate_random_address()).unwrap();
        let token2 = TokenId::try_from(generate_random_address()).unwrap();
        let token3 = TokenId::try_from(generate_random_address()).unwrap();

        // Generate random pool IDs for edges 1-5
        let edge1_id = PoolId::try_from(generate_random_address()).unwrap();
        let edge2_id = PoolId::try_from(generate_random_address()).unwrap();
        let edge3_id = PoolId::try_from(generate_random_address()).unwrap();
        let edge4_id = PoolId::try_from(generate_random_address()).unwrap();
        let edge5_id = PoolId::try_from(generate_random_address()).unwrap();

        println!("Testing with randomly generated addresses:");
        println!("Node 0 (token): {}", token0);
        println!("Node 1 (token): {}", token1);
        println!("Node 2 (token): {}", token2);
        println!("Node 3 (token): {}", token3);
        println!("Edge 1 (pool): {}", edge1_id);
        println!("Edge 2 (pool): {}", edge2_id);
        println!("Edge 3 (pool): {}", edge3_id);
        println!("Edge 4 (pool): {}", edge4_id);
        println!("Edge 5 (pool): {}", edge5_id);

        // Create pools with specific reserves to guarantee arbitrage opportunities
        let pools = vec![
            // Edge 1: 0-1 (this will be our updated pool with imbalanced reserves)
            Pool::new(
                edge1_id.clone(),
                token0.clone(),
                token1.clone(),
                Some(U256::from(800)),  // Imbalanced reserve
                Some(U256::from(1200)), // Makes price different from other paths
            ),
            // Edge 2: 0-2
            Pool::new(
                edge2_id.clone(),
                token0.clone(),
                token2.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
            // Edge 3: 1-2
            Pool::new(
                edge3_id.clone(),
                token1.clone(),
                token2.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
            // Edge 4: 1-3
            Pool::new(
                edge4_id.clone(),
                token1.clone(),
                token3.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
            // Edge 5: 2-3
            Pool::new(
                edge5_id.clone(),
                token2.clone(),
                token3.clone(),
                Some(U256::from(1000)),
                Some(U256::from(1000)),
            ),
        ];

        // Make a copy of Edge1 with even more imbalanced reserves to trigger arbitrage
        let updated_pool = Pool::new(
            edge1_id.clone(),
            token0.clone(),
            token1.clone(),
            Some(U256::from(700)),  // Further imbalanced
            Some(U256::from(1300)), // To create profitable arbitrage
        );

        // Find cycles affected by the updated pool
        let cycles = find_affected_cycles(&pools[1..], updated_pool.clone());

        // Debug output
        // println!("\nFound {} cycles:", cycles.len());
        for (i, cycle) in cycles.iter().enumerate() {
            println!(
                "Cycle {}: {} swaps, profitable: {}",
                i + 1,
                cycle.swaps.len(),
                cycle.is_positive()
            );

            // Print cycle edges
            print!("Edges: ");
            for (j, swap) in cycle.swaps.iter().enumerate() {
                let edge_id = &swap.id().pool_id;
                print!(
                    "{}{}",
                    edge_id,
                    if j < cycle.swaps.len() - 1 {
                        " → "
                    } else {
                        ""
                    }
                );
            }
            println!();

            // Print node path (using token index mapping for clarity)
            print!("Path: ");
            for (j, swap) in cycle.swaps.iter().enumerate() {
                let from = swap.token_in();
                let to = swap.token_out();

                // Map tokens to node numbers for readability
                let from_node = if from == &token0 {
                    "0"
                } else if from == &token1 {
                    "1"
                } else if from == &token2 {
                    "2"
                } else {
                    "3"
                };

                let to_node = if to == &token0 {
                    "0"
                } else if to == &token1 {
                    "1"
                } else if to == &token2 {
                    "2"
                } else {
                    "3"
                };

                if j == 0 {
                    print!("{} → ", from_node);
                }
                print!(
                    "{}{}",
                    to_node,
                    if j < cycle.swaps.len() - 1 {
                        " → "
                    } else {
                        ""
                    }
                );
            }
            println!("\n");
        }

        // Verify we found the expected cycles
        assert!(cycles.len() >= 1, "Should find at least one cycle");

        // Helper to check if a specific cycle is in the results using pool IDs and desired length
        let has_cycle = |pool_ids: &[&PoolId], cycle_len: usize| -> bool {
            cycles.iter().any(|cycle| {
                if cycle.swaps.len() != cycle_len {
                    return false;
                }

                // Check all possible rotations of the cycle
                for start in 0..cycle.swaps.len() {
                    let mut matches = true;
                    for i in 0..cycle.swaps.len() {
                        let pos = (start + i) % cycle.swaps.len();
                        let cycle_pool_id = &cycle.swaps[pos].id().pool_id;
                        if cycle_pool_id != pool_ids[i] {
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

        // Check for the 3-edge cycle: (1,3,2)
        let found_cycle1 = has_cycle(&[&edge1_id, &edge3_id, &edge2_id], 3);

        // Check for the 4-edge cycle: (1,4,5,2)
        let found_cycle2 = has_cycle(&[&edge1_id, &edge4_id, &edge5_id, &edge2_id], 4);

        println!("Cycle (1,3,2) found: {}", found_cycle1);
        println!("Cycle (1,4,5,2) found: {}", found_cycle2);

        // Try more extreme imbalance if no cycles found
        if !found_cycle1 && !found_cycle2 {
            // println!("\nNo cycles found with current pool balance. Trying with more extreme imbalance...");

            // Create dramatically imbalanced pool
            let extreme_updated_pool = Pool::new(
                edge1_id.clone(),
                token0.clone(),
                token1.clone(),
                Some(U256::from(500)),  // Very imbalanced
                Some(U256::from(1500)), // To force profitable arbitrage
            );

            // Try again with more extreme imbalance
            let extreme_cycles = find_affected_cycles(&pools[1..], extreme_updated_pool);
            println!(
                "Found {} cycles with extreme imbalance",
                extreme_cycles.len()
            );

            // If still no cycles, print additional diagnostic info
            if extreme_cycles.is_empty() {
                println!("\nDIAGNOSTIC: Trying to trace cycle detection issue");
                println!(
                    "Check that your Cycle::is_positive() implementation is working correctly"
                );
                println!("Verify that the log_rate calculation in your Swap implementation handles imbalanced reserves properly");
            }
        }

        assert!(
            found_cycle1 || found_cycle2,
            "Should find at least one of the expected cycles"
        );
    }

    #[test]
    fn test_extreme_imbalance() {
        println!("\n========== EXTREME IMBALANCE TEST ==========");
        // Create three tokens for a simple cycle
        let token_a = TokenId::try_from(generate_random_address()).unwrap();
        let token_b = TokenId::try_from(generate_random_address()).unwrap();
        let token_c = TokenId::try_from(generate_random_address()).unwrap();

        println!("Token A: {}", token_a);
        println!("Token B: {}", token_b);
        println!("Token C: {}", token_c);

        // Create three pools with balanced reserves initially
        let pool_ab = Pool::new(
            PoolId::try_from(generate_random_address()).unwrap(),
            token_a.clone(),
            token_b.clone(),
            Some(U256::from(1000)),
            Some(U256::from(1000)),
        );

        let pool_bc = Pool::new(
            PoolId::try_from(generate_random_address()).unwrap(),
            token_b.clone(),
            token_c.clone(),
            Some(U256::from(1000)),
            Some(U256::from(1000)),
        );

        // Start with balanced C-A pool
        let pool_ca = Pool::new(
            PoolId::try_from(generate_random_address()).unwrap(),
            token_c.clone(),
            token_a.clone(),
            Some(U256::from(1000)),
            Some(U256::from(1000)),
        );

        println!("Pool A-B: {}", pool_ab.id);
        println!("Pool B-C: {}", pool_bc.id);
        println!("Pool C-A: {}", pool_ca.id);

        // Build a mini market with all pools initially balanced
        let all_pools = vec![pool_ab.clone(), pool_bc.clone()];

        // Create an updated version of the C-A pool with extreme imbalance
        let updated_pool_ca = Pool::new(
            pool_ca.id.clone(),
            token_c.clone(),
            token_a.clone(),
            Some(U256::from(10000)), // 10x imbalance
            Some(U256::from(100)),   // 10x imbalance
        );

        println!("Updated Pool C-A with extreme imbalance: 10000:100");

        // Use the extremely imbalanced pool as the updated pool
        let cycles = find_affected_cycles(&all_pools, updated_pool_ca);

        println!("Found {} cycles", cycles.len());

        // Add debug info if cycles are found
        if !cycles.is_empty() {
            for (i, cycle) in cycles.iter().enumerate() {
                println!(
                    "Cycle {} has {} swaps, is profitable: {}",
                    i + 1,
                    cycle.swaps.len(),
                    cycle.is_positive()
                );

                println!("Cycle path:");
                for (j, swap) in cycle.swaps.iter().enumerate() {
                    println!(
                        "  Swap {}: {} -> {}",
                        j + 1,
                        swap.token_in(),
                        swap.token_out()
                    );

                    if let (Some(reserve_in), Some(reserve_out)) =
                        (swap.reserve_in(), swap.reserve_out())
                    {
                        println!("    Reserves: {} in, {} out", reserve_in, reserve_out);
                    }

                    if let Some(log_rate) = swap.log_rate() {
                        println!("    Log rate: {}", log_rate);
                    }
                }
            }
        } else {
            println!("\nWARNING: No profitable cycles found even with extreme imbalance!");
            println!("Check the cycle.is_positive() implementation - you may need to:");
            println!("1. Verify log rate calculations in Swap implementation");
            println!("2. Check if reserve values are correctly propagated");
            println!("3. Ensure the profitability threshold isn't too conservative");
        }

        // Success message if cycles found
        if !cycles.is_empty() {
            println!(
                "\nSUCCESS: Found {} profitable cycles with extreme imbalance",
                cycles.len()
            );
        }
    }

    /// Test to explicitly demonstrate profitable arbitrage cycles
    #[test]
    fn test_show_profitable_cycles() {
        println!("\n========== PROFITABLE CYCLE DEMONSTRATION ==========");

        // Create three tokens for a simple triangle
        let token_a = TokenId::try_from(generate_random_address()).unwrap();
        let token_b = TokenId::try_from(generate_random_address()).unwrap();
        let token_c = TokenId::try_from(generate_random_address()).unwrap();

        println!("Created test tokens:");
        println!("  Token A: {}", token_a);
        println!("  Token B: {}", token_b);
        println!("  Token C: {}", token_c);

        // Create three pools with carefully chosen reserves to ensure we get a profitable cycle
        // Pool A-B: Balanced
        let pool_ab = Pool::new(
            PoolId::try_from(generate_random_address()).unwrap(),
            token_a.clone(),
            token_b.clone(),
            Some(U256::from(1_000_000_000_000_000_u128)),
            Some(U256::from(1_000_000_000_000_000_u128)),
        );

        // Pool B-C: Balanced
        let pool_bc = Pool::new(
            PoolId::try_from(generate_random_address()).unwrap(),
            token_b.clone(),
            token_c.clone(),
            Some(U256::from(1_000_000_000_000_000_u128)),
            Some(U256::from(1_000_000_000_000_000_u128)),
        );

        // Pool C-A: Initially balanced
        let pool_ca_balanced = Pool::new(
            PoolId::try_from(generate_random_address()).unwrap(),
            token_c.clone(),
            token_a.clone(),
            Some(U256::from(1_000_000_000_000_000_u128)),
            Some(U256::from(1_000_000_000_000_000_u128)),
        );

        println!("Created pools with balanced reserves:");
        println!("  Pool A-B: ID={}", pool_ab.id);
        println!("  Pool B-C: ID={}", pool_bc.id);
        println!("  Pool C-A: ID={}", pool_ca_balanced.id);

        // First test: No profitable cycles with balanced pools
        let all_balanced_pools = vec![pool_ab.clone(), pool_bc.clone(), pool_ca_balanced.clone()];
        println!("\nTesting with all balanced pools:");
        let cycles_balanced =
            find_affected_cycles(&all_balanced_pools[0..2], pool_ca_balanced.clone());

        println!(
            "Found {} cycles when all pools are balanced",
            cycles_balanced.len()
        );
        if cycles_balanced.is_empty() {
            println!("No cycles found with balanced pools - this is expected as there are no price discrepancies.");
        } else {
            // Display cycles
            for (i, cycle) in cycles_balanced.iter().enumerate() {
                println!(
                    "Cycle {}: {} swaps, IS PROFITABLE: {}",
                    i + 1,
                    cycle.swaps.len(),
                    cycle.is_positive()
                );

                println!("  Log rates for each swap:");
                let mut total_log_rate = 0;
                for (j, swap) in cycle.swaps.iter().enumerate() {
                    let log_rate = swap.log_rate();
                    total_log_rate += log_rate;
                    println!(
                        "    Swap {}: {} -> {}, log_rate: {}",
                        j + 1,
                        swap.token_in(),
                        swap.token_out(),
                        log_rate
                    );
                }
                println!(
                    "  TOTAL LOG RATE: {} (Positive means profitable)",
                    total_log_rate
                );
            }
        }

        // Create highly imbalanced C-A pool to create profitable arbitrage
        let pool_ca_imbalanced = Pool::new(
            pool_ca_balanced.id.clone(),
            token_c.clone(),
            token_a.clone(),
            Some(U256::from(2_000_000_000_000_000_u128)), // 2x reserve0
            Some(U256::from(500_000_000_000_000_u128)),   // 0.5x reserve1
        );

        println!("\nUpdated C-A pool with imbalanced reserves:");
        println!(
            "  Original: reserve0=1000000000000000, reserve1=1000000000000000, log_rate={}",
            get_log_rate(&pool_ca_balanced)
        );
        println!(
            "  Imbalanced: reserve0=2000000000000000, reserve1=500000000000000, log_rate={}",
            get_log_rate(&pool_ca_imbalanced)
        );

        // Second test: Profitable cycles with imbalanced pool
        println!("\nTesting with imbalanced C-A pool:");
        let cycles_imbalanced = find_affected_cycles(&all_balanced_pools[0..2], pool_ca_imbalanced);

        println!(
            "Found {} cycles with imbalanced C-A pool",
            cycles_imbalanced.len()
        );
        if cycles_imbalanced.is_empty() {
            println!("No cycles found with imbalanced pool - this is unexpected.");
        } else {
            // Display cycles in detail
            for (i, cycle) in cycles_imbalanced.iter().enumerate() {
                println!(
                    "Cycle {}: {} swaps, IS PROFITABLE: {}",
                    i + 1,
                    cycle.swaps.len(),
                    cycle.is_positive()
                );

                println!("  Cycle path:");
                let mut path_str = String::new();
                for (j, swap) in cycle.swaps.iter().enumerate() {
                    let from = swap.token_in();
                    let to = swap.token_out();

                    // Map tokens to letters for readability
                    let from_letter = if from == &token_a {
                        "A"
                    } else if from == &token_b {
                        "B"
                    } else {
                        "C"
                    };

                    let to_letter = if to == &token_a {
                        "A"
                    } else if to == &token_b {
                        "B"
                    } else {
                        "C"
                    };

                    if j == 0 {
                        path_str.push_str(from_letter);
                        path_str.push_str(" → ");
                    }
                    path_str.push_str(to_letter);
                    if j < cycle.swaps.len() - 1 {
                        path_str.push_str(" → ");
                    }
                }
                println!("  {}", path_str);

                println!("  Log rates for each swap:");
                let mut total_log_rate = 0;
                for (j, swap) in cycle.swaps.iter().enumerate() {
                    let log_rate = swap.log_rate();
                    total_log_rate += log_rate;

                    // Map tokens to letters
                    let from = swap.token_in();
                    let to = swap.token_out();
                    let from_letter = if from == &token_a {
                        "A"
                    } else if from == &token_b {
                        "B"
                    } else {
                        "C"
                    };
                    let to_letter = if to == &token_a {
                        "A"
                    } else if to == &token_b {
                        "B"
                    } else {
                        "C"
                    };

                    println!(
                        "    Swap {}: {} → {}, log_rate: {}",
                        j + 1,
                        from_letter,
                        to_letter,
                        log_rate
                    );

                    if let (Some(reserve_in), Some(reserve_out)) =
                        (swap.reserve_in(), swap.reserve_out())
                    {
                        println!(
                            "      Reserves: {} {}, {} {}",
                            reserve_in, from_letter, reserve_out, to_letter
                        );
                    }
                }
                println!(
                    "  TOTAL LOG RATE: {} (Positive means profitable)",
                    total_log_rate
                );
                println!("  This cycle is profitable because the total log rate is positive.");
                println!(
                    "  The imbalanced C-A pool creates a price discrepancy that can be exploited."
                );
            }
        }

        // If we found profitable cycles, consider the test successful
        assert!(
            !cycles_imbalanced.is_empty(),
            "Should find at least one profitable cycle"
        );

        // Verify cycles are actually profitable
        for cycle in cycles_imbalanced.iter() {
            assert!(cycle.is_positive(), "Cycle should be profitable");

            // Calculate total log rate manually to verify
            let mut total_log_rate = 0;
            for swap in cycle.swaps.iter() {
                total_log_rate += swap.log_rate();
            }
            assert!(total_log_rate > 0, "Total log rate should be positive");
        }

        println!("\nTEST SUCCESSFUL: Demonstrated profitable arbitrage cycles");
        println!("A positive total log rate indicates a profitable cycle");
        println!("The more imbalanced the pools, the more profitable the arbitrage opportunity");
    }
}

// Benchmark group for arbitrage cycle detection
criterion_group!(
    benches,
    bench_find_cycles,
    bench_production_data,
    bench_specific_arbitrage_cycle
);
// Main entry point for benchmarks
criterion_main!(benches);
