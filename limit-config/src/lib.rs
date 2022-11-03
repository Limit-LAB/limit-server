use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
pub static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

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
