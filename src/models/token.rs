use alloy::primitives::Address;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable, Selectable};

use super::pair::DBAddress;

/// Token model
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Token {
    /// The ID of the token
    id: i32,
    /// The address of the token
    address: DBAddress,
    /// The symbol of the
    #[allow(dead_code)]
    symbol: Option<String>,
    /// The name of the token
    #[allow(dead_code)]
    name: Option<String>,
    /// The decimals of the token
    #[allow(dead_code)]
    decimals: Option<i32>,
    /// Whether the token is valid
    #[allow(dead_code)]
    is_valid: bool,
    /// The exchange rate of the token
    #[allow(dead_code)]
    exchange_rate: Option<BigDecimal>,
    /// The timestamp when the exchange rate was last updated
    #[allow(dead_code)]
    updated_last: Option<NaiveDateTime>,
}

/// Parameters for creating a new Token
/// This is Value Object pattern to avoid long constructor arguments
#[derive(Debug)]
#[allow(dead_code)]
pub struct TokenParams {
    /// The ID of the token
    pub id: i32,
    /// The address of the token
    pub address: Address,
    /// The symbol of the token
    pub symbol: Option<String>,
    /// The name of the token
    pub name: Option<String>,
    /// The decimals of the token
    pub decimals: Option<i32>,
    /// Whether the token is valid
    pub is_valid: bool,
    /// The exchange rate of the token
    pub exchange_rate: Option<BigDecimal>,
    /// The timestamp when the exchange rate was last updated
    pub updated_last: Option<NaiveDateTime>,
}

impl Token {
    /// Get the address of the token
    #[must_use]
    pub fn address(&self) -> Address {
        self.address.value
    }

    /// Get the ID of the token
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the decimals of the token
    #[must_use]
    pub fn decimals(&self) -> Option<i32> {
        self.decimals
    }
}

/// A new token
#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schemas::tokens)]
pub struct NewToken {
    /// The address of the token
    address: DBAddress,
    /// The symbol of the token
    symbol: Option<String>,
    /// The name of the token
    name: Option<String>,
    /// The decimals of the token
    decimals: i32,
    /// Whether the token is valid
    is_valid: bool,
    /// The exchange rate of the token
    exchange_rate: Option<BigDecimal>,
    /// The timestamp when the exchange rate was last updated
    updated_last: Option<NaiveDateTime>,
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
    /// * `exchange_rate` - The optional exchange rate of the token in USD.
    /// * `updated_last` - The optional timestamp when the exchange rate was last updated.
    ///
    /// # Returns
    ///
    /// * Returns a new `NewToken` instance with sanitized `symbol` and `name` (if they were provided),
    ///   and the provided `address` and `decimals` values.
    #[must_use]
    // TODO: remove this once we have a better way to handle the number of arguments
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        address: Address,
        symbol: Option<String>,
        name: Option<String>,
        decimals: i32,
        is_valid: bool,
        exchange_rate: Option<BigDecimal>,
        updated_last: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            address: DBAddress::new(address),
            symbol: symbol.map(|s| sanitize_string(&s)),
            name: name.map(|n| sanitize_string(&n)),
            decimals,
            is_valid,
            exchange_rate,
            updated_last,
        }
    }

    /// Get the address of the token
    #[must_use]
    pub fn address(&self) -> Address {
        self.address.value
    }

    /// Get the symbol of the token
    #[must_use]
    pub fn symbol(&self) -> Option<String> {
        self.symbol.as_deref().map(std::string::ToString::to_string)
    }

    /// Get the name of the token
    #[must_use]
    pub fn name(&self) -> Option<String> {
        self.name.as_deref().map(std::string::ToString::to_string)
    }

    /// Get the decimals of the token
    #[must_use]
    pub fn decimals(&self) -> i32 {
        self.decimals
    }

    /// Whether the token is valid
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Get the exchange rate of the token
    #[must_use]
    pub fn exchange_rate(&self) -> Option<BigDecimal> {
        self.exchange_rate.clone()
    }

    /// Get the timestamp when the exchange rate was last updated
    #[must_use]
    pub fn updated_last(&self) -> Option<NaiveDateTime> {
        self.updated_last
    }
}

/// Sanitizes a given string by:
/// 1. Converting any invalid UTF-8 sequences to the replacement character.
/// 2. Removing any null byte characters (`\0`).
/// 3. Removing any non-printable or control characters.
///
/// # Arguments
/// * `value` - A string slice that represents the value to be sanitized.
///
/// # Returns
/// A new `String` with invalid UTF-8 replaced and null bytes removed.
fn sanitize_string(value: &str) -> String {
    // First convert to lossy UTF-8 string to handle invalid sequences
    let sanitized = String::from_utf8_lossy(value.as_bytes()).to_string();

    // Then remove null bytes and filter out any non-printable or replacement characters
    sanitized
        .replace('\0', "")
        .chars()
        .filter(|&c| c.is_ascii_graphic() || c.is_whitespace())
        .collect()
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

        // Check that the null byte is removed and invalid byte is replaced with ""
        assert_eq!(sanitized, "Ethereum"); // Null byte removed, and invalid byte replaced with ""
    }

    // Test NewToken::new method
    #[test]
    fn test_new_token_creation_with_sanitization() {
        let token = NewToken::new(
            WETH,
            Some("ETH\0".to_string()),      // Contains null byte
            Some("Ethereum\0".to_string()), // Contains null byte
            18,
            true,
            None,
            None,
        );

        // Create a new token using the values from the first token
        let new_token = NewToken::new(
            token.address.value,
            token.symbol,
            token.name,
            token.decimals,
            token.is_valid,
            token.exchange_rate,
            token.updated_last,
        );

        assert_eq!(new_token.address.value, WETH);

        // Check that the symbol and name have been sanitized
        assert_eq!(new_token.symbol, Some("ETH".to_string())); // Null byte removed
        assert_eq!(new_token.name, Some("Ethereum".to_string())); // Null byte removed
        assert_eq!(new_token.decimals, 18);
        assert!(new_token.is_valid);
        assert_eq!(new_token.exchange_rate, None);
        assert_eq!(new_token.updated_last, None);
    }

    // Test with None for symbol and name (no sanitization needed)
    #[test]
    fn test_new_token_creation_with_none_values() {
        let token = NewToken::new(WETH, None, None, 6, true, None, None);

        let new_token = NewToken::new(
            token.address.value,
            token.symbol,
            token.name,
            token.decimals,
            token.is_valid,
            token.exchange_rate,
            token.updated_last,
        );

        assert_eq!(new_token.address.value, WETH);
        assert_eq!(new_token.symbol, None); // No sanitization or modification needed
        assert_eq!(new_token.name, None); // No sanitization or modification needed
        assert_eq!(new_token.decimals, 6);
        assert!(new_token.is_valid);
        assert_eq!(new_token.exchange_rate, None);
        assert_eq!(new_token.updated_last, None);
    }
}
