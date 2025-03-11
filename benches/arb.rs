use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::prelude::*;
use std::collections::HashSet;
use std::str::FromStr;
use alloy::primitives::{Address, U256};
use fly::arb::{
    cycle::Cycle,
    pool::{Pool, PoolId},
    token::TokenId,
};

/// Generate a new random token address
fn generate_random_address() -> String {
    let addr_str = format!("0x{:040x}", fastrand::u64(..)).as_str();
    let address_checksum = Address::from_str(addr_str).unwrap();
    address_checksum.to_string()
}

/// Generate synthetic test data for benchmarking
fn generate_benchmark_pools(pool_count: usize, token_count: usize) -> Vec<Pool> {
    let mut rng = rand::rng();
    let mut pools = Vec::with_capacity(pool_count);

    // Create token IDs
    let tokens: Vec<TokenId> = (0..token_count)
        .map(|i| TokenId::try_from(generate_random_address()).unwrap())
        .collect();

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

        pools.push(pool);
    }

    pools
}

/// Find all cycles affected by an updated pool
/// This is a placeholder - implement the actual function in your crate
fn find_affected_cycles(pools: &Vec<Pool>, updated_pool: Pool) -> Vec<Cycle> {
    // Implementation would go in your crate
    // Placeholder return
    Vec::new()
}

/// Benchmark finding cycles with a randomly updated pool
fn bench_find_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_affected_cycles");

    // Configure measurement settings for more accurate results
    group.sample_size(50);         // Number of samples to take
    group.measurement_time(std::time::Duration::from_secs(10)); // Min time for each sample set

    // Benchmark with different pool counts to find our limits
    for pool_count in [100, 500, 1000, 2500, 5000].iter() {
        // Create a synthetic market with 20% of the pool count as tokens
        // This mimics real-world token-to-pool ratios
        let token_count = ((pool_count / 5) as usize).max(10);
        let pools = generate_benchmark_pools(*pool_count, token_count);

        // Select a random pool to update for the benchmark
        let random_pool_idx = rand::thread_rng().gen_range(0..pools.len());
        let updated_pool = pools[random_pool_idx].clone();

        // Configure a specific throughput measurement based on pool count
        group.throughput(criterion::Throughput::Elements(*pool_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(pool_count),
            pool_count,
            |b, _| {
                // Setup phase (not measured)
                b.iter_batched(
                    // Setup function (called for each batch)
                    || (pools.clone(), updated_pool.clone()),

                    // Benchmark function (timed)
                    |(p, up)| black_box(find_affected_cycles(&p, up)),

                    // Batch size
                    criterion::BatchSize::SmallInput
                )
            }
        );
    }

    group.finish();
}

/// Benchmark against production-like data
fn bench_production_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("production_data");

    // Configure for thorough statistical analysis
    group.sample_size(30);  // Fewer samples for complex scenarios
    group.measurement_time(std::time::Duration::from_secs(15));

    // Define our test matrix - format: (name, pool_count, token_count, density)
    let test_configs = [
        // Low density markets (fewer connections between tokens)
        ("sparse_small", 100, 50, 0.3),
        ("sparse_medium", 500, 200, 0.3),
        ("sparse_large", 1000, 400, 0.3),

        // Medium density markets (moderate connections)
        ("medium_small", 100, 30, 0.5),
        ("medium_medium", 500, 150, 0.5),
        ("medium_large", 1000, 300, 0.5),

        // High density markets (many connections between tokens)
        ("dense_small", 100, 20, 0.7),
        ("dense_medium", 500, 100, 0.7),
        ("dense_large", 1000, 200, 0.7),
    ];

    for (name, pool_count, token_count, density) in test_configs {
        // Generate pools with controlled density
        // Here we'd use density to control how pools are generated
        let pools = generate_benchmark_pools(pool_count, token_count);

        // For each scenario, we test multiple update patterns

        // 1. Update a high-connectivity pool (many cycles affected)
        let high_connectivity_pool = pools[0].clone(); // We'd identify this properly in real code

        // 2. Update a low-connectivity pool (few cycles affected)
        let low_connectivity_pool = pools[pools.len()-1].clone(); // Simplified

        // First benchmark: high connectivity pool updates
        group.bench_with_input(
            BenchmarkId::new("high_connectivity", name),
            &name,
            |b, _| {
                b.iter_batched(
                    || (pools.clone(), high_connectivity_pool.clone()),
                    |(p, up)| black_box(find_affected_cycles(&p, up)),
                    criterion::BatchSize::SmallInput
                )
            }
        );

        // Second benchmark: low connectivity pool updates
        group.bench_with_input(
            BenchmarkId::new("low_connectivity", name),
            &name,
            |b, _| {
                b.iter_batched(
                    || (pools.clone(), low_connectivity_pool.clone()),
                    |(p, up)| black_box(find_affected_cycles(&p, up)),
                    criterion::BatchSize::SmallInput
                )
            }
        );
    }

    group.finish();
}

// Criterion setup
criterion_group!(benches, bench_find_cycles, bench_production_data);
criterion_main!(benches);