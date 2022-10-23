use once_cell::sync::OnceCell;

use serde::{Deserialize, Serialize};

pub static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

pub mod mock {
    use uuid::Uuid;

    use crate::auth::JWTClaim;

    use super::*;
    pub fn mock() {
        GLOBAL_CONFIG.get_or_init(|| {
            let (server_secret_key, server_public_key) = limit_am::create_random_secret().unwrap();
            Config {
                database: Database::Sqlite {
                    path: "mock.db".to_string(),
                },
                jwt_secret: "mock".to_string(),
                database_pool_thread_count: 3,
                admin_jwt: jsonwebtoken::encode(
                    &jsonwebtoken::Header::default(),
                    &JWTClaim::new(Uuid::new_v4(), chrono::Duration::days(1)),
                    &jsonwebtoken::EncodingKey::from_secret("mock_admin".as_bytes()),
                )
                .unwrap(),
                metrics: Metrics::Terminal,
                server_secret_key,
                server_public_key,
            }
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Database {
    Sqlite { path: String },
    Postgres { url: String },
    Mysql { url: String },
}

/// Deploy mode of the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeployMode {
    /// standalone mode
    StandAlone { ip: String },
    /// master node of cluster, the url of slave nodes
    Master { ip: String, slaves_ip: Vec<String> },
    /// the url of the master
    Slave { master_ip: String },
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
    /// Database connection pool thread count
    /// default is 3
    pub database_pool_thread_count: usize,
    /// metrics config
    pub metrics: Metrics,
    /// remember to set this to a random string
    /// also reset when you update the server
    pub jwt_secret: String,
    /// generated when you first run the server
    pub admin_jwt: String,
    /// server secret key
    pub server_secret_key: String,
    /// server public key
    pub server_public_key: String,
}
