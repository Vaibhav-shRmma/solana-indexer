use anyhow::Result;
use config::{Config, File, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct SolanaConfig {
    pub ws_url: String,
    pub rpc_url: String,
    pub program_id: String,
}

#[derive(Debug, Deserialize, Clone )]
pub struct DatabaseConfig{
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig{
    pub solana: SolanaConfig,
    pub database: DatabaseConfig,
    pub api: ApiConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self>{
        let config = Config::builder()
        .add_source(File::with_name("config/default"))
        .add_source(Environment::default().separator("__"))
        .build()?;

    let app_config = config.try_deserialize::<AppConfig>()?;
    Ok(app_config)
    }
}