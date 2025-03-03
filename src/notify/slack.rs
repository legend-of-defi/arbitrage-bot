use eyre::Result;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

#[derive(Debug)]
pub struct SlackNotifier {
    token: String,
    client: Client,
}

impl SlackNotifier {
    pub fn new() -> Result<Self> {
        let token = std::env::var("SLACK_OAUTH_TOKEN")
            .map_err(|_| eyre::eyre!("SLACK_OAUTH_TOKEN not set"))?;

        // Create a client with a timeout
        let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

        Ok(Self { token, client })
    }

    pub async fn send_to(&self, msg: &str, channel: &str) -> Result<()> {
        let payload = json!({
            "channel": channel,
            "text": msg,
            "username": "Fly Bot",
            "icon_emoji": ":rocket:"
        });

        // Remove debug print in production
        // println!("Using token: {}", &self.token);

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

    pub async fn send(&self, msg: &str) -> Result<()> {
        self.send_to(msg, "#fly").await
    }

    pub async fn send_error(&self, error: &str) -> Result<()> {
        self.send_to(&format!(":warning: Error: {error}"), "#fly-errors")
            .await
    }
}
