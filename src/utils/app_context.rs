//! Application context for managing blockchain network connections.
//!
//! This module provides a centralized way to manage connections to different
//! Ethereum-compatible networks, including both local and remote providers.
//! It supports connections to:
//! - Ethereum Mainnet (local via IPC and remote via Infura)
//! - Base Network (local via WebSocket and remote via Alchemy)

use crate::utils::{db_connect::establish_connection, signer::Signer};
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
};
use alloy::providers::{Identity, RootProvider};
use diesel::PgConnection;
use eyre::{Error, Report, Result};
use log::info;
use std::env;

use alloy::{
    network::Ethereum,
    providers::{ProviderBuilder, WsConnect},
};
use url::Url;

// There has to be a better way to do this
type EthereumProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
    Ethereum,
>;

/// Application context holding shared network providers and connections.
pub struct AppContext {
    /// Base network provider (local or remote)
    pub base_provider: EthereumProvider,
    /// WebSocket URL for Base network
    pub base_provider_websocket_url: String,
    /// `PostgreSQL` database connection
    pub pg_connection: PgConnection,
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
    pub async fn new() -> Result<Self, Error> {
        // Create base provider using the existing method
        let base_provider = Self::create_new_provider().await?;

        Ok(Self {
            base_provider,
            base_provider_websocket_url: Self::base_provider_websocket_url(),
            pg_connection: establish_connection()?,
            signer: Signer::new("/tmp/fly.sock"),
        })
    }

    pub fn base_provider_websocket_url() -> String {
        "ws://localhost:8546".to_string()
    }

    /// Creates a new provider based on environment
    ///
    /// This returns a concrete provider type suitable for contract calls.
    ///
    /// # Returns
    /// * `Result<impl Provider<Ethereum>, Report>` - The provider
    ///
    /// # Errors
    /// * If connection fails
    /// * If provider initialization fails
    pub async fn create_new_provider() -> Result<EthereumProvider, Report> {
        if let Ok(api_key) = env::var("FLY_ALCHEMY_API_KEY") {
            info!("Using remote provider with API key {}", api_key);
            let url = Url::parse(&format!("https://base-mainnet.g.alchemy.com/v2/{api_key}"))?;
            Ok(ProviderBuilder::new().on_http(url))
        } else {
            let ws_url = Self::base_provider_websocket_url();
            info!("Using WebSocket provider at {}", ws_url);
            let ws = WsConnect::new(&ws_url);
            Ok(ProviderBuilder::new().on_ws(ws).await?)
        }
    }
}
