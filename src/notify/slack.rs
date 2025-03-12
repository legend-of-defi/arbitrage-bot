use eyre::Result;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// Slack notifier
#[derive(Debug)]
#[allow(dead_code)]
pub struct SlackNotifier {
    /// The Slack OAuth token
    token: String,
    /// The HTTP client
    client: Client,
}

#[allow(dead_code)]
impl SlackNotifier {
    /// Create a new Slack notifier
    pub fn new() -> Result<Self> {
        let token = std::env::var("SLACK_OAUTH_TOKEN")
            .map_err(|_| eyre::eyre!("SLACK_OAUTH_TOKEN not set"))?;

        // Create a client with a timeout
        let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

        Ok(Self { token, client })
    }

    /// Send a message to a specific channel
    pub async fn send_to(&self, msg: &str, channel: &str) -> Result<()> {
        let payload = json!({
            "channel": channel,
            "text": msg,
            "username": "Fly Bot",
            "icon_emoji": ":rocket:"
        });

        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.token)
            .json(&payload)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        // Remove debug print in production
        // println!("Response: {:?}", response);

        // Check if Slack API returned success
        if !response["ok"].as_bool().unwrap_or(false) {
            return Err(eyre::eyre!(
                "Slack API error: {}",
                response["error"].as_str().unwrap_or("unknown error")
            ));
        }

        Ok(())
    }

    /// Send a message to the default channel
    pub async fn send(&self, msg: &str) -> Result<()> {
        self.send_to(msg, "#fly").await
    }

    /// Send an error message to the error channel
    pub async fn send_error(&self, error: &str) -> Result<()> {
        self.send_to(&format!(":warning: Error: {error}"), "#fly-errors")
            .await
    }
}
