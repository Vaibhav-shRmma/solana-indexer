use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use serde_json::Value;


#[derive(Debug)]
pub struct TransactionRecord {
    pub signature: String,
    pub slot: i64,
    pub block_time: Option<i64>,
    pub fee: Option<i64>,
    pub success: bool,
    pub program_id: String,
    pub raw_data: Option<Value>,
}


#[derive(Debug)]
pub struct EventRecord {
    pub transaction_id: Uuid,
    pub event_type: String,
    pub slot: i64,
    pub block_time: Option<i64>,
    pub data: Value,
}


pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await?;

    tracing::info!("Connected to PostgreSQL");
    Ok(pool)
}


pub async fn insert_transaction(
    pool: &PgPool,
    record: &TransactionRecord,
) -> Result<Uuid> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO transactions
            (id, signature, slot, block_time, fee, success, program_id, raw_data)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (signature) DO NOTHING
        "#,
        id,
        record.signature,
        record.slot,
        record.block_time,
        record.fee,
        record.success,
        record.program_id,
        record.raw_data,
    )
    .execute(pool)
    .await?;

    Ok(id)
}


pub async fn insert_event(
    pool: &PgPool,
    record: &EventRecord,
) -> Result<Uuid> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO events
            (id, transaction_id, event_type, slot, block_time, data)
        VALUES
            ($1, $2, $3, $4, $5, $6)
        "#,
        id,
        record.transaction_id,
        record.event_type,
        record.slot,
        record.block_time,
        record.data,
    )
    .execute(pool)
    .await?;

    Ok(id)
}


pub async fn upsert_account(
    pool: &PgPool,
    pubkey: &str,
    slot: i64,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO accounts
            (id, pubkey, first_seen_slot, last_seen_slot, transaction_count)
        VALUES
            ($1, $2, $3, $3, 1)
        ON CONFLICT (pubkey) DO UPDATE SET
            last_seen_slot = EXCLUDED.last_seen_slot,
            transaction_count = accounts.transaction_count + 1,
            updated_at = NOW()
        "#,
        Uuid::new_v4(),
        pubkey,
        slot,
    )
    .execute(pool)
    .await?;

    Ok(())
}


pub async fn update_checkpoint(
    pool: &PgPool,
    program_id: &str,
    slot: i64,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO slot_checkpoints (id, program_id, last_slot)
        VALUES ($1, $2, $3)
        ON CONFLICT (program_id) DO UPDATE SET
            last_slot = EXCLUDED.last_slot,
            updated_at = NOW()
        "#,
        Uuid::new_v4(),
        program_id,
        slot,
    )
    .execute(pool)
    .await?;

    Ok(())
}


pub async fn get_checkpoint(
    pool: &PgPool,
    program_id: &str,
) -> Result<i64> {
    let row = sqlx::query!(
        "SELECT last_slot FROM slot_checkpoints WHERE program_id = $1",
        program_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.last_slot).unwrap_or(0))
}