//! Ethereum wallet implementation for tracking ERC20 token balances.
//!
//! This module provides functionality to interact with ERC20 tokens on EVM-compatible chains,
//! allowing balance queries and basic token information retrieval.

use alloy::network::Ethereum;
use alloy::primitives::{Address, U256};
use alloy::providers::RootProvider;
use alloy::sol;
use eyre::{Error, Result};
use std::env;
use std::str::FromStr;

/// A wallet that tracks a single ERC20 token balance.
///
/// The wallet connects to an EVM-compatible chain through a provider and
/// can query token information and balances.
#[derive(Debug)]
pub struct Wallet {
    /// The wallet's address
    address: Address,
    /// The ERC20 token contract address being tracked
    token_address: Address,
    /// The name of the ERC20 token
    #[allow(dead_code)]
    token_name: String,
    /// Network provider for blockchain interactions
    provider: RootProvider<Ethereum>,
    /// Current token balance for the wallet address
    balance: Option<U256>,
}

sol! {
    #[sol(rpc)]
    interface ERC20 {
        function balanceOf(address owner) external view returns (uint256 balance);
        function name() external view returns (string memory);
    }
}

impl Wallet {
    /// Creates a new wallet instance for tracking a specific ERC20 token.
    ///
    /// # Arguments
    /// * `provider` - The network provider for blockchain interactions
    /// * `token_address` - The address of the ERC20 token contract to track
    ///
    /// # Environment Variables
    /// * `FLY_BASE_WALLET_ADDRESS` - The wallet address to track balances for
    ///
    /// # Returns
    /// * `Result<Self>` - The wallet instance
    ///
    /// # Errors
    /// * If `FLY_BASE_WALLET_ADDRESS` environment variable is not set
    /// * If wallet address is invalid
    /// * If token name query fails
    pub async fn new(
        provider: RootProvider<Ethereum>,
        token_address: Address,
    ) -> Result<Self, Error> {
        let address = Address::from_str(&env::var("FLY_BASE_WALLET_ADDRESS")?)?;

        // Get the token name for logging purposes
        let erc20 = ERC20::new(token_address, provider.clone());
        let token_name = erc20.name().call().await?._0;

        Ok(Self {
            address,
            token_address,
            token_name,
            provider,
            balance: None,
        })
    }

    /// Updates the wallet's token balance by querying the blockchain.
    ///
    /// This method fetches the current balance from the ERC20 contract
    /// and stores it in the wallet's state.
    ///
    /// # Returns
    /// * `Result<()>` - Success or failure of balance update
    ///
    /// # Errors
    /// * If balance query to ERC20 contract fails
    pub async fn update_balance(&mut self) -> Result<()> {
        let erc20 = ERC20::new(self.token_address, self.provider.clone());
        self.balance = Some(erc20.balanceOf(self.address).call().await?.balance);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::utils::app_context::AppContext;

    #[tokio::test]
    async fn test_wallet() {
        // WETH on Base
        // let weth = Address::from_str("0x4200000000000000000000000000000000000006").unwrap();

        // let provider = AppContext::base_remote().await.unwrap();
        // let mut wallet = Wallet::new(provider, weth).await.unwrap();

        // wallet.update_balance().await.unwrap();
        // assert!(wallet.balance.is_some());
    }
}
