use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use eyre::{Error, Result};
use std::sync::OnceLock;

type PgPool = Pool<ConnectionManager<PgConnection>>;
type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

// Global connection pool
static CONNECTION_POOL: OnceLock<PgPool> = OnceLock::new();

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

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(15) // Adjust based on your application needs
        .build(manager)
        .map_err(|e| Error::msg(format!("Failed to create connection pool: {e}")))?;

    CONNECTION_POOL
        .set(pool)
        .map_err(|_| Error::msg("Pool already initialized"))?;
    Ok(())
}

/// Gets the global connection pool.
///
/// # Returns
/// * `&'static PgPool` - Reference to the connection pool
///
/// # Panics
/// * If the pool hasn't been initialized
pub fn get_pool() -> &'static PgPool {
    CONNECTION_POOL
        .get()
        .expect("Database pool not initialized")
}

/// Gets a connection from the pool.
///
/// # Returns
/// * `Result<PgPooledConnection>` - A pooled database connection
///
/// # Errors
/// * If getting a connection from the pool fails
pub fn get_connection() -> Result<PgPooledConnection> {
    get_pool()
        .get()
        .map_err(|e| Error::msg(format!("Failed to get connection: {e}")))
}

/// Establishes a connection to the Postgres database.
///
/// This function is maintained for backward compatibility.
/// Consider using `get_connection()` instead for better performance.
///
/// # Returns
/// * `Result<PgConnection>` - The database connection
///
/// # Errors
/// * If `DATABASE_URL` environment variable is not set
/// * If database connection fails
pub fn establish_connection() -> Result<PgConnection> {
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| Error::msg("DATABASE_URL must be set"))?;

    PgConnection::establish(&database_url)
        .map_err(|e| Error::msg(format!("Error connecting to {database_url}: {e}")))
}
