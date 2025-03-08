use std::sync::OnceLock;

use deadpool_postgres::{Config, Pool, PoolConfig};
use eyre::{Error, Result};

// Global connection pool
static CONNECTION_POOL: OnceLock<Pool> = OnceLock::new();

/// Initializes the database connection pool.
///
/// # Returns
/// * `Result<()>` - Success or failure of pool initialization
///
/// # Errors
/// * If `DATABASE_URL` environment variable is not set
/// * If pool creation fails
pub fn init_pool() -> Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| Error::msg("DATABASE_URL must be set"))?;

    let mut cfg = Config::new();
    cfg.url = Some(database_url);
    cfg.pool = Some(PoolConfig::new(15));

    let pool = cfg
        .create_pool(None, tokio_postgres::NoTls)
        .map_err(|e| Error::msg(format!("Failed to create connection pool: {e}")))?;

    CONNECTION_POOL
        .set(pool)
        .map_err(|_| Error::msg("Pool already initialized"))?;

    Ok(())
}

/// Gets the global connection pool.
///
/// # Returns
/// * `&'static Pool` - Reference to the connection pool
///
/// # Panics
/// * If the pool hasn't been initialized
pub fn get_pool() -> &'static Pool {
    CONNECTION_POOL
        .get()
        .expect("Database pool not initialized")
}

/// Gets a connection from the pool.
///
/// # Returns
/// * `Result<deadpool_postgres::Client>` - A pooled database connection
///
/// # Errors
/// * If getting a connection from the pool fails
pub async fn get_connection() -> Result<deadpool_postgres::Client> {
    get_pool()
        .get()
        .await
        .map_err(|e| Error::msg(format!("Failed to get connection: {e}")))
}
