#![feature(async_closure)]

use diesel::{r2d2::ConnectionManager, SqliteConnection};
use r2d2::Pool;

pub mod auth;
pub mod config;
pub mod orm;
pub mod schema;
pub mod user;

#[derive(Clone)]
pub struct ServerState {
    pub sqlite_pool: Pool<ConnectionManager<SqliteConnection>>,
}

#[cfg(test)]
pub mod test;
