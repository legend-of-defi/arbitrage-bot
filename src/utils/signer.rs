use eyre::{Error, Result};
/// Interface for fly executor - a separate process that handles transaction signing
///
/// This (core) service will prepare a bundle of transactions and send them to the signer
/// which will sign and return the signed transactions. The core service will then send the
/// signed transactions to the RPC node.
///
/// This is the implementation of the Privilege Separation Principle.
use alloy::primitives::{Address, U256};
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
    pub amount: U256,
    pub is_token0: bool,
}

pub struct Signer {
    stream: Option<UnixStream>,
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
    /// * `Result<(), Error>` - The result of the call
    ///
    /// # Errors
    /// * `Error::msg("Stream disconnected")` - If the stream is disconnected
    async fn ensure_connected(&mut self) -> Result<(), Error> {
        if self.stream.is_none() {
            self.stream = Some(UnixStream::connect(&self.socket_path).await?);
        }
        Ok(())
    }

    /// Call the signer with a swap request
    ///
    /// # Returns
    /// * `Result<(), Error>` - The result of the call
    ///
    /// # Errors
    /// * `Error::msg("Stream disconnected")` - If the stream is disconnected
    /// * `Error::msg("Stream not connected")` - If the stream is not connected
    /// * `Error::msg("Failed to reconnect")` - If the stream is not connected and cannot be reconnected
    pub async fn call(&mut self, msg: &Order) -> Result<(), Error> {
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
