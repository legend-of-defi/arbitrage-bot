/// Interface for fly executor - a separate process that handles transaction signing
///
/// This (core) service will prepare a bundle of transactions and send them to the signer
/// which will sign and return the signed transactions. The core service will then send the
/// signed transactions to the RPC node.
///
/// This is the implementation of the Privilege Separation Principle.
use alloy::primitives::{Address, U256};
use eyre::{Error, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// An order to be sent to the signer
/// The signer will call something like `IFlySwapper::new(address, provider).call(order)`
/// where `IFlySwapper` is an interface to our smart contract
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    /// The pools to swap through. The order of the pools is important, of course.
    pub pool: Address,
    /// The amount to swap
    pub amount: U256,
    /// Whether the amount is in token0
    pub is_token0: bool,
}

/// A signer for the fly executor
pub struct Signer {
    /// The stream to the signer
    #[allow(dead_code)]
    stream: Option<UnixStream>,
    /// The path to the socket
    #[allow(dead_code)]
    socket_path: String,
}

impl Signer {
    /// Creates a new Signer instance.
    ///
    /// # Returns
    /// * `Result<Self, Error>` - The signer instance
    ///
    /// # Errors
    /// * If socket path is invalid
    #[must_use]
    pub fn new(socket_path: &str) -> Self {
        Self {
            stream: None,
            socket_path: socket_path.to_string(),
        }
    }

    /// Ensure the stream is connected in case the signer is restarted
    ///
    /// # Returns
    /// * `Result<()>` - The result of the call
    ///
    /// # Errors
    /// * `Error::msg("Stream disconnected")` - If the stream is disconnected
    #[allow(dead_code)]
    async fn ensure_connected(&mut self) -> Result<()> {
        if self.stream.is_none() {
            self.stream = Some(UnixStream::connect(&self.socket_path).await?);
        }
        Ok(())
    }

    /// Call the signer with a swap request
    ///
    /// # Returns
    /// * `Result<()>` - The result of the call
    ///
    /// # Errors
    /// * `Error::msg("Stream disconnected")` - If the stream is disconnected
    /// * `Error::msg("Stream not connected")` - If the stream is not connected
    /// * `Error::msg("Failed to reconnect")` - If the stream is not connected and cannot be reconnected
    #[allow(dead_code)]
    pub async fn call(&mut self, msg: &Order) -> Result<()> {
        self.ensure_connected().await?;

        let data = serde_json::to_vec(&msg)?;
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| Error::msg("Stream not connected"))?;

        if stream.write_all(&data).await.is_err() {
            // Connection lost, clear stream and retry once
            self.stream = None;
            self.ensure_connected().await?;
            self.stream
                .as_mut()
                .ok_or_else(|| Error::msg("Failed to reconnect"))?
                .write_all(&data)
                .await?;
        }

        let mut response = vec![0; 1024];
        let n = self
            .stream
            .as_mut()
            .ok_or_else(|| Error::msg("Stream disconnected"))?
            .read(&mut response)
            .await?;

        let response: String = serde_json::from_slice(&response[..n])?;

        match response.as_str() {
            "OK" => Ok(()),
            status => Err(Error::msg(format!("Unexpected status: {status}"))),
        }
    }
}
