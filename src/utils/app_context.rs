//! Application context for managing blockchain network connections.
//!
//! This module provides a centralized way to manage connections to different
//! Ethereum-compatible networks, including both local and remote providers.
//! It supports connections to:
//! - Ethereum Mainnet (local via IPC and remote via Infura)
//! - Base Network (local via WebSocket and remote via Alchemy)

use crate::utils::signer::Signer;
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
};
use alloy::providers::{Identity, RootProvider};
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use eyre::{Error, Result};
use log::info;
use std::env;

use alloy::{
    network::Ethereum,
    providers::{ProviderBuilder, WsConnect},
};

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
    /// Transaction signer
    pub signer: Signer,
    /// Diesel async connection pool
    pub db: diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>,
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
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://fly:fly@/tmp/fly".to_string());

        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
        let pool = Pool::builder(config).build().map_err(|e| eyre::eyre!(e))?;

        // Create base provider using the existing method
        let base_provider = Self::create_new_provider().await?;

        Ok(Self {
            base_provider,
            base_provider_websocket_url: Self::base_provider_websocket_url(),
            signer: Signer::new("/tmp/fly.sock"),
            db: pool,
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
    /// * `Result<impl Provider<Ethereum>>` - The provider
    ///
    /// # Errors
    /// * If connection fails
    /// * If provider initialization fails
    pub async fn create_new_provider() -> Result<EthereumProvider> {
        if let Ok(api_key) = env::var("FLY_ALCHEMY_API_KEY") {
            info!("Using remote provider with API key {}", api_key);
            let ws_url =
                "wss://base-mainnet.g.alchemy.com/v2/pzwXUHHsvHjgeSCT5rW_whOyYo7kas4d".to_string();
            let ws = WsConnect::new(&ws_url);
            Ok(ProviderBuilder::new().on_ws(ws).await?)
        } else if let Ok(ws_url) = env::var("RPC_WS_URL") {
            info!("Using WebSocket provider at {}", ws_url);
            let ws = WsConnect::new(&ws_url);
            Ok(ProviderBuilder::new().on_ws(ws).await?)
        } else {
            let ws_url = Self::base_provider_websocket_url();
            info!("Using WebSocket provider at {}", ws_url);
            let ws = WsConnect::new(&ws_url);
            Ok(ProviderBuilder::new().on_ws(ws).await?)
        }
    }
}
