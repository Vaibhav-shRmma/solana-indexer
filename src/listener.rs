use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};
use url::Url;


#[derive(Debug, Clone)]
pub struct RawTransaction {
    pub signature: String,
    pub slot: u64,
    pub logs: Vec<String>,
}

pub async fn start_listener(
    ws_url: String,
    program_id: String,
    tx: Sender<RawTransaction>,  
) -> Result<()> {
    loop {
        info!("Connecting to Solana WebSocket...");

        match run_listener(&ws_url, &program_id, tx.clone()).await {
            Ok(_) => {
                warn!("WebSocket closed cleanly, reconnecting in 5s...");
            }
            Err(e) => {
                error!("WebSocket error: {}. Reconnecting in 5s...", e);
            }
        }


        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn run_listener(
    ws_url: &str,
    program_id: &str,
    tx: Sender<RawTransaction>,
) -> Result<()> {
    let url = Url::parse(ws_url)?;
    let (ws_stream, _) = connect_async(url).await?;
    info!("WebSocket connected!");

    let (mut write, mut read) = ws_stream.split();

   
    let subscribe_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            { "mentions": [program_id] },
            { "commitment": "confirmed" }
        ]
    });

    write.send(Message::Text(subscribe_msg.to_string())).await?;
    info!("Subscribed to program logs: {}", program_id);

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    if json["result"].is_number() {
                        info!("Subscription confirmed, ID: {}", json["result"]);
                        continue;
                    }

                    if let Some(params) = json["params"].as_object() {
                        if let Some(result) = params.get("result") {
                            if let Some(raw_tx) = parse_log_notification(result) {

                                if tx.send(raw_tx).await.is_err() {
                                    error!("Pipeline channel closed, stopping listener");
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Ping(data)) => {

                write.send(Message::Pong(data)).await?;
            }
            Ok(Message::Close(_)) => {
                warn!("Server closed the connection");
                break;
            }
            Err(e) => {
                error!("WebSocket message error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_log_notification(result: &Value) -> Option<RawTransaction> {
    let value = result.get("value")?;

    let signature = value["signature"].as_str()?.to_string();
    let slot = result["context"]["slot"].as_u64()?;
    let logs = value["logs"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    Some(RawTransaction {
        signature,
        slot,
        logs,
    })
}