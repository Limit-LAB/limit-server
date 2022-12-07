use std::net::SocketAddr;

use limit_deps::{
    once_cell::sync::OnceCell,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "limit_deps::serde")]
pub struct Config {
    /// agent bind address
    pub addr: SocketAddr,
}

static GLOBAL: OnceCell<Config> = OnceCell::new();

impl Config {
    pub fn init(self) {
        GLOBAL.set(self).expect("Global config already set");
    }

    pub fn get<'a>() -> &'a Config {
        GLOBAL.get().expect("Global config not initialized")
    }
}
