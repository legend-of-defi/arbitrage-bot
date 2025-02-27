//! Application context for managing blockchain network connections.
//!
//! This module provides a centralized way to manage connections to different
//! Ethereum-compatible networks, including both local and remote providers.
//! It supports connections to:
//! - Ethereum Mainnet (local via IPC and remote via Infura)
//! - Base Network (local via WebSocket and remote via Alchemy)

use crate::utils::{db_connect::establish_connection, signer::Signer};
use diesel::PgConnection;
use eyre::{Error, Result};
use std::env;

use alloy::{
    network::Ethereum,
    providers::{Provider, ProviderBuilder, RootProvider, WsConnect},
};
use url::Url;

/// Application context holding shared network providers.
///
/// This struct maintains connections to blockchain networks,
/// providing both local and remote access to the Base network.
#[allow(dead_code)]
pub struct AppContext {
    /// Base network connection (either local or remote)
    pub base_provider: RootProvider<Ethereum>,
    /// Database connection
    pub conn: PgConnection,
    /// Transaction signer
    pub signer: Signer,
}

impl AppContext {
    /// Creates a new application context with all configured providers.
    ///
    /// # Returns
    /// * `Result<Self, Error>` - The initialized context or an error
    ///
    /// # Errors
    /// * If any of the provider connections fail
    /// * If required environment variables are missing
    #[allow(dead_code)]
    pub async fn new() -> Result<Self, Error> {
        Ok(Self {
            base_provider: Self::base_provider().await?,
            conn: establish_connection()?,
            signer: Signer::new("/tmp/fly.sock"),
        })
    }

    /// Creates a connection to Base network via Alchemy.
    ///
    /// # Returns
    /// * `Result<RootProvider<Ethereum>, Error>` - The provider
    ///
    /// # Environment Variables
    /// * `FLY_ALCHEMY_API_KEY` - Alchemy API key for Base network access
    ///
    /// # Errors
    /// * If `FLY_ALCHEMY_API_KEY` environment variable is not set
    /// * If URL parsing fails
    /// * If provider initialization fails
    #[allow(dead_code)]
    pub fn base_remote() -> Result<RootProvider<Ethereum>, Error> {
        let api_key = env::var("FLY_ALCHEMY_API_KEY")
            .map_err(|_| Error::msg("FLY_ALCHEMY_API_KEY must be set"))?;

        let url = Url::parse(&format!("https://base-mainnet.g.alchemy.com/v2/{api_key}"))?;
        let provider = ProviderBuilder::new().on_http(url);
        Ok((*provider.root()).clone())
    }

    /// Creates a connection to a local Base node via WebSocket.
    ///
    /// # Returns
    /// * `Result<RootProvider<Ethereum>, Error>` - The provider
    ///
    /// # Path
    /// Connects to WebSocket at `ws://localhost:8546`
    ///
    /// # Errors
    /// * If WebSocket connection fails
    /// * If provider initialization fails
    #[allow(dead_code)]
    pub async fn base_local() -> Result<RootProvider<Ethereum>, Error> {
        let ws = WsConnect::new("ws://localhost:8546");
        let provider = ProviderBuilder::new().on_ws(ws).await?;
        Ok((*provider.root()).clone())
    }

    /// Selects the appropriate Base provider based on environment.
    ///
    /// Uses remote Alchemy provider if FLY_ALCHEMY_API_KEY is set,
    /// otherwise falls back to local WebSocket connection.
    ///
    /// # Returns
    /// * `Result<RootProvider<Ethereum>, Error>` - The selected provider
    ///
    /// # Errors
    /// * If the selected provider connection fails
    #[allow(dead_code)]
    pub async fn base_provider() -> Result<RootProvider<Ethereum>, Error> {
        if env::var("FLY_ALCHEMY_API_KEY").is_ok() {
            Self::base_remote()
        } else {
            Self::base_local().await
        }
    }
}
