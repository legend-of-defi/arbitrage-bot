use alloy::primitives::Address;
use diesel::{Insertable, Queryable, Selectable};

use super::pair::DBAddress;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Token {
    id: i32,
    address: DBAddress,
    symbol: Option<String>,
    name: Option<String>,
    decimals: Option<i32>,
}

impl Token {
    pub fn new(
        id: i32,
        address: Address,
        symbol: Option<String>,
        name: Option<String>,
        decimals: Option<i32>,
    ) -> Self {
        Self {
            id,
            address: DBAddress::new(address),
            symbol,
            name,
            decimals,
        }
    }

    pub fn address(&self) -> Address {
        self.address.value
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn symbol(&self) -> Option<String> {
        self.symbol.as_deref().map(|s| s.to_string())
    }

    pub fn decimals(&self) -> Option<i32> {
        self.decimals
    }

    pub fn name(&self) -> Option<String> {
        self.name.as_deref().map(|n| n.to_string())
    }
}

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schemas::tokens)]
pub struct NewToken {
    address: DBAddress,
    symbol: Option<String>,
    name: Option<String>,
    decimals: i32,
}

impl NewToken {
    /// Creates a new `NewToken` instance, sanitizing `symbol` and `name` fields if provided.
    ///
    /// # Arguments
    ///
    /// * `address` - The address of the token (usually a string representation of the address).
    /// * `symbol` - The optional symbol of the token (e.g., "ETH"). It will be sanitized if provided.
    /// * `name` - The optional name of the token (e.g., "Ethereum"). It will be sanitized if provided.
    /// * `decimals` - The number of decimals the token uses (e.g., 18).
    ///
    /// # Returns
    ///
    /// * Returns a new `NewToken` instance with sanitized `symbol` and `name` (if they were provided),
    ///   and the provided `address` and `decimals` values.
    pub fn new(
        address: Address,
        symbol: Option<String>,
        name: Option<String>,
        decimals: i32,
    ) -> Self {
        Self {
            address: DBAddress::new(address),
            symbol: symbol.map(|s| sanitize_string(&s)),
            name: name.map(|n| sanitize_string(&n)),
            decimals,
        }
    }

    pub fn address(&self) -> Address {
        self.address.value
    }

    pub fn symbol(&self) -> Option<String> {
        self.symbol.as_deref().map(|s| s.to_string())
    }

    pub fn name(&self) -> Option<String> {
        self.name.as_deref().map(|n| n.to_string())
    }

    pub fn decimals(&self) -> i32 {
        self.decimals
    }
}

/// Sanitizes a given string by:
/// 1. Converting any invalid UTF-8 sequences to the replacement character `�`.
/// 2. Removing any null byte characters (`\0`).
///
/// # Arguments
/// * `value` - A string slice that represents the value to be sanitized.
///
/// # Returns
/// A new `String` with invalid UTF-8 replaced and null bytes removed.
fn sanitize_string(value: &str) -> String {
    let sanitized = String::from_utf8_lossy(value.as_bytes()).to_string();
    sanitized.replace('\0', "")
}

#[cfg(test)]
mod tests {
    use crate::utils::constants::WETH;

    use super::*;

    // Test sanitization function
    #[test]
    fn test_sanitize_string() {
        // Create a raw byte vector with both a null byte and an invalid UTF-8 byte (0x80)
        let input_invalid_bytes = vec![
            b'E', b't', b'h', b'e', b'\0', b'r', b'e', b'u', b'm', b'\x80',
        ];

        // Convert the raw byte slice to a string using `from_utf8_lossy`, which handles invalid UTF-8
        let input_invalid = String::from_utf8_lossy(&input_invalid_bytes);

        // Sanitize the string (removes null byte and replaces invalid UTF-8)
        let sanitized = sanitize_string(&input_invalid);

        // Check that the null byte is removed and invalid UTF-8 byte is replaced with "�"
        assert_eq!(sanitized, "Ethereum�"); // Null byte removed, and invalid byte replaced with "�"
    }

    // Test NewToken::new methodca
    #[test]
    fn test_new_token_creation_with_sanitization() {
        let token = NewToken::new(
            WETH,
            Some("ETH\0".to_string()),      // Contains null byte
            Some("Ethereum\0".to_string()), // Contains null byte
            18,
        );

        let new_token = NewToken::new(
            token.address.value,
            token.symbol,
            token.name,
            token.decimals,
        );

        // Verify that the sanitization worked
        assert_eq!(new_token.address.value, WETH);

        // Check that the symbol and name have been sanitized
        assert_eq!(new_token.symbol, Some("ETH".to_string())); // Null byte removed
        assert_eq!(new_token.name, Some("Ethereum".to_string())); // Null byte removed
        assert_eq!(new_token.decimals, 18);
    }

    // Test with None for symbol and name (no sanitization needed)
    #[test]
    fn test_new_token_creation_with_none_values() {
        let token = NewToken::new(WETH, None, None, 6);

        let new_token = NewToken::new(
            token.address.value,
            token.symbol,
            token.name,
            token.decimals,
        );

        assert_eq!(new_token.address.value, WETH);
        assert_eq!(new_token.symbol, None); // No sanitization or modification needed
        assert_eq!(new_token.name, None); // No sanitization or modification needed
        assert_eq!(new_token.decimals, 6);
    }
}
