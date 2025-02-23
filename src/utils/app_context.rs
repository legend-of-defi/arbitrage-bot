//! Application context for managing blockchain network connections.
//!
//! This module provides a centralized way to manage connections to different
//! Ethereum-compatible networks, including both local and remote providers.
//! It supports connections to:
//! - Ethereum Mainnet (local via IPC and remote via Infura)
//! - Base Network (local via IPC and remote via Alchemy)

use crate::utils::{db_connect::establish_connection, signer::Signer};
use diesel::PgConnection;
use eyre::{Error, Result};
use std::env;

use alloy::{
    network::Ethereum,
    providers::{IpcConnect, Provider, ProviderBuilder, RootProvider},
};
use url::Url;

/// Application context holding shared network providers.
///
/// This struct maintains connections to different blockchain networks,
/// providing both local and remote access to Ethereum and Base networks.
#[allow(dead_code)]
pub struct AppContext {
    /// Local Base node connection via IPC
    pub base_local: RootProvider<Ethereum>,
    /// Remote Base network connection via Alchemy
    pub base_remote: RootProvider<Ethereum>,
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
            base_local: Self::base_local().await?,
            base_remote: Self::base_remote()?,
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

    /// Creates a connection to a local Base node via IPC.
    ///
    /// # Returns
    /// * `Result<RootProvider<Ethereum>, Error>` - The provider
    ///
    /// # Path
    /// Uses the IPC socket at `/opt/base/data/geth.ipc`
    ///
    /// # Errors
    /// * If IPC socket connection fails
    /// * If provider initialization fails
    #[allow(dead_code)]
    pub async fn base_local() -> Result<RootProvider<Ethereum>, Error> {
        let ipc_path = "/opt/base/data/geth.ipc";
        let ipc = IpcConnect::new(ipc_path.to_string());
        let provider = ProviderBuilder::new().on_ipc(ipc).await?;
        Ok((*provider.root()).clone())
    }
}
