use criterion::{criterion_group, criterion_main, Criterion};

pub fn benchmark_triangular_cycles(_: &mut Criterion) {

}
//     let mut group = c.benchmark_group("arbitrage");

//     for size in [1000, 5000, 10000].iter() {
//         group.bench_with_input(format!("triangular_{}", size), size, |b, &size| {
//             b.iter(|| {
//                 let pools = generate_many_pools(size, 0.7);
//                 triangular_cycles(pools)
//             })
//         });
//     }
//     group.finish();
// }
// /// Generate a new random token address
// fn generate_token() -> String {
//     format!("0x{:040x}", fastrand::u64(..))
// }

// /// Select a random token from existing tokens, different from excluded_token if specified
// fn select_random_token(
//     existing_tokens: &HashSet<String>,
//     excluded_token: Option<&str>,
// ) -> Option<String> {
//     if existing_tokens.is_empty() {
//         return None;
//     }

//     for _ in 0..10 {
//         // Limit attempts to avoid infinite loop
//         let token = existing_tokens
//             .iter()
//             .nth(fastrand::usize(..existing_tokens.len()))
//             .unwrap()
//             .clone();

//         if excluded_token.map_or(true, |excluded| token != excluded) {
//             return Some(token);
//         }
//     }
//     None
// }

// /// Generate test pools with controlled density
// /// - count: number of pools to generate
// /// - density: 0.0 to 1.0, higher values create more interconnected pools by reusing tokens
// pub fn generate_many_pools(count: usize, density: f64) -> HashSet<Pool> {
//     let mut pools = HashSet::new();
//     let mut existing_tokens = HashSet::new();

//     for _ in 0..count {
//         let token0 = if !existing_tokens.is_empty() && fastrand::f64() < density {
//             // Reuse an existing token
//             select_random_token(&existing_tokens, None).unwrap_or_else(generate_token)
//         } else {
//             // Generate new token
//             let token = generate_token();
//             existing_tokens.insert(token.clone());
//             token
//         };

//         let token1 = if !existing_tokens.is_empty() && fastrand::f64() < density {
//             // Reuse an existing token (different from token0)
//             select_random_token(&existing_tokens, Some(&token0)).unwrap_or_else(generate_token)
//         } else {
//             // Generate new token
//             let token = generate_token();
//             existing_tokens.insert(token.clone());
//             token
//         };

//         if !existing_tokens.contains(&token1) {
//             existing_tokens.insert(token1.clone());
//         }

//         pools.insert(Pool {
//             address: generate_token(),
//             token0,
//             token1,
//             reserve0: fastrand::u64(..), // Avoid zero reserves
//             reserve1: fastrand::u64(..),
//         });
//     }
//     pools
// }

criterion_group!(benches, benchmark_triangular_cycles);
criterion_main!(benches);
