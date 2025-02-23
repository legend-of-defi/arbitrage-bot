use std::env;

/// Configuration struct for the application
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub rpc_url: String,
    pub ipc_path: String,
}

impl Config {
    /// Default values for configuration
    fn defaults() -> Self {
        let default_ipc_path = if cfg!(windows) {
            r"\\.\pipe\mev_eth"
        } else {
            "/opt/reth/data/reth.ipc"
        };

        Self {
            database_url: "postgresql://fly@localhost?host=/var/run/postgresql".to_string(),
            rpc_url: "https://mainnet.base.org".to_string(),
            ipc_path: default_ipc_path.to_string(),
        }
    }

    /// Load configuration from environment variables
    ///
    /// # Environment Variables:
    /// - `DATABASE_URL`: `PostgreSQL` connection string
    /// - `RPC_URL`: Ethereum RPC endpoint URL
    /// - `IPC_PATH`: Path to IPC socket/pipe
    ///
    /// # Platform-specific notes:
    /// - Linux: Add environment variables to systemd service file
    /// - Windows: Set using `PowerShell`, e.g.: `$env:RPC_URL = "https://mainnet.base.org"`
    ///
    /// # Returns
    /// Returns `Config` with values from environment variables or defaults
    #[must_use]
    pub fn from_env() -> Self {
        let defaults = Self::defaults();

        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or(defaults.database_url),
            rpc_url: env::var("RPC_URL")
                .unwrap_or(defaults.rpc_url),
            ipc_path: env::var("IPC_PATH")
                .unwrap_or(defaults.ipc_path),
        }
    }

    /// Create a test configuration
    #[cfg(test)]
    #[must_use] pub fn test_config() -> Self {
        Self::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        // Set test environment variables
        env::set_var("DATABASE_URL", "test_db_url");
        env::set_var("RPC_URL", "test_rpc_url");
        env::set_var("IPC_PATH", "test_ipc_path");

        let config = Config::from_env();
        assert_eq!(config.database_url, "test_db_url");
        assert_eq!(config.rpc_url, "test_rpc_url");
        assert_eq!(config.ipc_path, "test_ipc_path");
    }

    #[test]
    fn test_config_defaults() {
        // Clear environment variables
        env::remove_var("DATABASE_URL");
        env::remove_var("RPC_URL");
        env::remove_var("IPC_PATH");

        let config = Config::from_env();
        let defaults = Config::defaults();

        assert_eq!(config.database_url, defaults.database_url);
        assert_eq!(config.rpc_url, defaults.rpc_url);
        assert_eq!(config.ipc_path, defaults.ipc_path);
    }
}