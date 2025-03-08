use eyre::Result;
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use super::app_context::AppContext;

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
/// * If `WEBSOCKET_URL` environment variable is not set
pub async fn send_ws_request(
    ctx: &AppContext,
    request: String,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let (mut ws_stream, _) = connect_async(ctx.base_provider_websocket_url.clone()).await?;
    // Send the request
    ws_stream.send(Message::Text(request)).await?;
    // Return the stream for continued use
    Ok(ws_stream)
}
