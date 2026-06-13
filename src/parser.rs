use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{error, info, warn};

use crate::db::{EventRecord, TransactionRecord};
use crate::listener::RawTransaction;

#[derive(Debug)]
pub struct ParsedTransaction {
    pub transaction: TransactionRecord,
    pub events: Vec<EventRecord>,
    pub account_keys: Vec<String>,
}

pub struct Parser {
    pub rpc_url: String,
    pub http_client: Client,
    pub program_id: String,
}

impl Parser {
    pub fn new(rpc_url: String, program_id: String) -> Self {
        Self {
            rpc_url,
            http_client: Client::new(),
            program_id,
        }
    }

    pub async fn parse(&self, raw: &RawTransaction) -> Result<Option<ParsedTransaction>> {
        let tx_detail = self.fetch_transaction(&raw.signature).await?;

        let tx_detail = match tx_detail {
            Some(d) => d,
            None => {
                warn!("Transaction not found: {}", raw.signature);
                return Ok(None);
            }
        };

        let slot = raw.slot as i64;
        let block_time = tx_detail["blockTime"].as_i64();
        let fee = tx_detail["meta"]["fee"].as_i64();
        let err = &tx_detail["meta"]["err"];
        let success = err.is_null();

        let account_keys = self.extract_account_keys(&tx_detail);

        let transaction = TransactionRecord {
            signature: raw.signature.clone(),
            slot,
            block_time,
            fee,
            success,
            program_id: self.program_id.clone(),
            raw_data: Some(tx_detail.clone()),
        };

        let events = self.decode_logs(&raw.logs, slot, block_time);

        Ok(Some(ParsedTransaction {
            transaction,
            events,
            account_keys,
        }))
    }

    async fn fetch_transaction(&self, signature: &str) -> Result<Option<Value>> {
    use tokio::time::{sleep, Duration};

    for attempt in 1..=5 {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                signature,
                {
                    "encoding": "jsonParsed",
                    "maxSupportedTransactionVersion": 0
                }
            ]
        });

        let response = self.http_client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        if !response["result"].is_null() {
            return Ok(Some(response["result"].clone()));
        }

        warn!(
            "Transaction {} not available yet (attempt {}/5)",
            signature,
            attempt
        );

        sleep(Duration::from_millis(500)).await;
    }

    warn!(
        "Transaction {} still unavailable after 5 retries",
        signature
    );

    Ok(None)
}

    fn extract_account_keys(&self, tx_detail: &Value) -> Vec<String> {
        tx_detail["transaction"]["message"]["accountKeys"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|k| {
                        k["pubkey"].as_str()
                            .or_else(|| k.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn decode_logs(
        &self,
        logs: &[String],
        slot: i64,
        block_time: Option<i64>,
    ) -> Vec<EventRecord> {
        let mut events = Vec::new();

        for log in logs {
            if log.contains("ray_log") {
                events.push(EventRecord {
                    transaction_id: uuid::Uuid::nil(), 
                    event_type: "swap".to_string(),
                    slot,
                    block_time,
                    data: json!({ "raw_log": log }),
                });
            }
            else if log.contains("Program log: initialize") ||
                    log.contains("Program log: deposit") {
                events.push(EventRecord {
                    transaction_id: uuid::Uuid::nil(),
                    event_type: "add_liquidity".to_string(),
                    slot,
                    block_time,
                    data: json!({ "raw_log": log }),
                });
            }

            else if log.contains("Program log: withdraw") {
                events.push(EventRecord {
                    transaction_id: uuid::Uuid::nil(),
                    event_type: "remove_liquidity".to_string(),
                    slot,
                    block_time,
                    data: json!({ "raw_log": log }),
                });
            }
        }

        events
    }
}