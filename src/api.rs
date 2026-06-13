use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;

pub struct AppState {
    pub pool: PgPool,
    pub program_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct TransactionQuery {
    account: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct EventQuery {
    event_type: Option<String>,
    limit: Option<i64>,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/transactions", get(transactions_handler))
        .route("/accounts/:pubkey/history", get(account_history_handler))
        .route("/events", get(events_handler))
        .route("/stats", get(stats_handler))
        .with_state(state)
}

async fn health_handler(State(state): State<Arc<AppState>>) -> Json<Value> {
    let row = sqlx::query!(
        "SELECT last_slot FROM slot_checkpoints WHERE program_id = $1",
        state.program_id
    )
    .fetch_optional(&state.pool)
    .await;

    let last_slot = match row {
        Ok(Some(r)) => r.last_slot,
        _ => 0,
    };

    let uptime = chrono::Utc::now()
        .signed_duration_since(state.start_time)
        .num_seconds();

    Json(json!({
        "status": "ok",
        "program_id": state.program_id,
        "last_indexed_slot": last_slot,
        "uptime_seconds": uptime,
    }))
}

async fn transactions_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TransactionQuery>,
) -> Json<Value> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    if let Some(account) = params.account {
        let rows = sqlx::query!(
            r#"
            SELECT t.signature, t.slot, t.block_time, t.fee, t.success
            FROM transactions t
            JOIN accounts a ON a.pubkey = $1
            WHERE t.program_id = $2
            ORDER BY t.slot DESC
            LIMIT $3 OFFSET $4
            "#,
            account,
            state.program_id,
            limit,
            offset,
        )
        .fetch_all(&state.pool)
        .await;

        match rows {
            Ok(records) => {
                let txns: Vec<Value> = records
                    .iter()
                    .map(|r| json!({
                        "signature": r.signature,
                        "slot": r.slot,
                        "block_time": r.block_time,
                        "fee": r.fee,
                        "success": r.success,
                    }))
                    .collect();

                Json(json!({
                    "transactions": txns,
                    "count": txns.len()
                }))
            }
            Err(e) => Json(json!({ "error": e.to_string() })),
        }
    } else {
        let rows = sqlx::query!(
            r#"
            SELECT signature, slot, block_time, fee, success
            FROM transactions
            WHERE program_id = $1
            ORDER BY slot DESC
            LIMIT $2 OFFSET $3
            "#,
            state.program_id,
            limit,
            offset,
        )
        .fetch_all(&state.pool)
        .await;

        match rows {
            Ok(records) => {
                let txns: Vec<Value> = records
                    .iter()
                    .map(|r| json!({
                        "signature": r.signature,
                        "slot": r.slot,
                        "block_time": r.block_time,
                        "fee": r.fee,
                        "success": r.success,
                    }))
                    .collect();

                Json(json!({
                    "transactions": txns,
                    "count": txns.len()
                }))
            }
            Err(e) => Json(json!({ "error": e.to_string() })),
        }
    }
}

async fn account_history_handler(
    State(state): State<Arc<AppState>>,
    Path(pubkey): Path<String>,
) -> Json<Value> {
    let account = sqlx::query!(
        "SELECT pubkey, first_seen_slot, last_seen_slot, transaction_count FROM accounts WHERE pubkey = $1",
        pubkey
    )
    .fetch_optional(&state.pool)
    .await;

    match account {
        Ok(Some(a)) => Json(json!({
            "pubkey": a.pubkey,
            "first_seen_slot": a.first_seen_slot,
            "last_seen_slot": a.last_seen_slot,
            "transaction_count": a.transaction_count,
        })),
        Ok(None) => Json(json!({ "error": "account not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn events_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventQuery>,
) -> Json<Value> {
    let limit = params.limit.unwrap_or(20).min(100);

    if let Some(event_type) = params.event_type {
        let rows = sqlx::query!(
            r#"
            SELECT e.event_type, e.slot, e.block_time, e.data
            FROM events e
            JOIN transactions t ON t.id = e.transaction_id
            WHERE e.event_type = $1 AND t.program_id = $2
            ORDER BY e.slot DESC
            LIMIT $3
            "#,
            event_type,
            state.program_id,
            limit,
        )
        .fetch_all(&state.pool)
        .await;

        match rows {
            Ok(records) => {
                let events: Vec<Value> = records
                    .iter()
                    .map(|r| json!({
                        "event_type": r.event_type,
                        "slot": r.slot,
                        "block_time": r.block_time,
                        "data": r.data,
                    }))
                    .collect();

                Json(json!({
                    "events": events,
                    "count": events.len()
                }))
            }
            Err(e) => Json(json!({ "error": e.to_string() })),
        }
    } else {
        let rows = sqlx::query!(
            r#"
            SELECT e.event_type, e.slot, e.block_time, e.data
            FROM events e
            JOIN transactions t ON t.id = e.transaction_id
            WHERE t.program_id = $1
            ORDER BY e.slot DESC
            LIMIT $2
            "#,
            state.program_id,
            limit,
        )
        .fetch_all(&state.pool)
        .await;

        match rows {
            Ok(records) => {
                let events: Vec<Value> = records
                    .iter()
                    .map(|r| json!({
                        "event_type": r.event_type,
                        "slot": r.slot,
                        "block_time": r.block_time,
                        "data": r.data,
                    }))
                    .collect();

                Json(json!({
                    "events": events,
                    "count": events.len()
                }))
            }
            Err(e) => Json(json!({ "error": e.to_string() })),
        }
    }
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> Json<Value> {
    let tx_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM transactions WHERE program_id = $1",
        state.program_id
    )
    .fetch_one(&state.pool)
    .await;

    let account_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM accounts"
    )
    .fetch_one(&state.pool)
    .await;

    let event_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM events"
    )
    .fetch_one(&state.pool)
    .await;

    let success_rate = sqlx::query!(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE success = true) as successful,
            COUNT(*) as total
        FROM transactions
        WHERE program_id = $1
        "#,
        state.program_id
    )
    .fetch_one(&state.pool)
    .await;

    Json(json!({
        "transactions": tx_count.map(|r| r.count).unwrap_or(None),
        "accounts": account_count.map(|r| r.count).unwrap_or(None),
        "events": event_count.map(|r| r.count).unwrap_or(None),
        "success_rate": success_rate.map(|r| {
            if r.total.unwrap_or(0) > 0 {
                r.successful.unwrap_or(0) as f64 / r.total.unwrap_or(1) as f64
            } else { 0.0 }
        }).unwrap_or(0.0),
    }))
}