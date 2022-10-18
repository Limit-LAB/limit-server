use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
pub static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

pub mod mock {
    use uuid::Uuid;

    use crate::auth::JWTClaim;

    use super::*;
    pub fn mock() {
        GLOBAL_CONFIG.get_or_init(|| Config {
            database: Database::Sqlite {
                path: "mock.db".to_string(),
            },
            jwt_secret: "mock".to_string(),
            admin_jwt: jsonwebtoken::encode(
                &jsonwebtoken::Header::default(),
                &JWTClaim::new(Uuid::new_v4(), chrono::Duration::days(1)),
                &jsonwebtoken::EncodingKey::from_secret("mock_admin".as_bytes()),
            )
            .unwrap(),
            metrics: Metrics::Terminal,
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Database {
    Sqlite { path: String },
    Postgres { url: String },
    Mysql { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Metrics {
    /// prometheus metrics
    Prometheus { url: String },
    /// both influxdb and victoria metrics
    InfluxDB { url: String },
    /// directly print metrics to terminal
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Database config
    pub database: Database,
    /// metrics config
    pub metrics: Metrics,
    /// remember to set this to a random string
    /// also reset when you update the server
    pub jwt_secret: String,
    /// generated when you first run the server
    pub admin_jwt: String,
}
