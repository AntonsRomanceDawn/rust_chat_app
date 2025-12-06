use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::instrument;
use uuid::Uuid;

use crate::{database::db::Db, dtos::ServerResp};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub access_expiry: i64,
    pub refresh_expiry: i64,
}

impl Config {
    #[instrument]
    pub fn init() -> Config {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let access_expiry: i64 = std::env::var("ACCESS_TOKEN_EXPIRY")
            .expect("ACCESS_TOKEN_EXPIRY must be set")
            .parse()
            .expect("ACCESS_TOKEN_EXPIRY must be a valid u64");
        let refresh_expiry: i64 = std::env::var("REFRESH_TOKEN_EXPIRY")
            .expect("REFRESH_TOKEN_EXPIRY must be set")
            .parse()
            .expect("REFRESH_TOKEN_EXPIRY must be a valid u64");

        Config {
            database_url,
            jwt_secret,
            port: 3000,
            access_expiry,
            refresh_expiry,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Arc<Db>,
    pub channels: Arc<DashMap<Uuid, mpsc::UnboundedSender<ServerResp>>>,
}
