use diesel::pg::PgConnection;
use diesel::prelude::*;
use eyre::{Error, Result};

/// Establishes a connection to the Postgres database.
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

