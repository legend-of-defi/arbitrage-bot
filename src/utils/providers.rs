use alloy::{
    providers::ProviderBuilder,
    rpc::client::ClientBuilder
};
use alloy::network::Ethereum;
use alloy::providers::fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller};
use alloy::providers::{Identity, IpcConnect, RootProvider};
use alloy::rpc::client::RpcClient;
use crate::config::Config;
use eyre::{Error, Result, Report};

/// Creates a new IPC provider for Ethereum network communication
///
/// # Returns
/// A configured provider with all necessary fillers
///
/// # Errors
/// * If IPC connection fails
/// * If provider initialization fails
/// * If environment variables are invalid
#[allow(dead_code)]
pub async fn create_ipc_provider() -> Result<FillProvider<JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, RootProvider, Ethereum>, Error> {
    let config = Config::from_env();
    let ipc = IpcConnect::new(config.ipc_path);
    let provider = ProviderBuilder::new().on_ipc(ipc).await?;
    Ok(provider)
}

/// Creates a new HTTP provider for Ethereum network communication
///
/// # Returns
/// A configured provider with all necessary fillers
///
/// # Errors
/// * If HTTP connection fails
/// * If provider initialization fails
/// * If environment variables are invalid
///
/// # Panics
/// * If RPC URL cannot be parsed
#[allow(dead_code)]
pub async fn create_http_provider() -> Result<FillProvider<JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, RootProvider, Ethereum>, Error> {
    let config = Config::from_env();
    let provider = ProviderBuilder::new().on_http(config.rpc_url.parse().unwrap());
    Ok(provider)
}

/// Creates a new RPC client for direct Ethereum RPC calls
///
/// # Returns
/// A configured RPC client
///
/// # Errors
/// * If client creation fails
/// * If environment variables are invalid
///
/// # Panics
/// * If RPC URL cannot be parsed
#[allow(dead_code)]
pub fn create_rpc_provider() -> Result<RpcClient, Error> {
    let config = Config::from_env();
    let client = ClientBuilder::default().http(config.rpc_url.parse()?);
    Ok(client)
}

use futures::SinkExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;

/// Sends a WebSocket request to the specified endpoint
///
/// # Arguments
/// * `request` - The request string to send
///
/// # Returns
/// A WebSocket stream connection
///
/// # Errors
/// * If WebSocket connection fails
/// * If TLS handshake fails
/// * If connection URL is invalid
/// * If message sending fails
///
/// # Panics
/// * If WEBSOCKET_URL environment variable is not set
pub async fn send_ws_request(request: String)
    -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Report> {
    let websocket_url = std::env::var("WEBSOCKET_URL").expect("WEBSOCKET_URL not set");
    // Connect to WebSocket
    let (mut ws_stream, _) = connect_async(websocket_url).await?;
    // Send the request
    ws_stream.send(Message::Text(request)).await?;
    // Return the stream for continued use
    Ok(ws_stream)
}