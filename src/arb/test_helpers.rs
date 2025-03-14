#![allow(dead_code)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

/// Helper functions for testing
use crate::arb::pool::Pool;
use alloy::primitives::{Address, U256};

use super::cycle::Cycle;
use super::pool::PoolId;
use super::swap::{Direction, SwapId};
use super::swap_quote::SwapQuote;
use super::token::{Token, TokenId};
use super::{swap::Swap, world::World};

/// Creates a World instance with the given pools for testing.
///
/// # Arguments
///
/// * `pool_args` - A slice of tuples containing pool parameters:
///   (id, token0, token1, reserve0, reserve1)
///
/// # Returns
///
/// A World instance containing the specified pools
pub fn world(pool_args: &[(&str, &str, &str, u64, u64)]) -> World {
    let pools: std::collections::HashSet<_> = pool_args
        .iter()
        .map(|(id, token0, token1, reserve0, reserve1)| {
            pool(id, token0, token1, *reserve0, *reserve1)
        })
        .collect();

    World::new(&pools)
}

/// Creates a Token instance with the given ID for testing.
///
/// # Arguments
///
/// * `id` - A string representation of the token ID
///
/// # Returns
///
/// A Token instance with the specified ID
pub fn token(id: &str) -> Token {
    Token::new(TokenId::from(address_from_str(id)))
}

/// Creates a Swap instance with reserves for testing.
///
/// # Arguments
///
/// * `pool_id` - The ID of the pool
/// * `token_in` - The input token ID
/// * `token_out` - The output token ID
/// * `reserve_in` - The reserve of the input token
/// * `reserve_out` - The reserve of the output token
///
/// # Returns
///
/// A Swap instance with the specified parameters
pub fn swap(
    pool_id: &str,
    token_in: &str,
    token_out: &str,
    reserve_in: u64,
    reserve_out: u64,
) -> Swap {
    make_swap(
        pool_id,
        token_in,
        token_out,
        Some(reserve_in),
        Some(reserve_out),
    )
}

/// Creates a Swap instance without reserves for testing.
///
/// # Arguments
///
/// * `pool_id` - The ID of the pool
/// * `token_in` - The input token ID
/// * `token_out` - The output token ID
///
/// # Returns
///
/// A Swap instance with the specified parameters but no reserves
pub fn bare_swap(pool_id: &str, token_in: &str, token_out: &str) -> Swap {
    make_swap(pool_id, token_in, token_out, None, None)
}

/// Helper function to create a Swap instance with the given parameters.
///
/// # Arguments
///
/// * `pool_id` - The ID of the pool
/// * `token_in` - The input token ID
/// * `token_out` - The output token ID
/// * `reserve_in` - The optional reserve of the input token
/// * `reserve_out` - The optional reserve of the output token
///
/// # Returns
///
/// A Swap instance with the specified parameters
///
/// # Panics
///
/// Panics if `token_in` and `token_out` are the same
fn make_swap(
    pool_id: &str,
    token_in: &str,
    token_out: &str,
    reserve_in: Option<u64>,
    reserve_out: Option<u64>,
) -> Swap {
    assert!(
        (token_in != token_out),
        "Token0 and token1 must be different"
    );

    let token0_id = TokenId(address_from_str(token_in));
    let token1_id = TokenId(address_from_str(token_out));

    let direction = if token0_id < token1_id {
        Direction::ZeroForOne
    } else {
        Direction::OneForZero
    };

    let pool_id = PoolId::from(address_from_str(pool_id));

    // Convert reserve_in to Option<U256> based on whether it's None or Some
    let reserve_in_u256 = reserve_in.map(U256::from);

    // Convert reserve_out to Option<U256>
    let reserve_out_u256 = reserve_out.map(U256::from);

    Swap::new(
        SwapId { pool_id, direction },
        TokenId::from(address_from_str(token_in)),
        TokenId::from(address_from_str(token_out)),
        reserve_in_u256,
        reserve_out_u256,
    )
    .unwrap()
}

/// Generates a deterministic Address from a string by padding it with zeros.
///
/// # Arguments
///
/// * `s` - The string to convert to an address
///
/// # Returns
///
/// An Address derived from the input string
pub fn address_from_str(s: &str) -> Address {
    // Verify string only contains valid hex characters (0-9, a-f, A-F)
    // They must convert to valid a Address and when looking at the Address in the console,
    // it must match the input string.
    assert!(
        s.chars().all(|c| c.is_ascii_hexdigit()),
        "Invalid hex character in string: {s}. Only hex characters are allowed."
    );
    // Take first 40 chars or pad with zeros if shorter
    let hex_str = format!("{s:0<40}");

    // Create a byte array from the hex string
    let mut bytes = [0u8; 20];
    for (i, chunk) in hex_str.as_bytes().chunks(2).enumerate().take(20) {
        let byte_str = std::str::from_utf8(chunk).unwrap_or("11");
        let byte_val = u8::from_str_radix(byte_str, 16).unwrap_or(0);
        bytes[i] = byte_val;
    }

    Address::from(bytes)
}

/// Creates a `SwapQuote` instance for testing.
///
/// # Arguments
///
/// * `id` - The ID of the pool
/// * `token0` - The first token ID
/// * `token1` - The second token ID
/// * `reserve0` - The reserve of the first token
/// * `reserve1` - The reserve of the second token
/// * `amount_in` - The amount of tokens to input into the swap
///
/// # Returns
///
/// A `SwapQuote` instance with the specified parameters
pub fn swap_quote(
    id: &str,
    token0: &str,
    token1: &str,
    reserve0: u64,
    reserve1: u64,
    amount_in: u64,
) -> SwapQuote {
    SwapQuote::new(
        &swap(id, token0, token1, reserve0, reserve1),
        U256::from(amount_in),
    )
}

/// Creates a Pool instance with the given parameters for testing.
///
/// # Arguments
///
/// * `id` - The ID of the pool
/// * `token0` - The first token ID
/// * `token1` - The second token ID
/// * `reserve0` - The reserve of the first token
/// * `reserve1` - The reserve of the second token
///
/// # Returns
///
/// A Pool instance with the specified parameters
pub fn pool(id: &str, token0: &str, token1: &str, reserve0: u64, reserve1: u64) -> Pool {
    assert!(token0 < token1, "Token0 must be less than token1");

    Pool::new(
        PoolId::from(address_from_str(id)),
        TokenId::from(address_from_str(token0)),
        TokenId::from(address_from_str(token1)),
        Some(U256::from(reserve0)),
        Some(U256::from(reserve1)),
    )
}

/// Creates a Pool instance without reserves for testing.
///
/// # Arguments
///
/// * `symbol` - The ID of the pool
/// * `token0` - The first token ID
/// * `token1` - The second token ID
///
/// # Returns
///
/// A Pool instance with the specified parameters but no reserves
pub fn bare_pool(symbol: &str, token0: &str, token1: &str) -> Pool {
    Pool::new(
        PoolId::from(address_from_str(symbol)),
        TokenId::from(address_from_str(token0)),
        TokenId::from(address_from_str(token1)),
        None,
        None,
    )
}

/// Gets a Swap instance from a World by its index.
///
/// # Arguments
///
/// * `market` - The World instance containing the swaps
/// * `index` - The index of the swap to retrieve
///
/// # Returns
///
/// A reference to the Swap at the specified index
pub fn swap_by_index(market: &World, index: usize) -> &Swap {
    &market.swap_vec[index]
}

/// Create a cycle from a list of swap parameters
pub fn cycle(swaps: &[(&str, &str, &str, u64, u64)]) -> Result<Cycle, String> {
    let swaps = swaps
        .iter()
        .map(|(pool, token0, token1, reserve0, reserve1)| {
            swap(pool, token0, token1, *reserve0, *reserve1)
        })
        .collect();

    Cycle::new(swaps).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_from_str() {
        // Short strings get padded with zeros
        assert_eq!(
            address_from_str("f1").to_string(),
            "0xF100000000000000000000000000000000000000"
        );

        // Longer strings get truncated
        assert_eq!(
            address_from_str("ABC1").to_string(),
            "0xabC1000000000000000000000000000000000000"
        );
    }

    #[test]
    #[should_panic(
        expected = "Invalid hex character in string: test. Only hex characters are allowed."
    )]
    fn test_address_from_str_panics() {
        address_from_str("test");
    }
}
