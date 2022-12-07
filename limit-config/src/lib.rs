use std::{net::SocketAddr, path::PathBuf};

use limit_deps::{url::Url, *};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
pub static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "limit_deps::serde")]
pub enum Database {
    Sqlite { path: PathBuf },
    Postgres { url: Url },
    Mysql { url: Url },
}

/// Deploy mode of the server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "limit_deps::serde")]
pub enum DeployMode {
    /// standalone mode
    StandAlone {
        /// bind address
        addr: SocketAddr,
    },
    /// master node of cluster
    Master {
        /// bind address
        addr: SocketAddr,
        /// Url of slave nodes
        slaves: Vec<Url>,
    },
    /// the url of the master
    Slave { master: Url },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "limit_deps::serde")]
pub enum Metrics {
    /// prometheus metrics
    Prometheus { url: Url },
    /// both influxdb and victoria metrics
    InfluxDB { url: Url },
    /// directly print metrics to terminal
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "limit_deps::serde")]
pub struct Agent {
    /// agent bind address
    pub addr: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "limit_deps::serde")]
pub struct Config {
    /// server url
    pub url: String,

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

    /// per user message on-the-fly limit
    /// default is 100
    pub per_user_message_on_the_fly_limit: usize,

    /// http agent setting
    pub agent: Agent,
}
