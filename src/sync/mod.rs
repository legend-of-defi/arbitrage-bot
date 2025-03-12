/// Sync module
///
/// This module contains all the functions for syncing the database.
///
/// # Errors
/// Returns an error if the database connection fails
///
/// # Returns
/// Returns the number of pairs synced
pub mod exchange_rates;
/// Sync factories for pairs
///
/// This module contains all the functions for syncing the factories for pairs.
///
/// # Errors
/// Returns an error if the database connection fails
///
pub mod factories;
/// Sync factory pairs
///
/// This module contains all the functions for syncing the factory pairs.
///
/// # Errors
/// Returns an error if the database connection fails
pub mod factory_pairs;
/// Sync pair created events
///
/// This module contains all the functions for syncing the pair created events.
///
/// # Errors
/// Returns an error if the database connection fails
pub mod pair_created_events;
/// Sync pair tokens
///
/// This module contains all the functions for syncing the pair tokens.
///
/// # Errors
/// Returns an error if the database connection fails
pub mod pair_tokens;
/// Sync reserves
///
/// This module contains all the functions for syncing the reserves.
///
/// # Errors
/// Returns an error if the database connection fails
pub mod reserves;
/// Sync events
///
/// This module contains all the functions for syncing the events.
///
/// # Errors
/// Returns an error if the database connection fails
pub mod sync_events;
/// Sync USD
///
/// This module contains all the functions for syncing the USD.
///
/// # Errors
/// Returns an error if the database connection fails
pub mod usd;

pub use exchange_rates::exchange_rates;
pub use factories::factories;
pub use factory_pairs::factory_pairs;
pub use pair_created_events::pair_created_events;
pub use pair_tokens::pair_tokens;
pub use reserves::reserves;
pub use sync_events::events;
pub use usd::usd;
