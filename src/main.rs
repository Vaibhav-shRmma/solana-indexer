mod api;
mod config;
mod db;
mod listener;
mod parser;

use anyhow::Result;
use dotenv::dotenv;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = config::AppConfig::load()?;
    info!("Starting Solana Indexer...");

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = db::create_pool(&database_url, config.database.max_connections).await?;
    info!("Database ready!");

    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

    let ws_url = config.solana.ws_url.clone();
    let program_id = config.solana.program_id.clone();

    tokio::spawn(async move {
        if let Err(e) = listener::start_listener(ws_url, program_id, tx).await {
            tracing::error!("Listener failed: {}", e);
        }
    });

    let parser = parser::Parser::new(
        config.solana.rpc_url.clone(),
        config.solana.program_id.clone(),
    );

    let api_state = Arc::new(api::AppState {
        pool: pool.clone(),
        program_id: config.solana.program_id.clone(),
        start_time: chrono::Utc::now(),
    });

    let api_port = config.api.port;
    tokio::spawn(async move {
        let router = api::create_router(api_state);
        let addr = format!("0.0.0.0:{}", api_port);
        info!("API server running on http://localhost:{}", api_port);
       let addr = addr.parse().unwrap();

    axum::Server::bind(&addr)
       .serve(router.into_make_service())
       .await
       .unwrap();
    });

    info!("Listening for Raydium transactions...");

    while let Some(raw_tx) = rx.recv().await {
        info!("slot: {} | sig: {}...{}",
            raw_tx.slot,
            &raw_tx.signature[..8],
            &raw_tx.signature[raw_tx.signature.len()-8..]
        );

        match parser.parse(&raw_tx).await {
            Ok(Some(parsed)) => {
                match db::insert_transaction(&pool, &parsed.transaction).await {
                    Ok(tx_id) => {
                        info!("saved tx | events: {}", parsed.events.len());

                        for mut event in parsed.events {
                            event.transaction_id = tx_id;
                            if let Err(e) = db::insert_event(&pool, &event).await {
                                tracing::warn!("failed to insert event: {}", e);
                            }
                        }

                        for pubkey in &parsed.account_keys {
                            let _ = db::upsert_account(&pool, pubkey, parsed.transaction.slot).await;
                        }

                        let _ = db::update_checkpoint(
                            &pool,
                            &config.solana.program_id,
                            parsed.transaction.slot
                        ).await;
                    }
                    Err(e) => tracing::warn!("failed to insert tx: {}", e),
                }
            }
            Ok(None) => tracing::warn!("could not parse tx"),
            Err(e) => tracing::warn!("parse error: {}", e),
        }
    }

    Ok(())
}