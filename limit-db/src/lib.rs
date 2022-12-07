#![allow(non_snake_case)]

use std::task::{Context, Poll};

use diesel::{r2d2::ConnectionManager, SqliteConnection};
use limit_deps::{hyper::Body, tonic::body::BoxBody, *};
use r2d2::Pool;
use tower::Service;

pub mod event;
pub mod macros;
pub mod orm;
pub mod user;

pub mod schema {
    use limit_deps::diesel;

    include!("schema.rs");
}

pub type RedisClient = redis::Client;

/// add extension DB to `hyper::Request`
#[derive(Clone)]
pub struct DBService<Inner> {
    inner: Inner,
    pool: DBPool,
    redis_pool: RedisClient,
}

#[derive(Debug, Clone)]
pub enum DBPool {
    Sqlite(Pool<ConnectionManager<SqliteConnection>>),
    Postgres,
    Mysql,
}

impl DBPool {
    pub fn new(config: &limit_config::Config) -> Self {
        match &config.database {
            limit_config::Database::Sqlite { path } => {
                let manager = ConnectionManager::<SqliteConnection>::new(
                    path.to_str().expect("Invalid sqlite path"),
                );
                let pool = Pool::builder()
                    .test_on_check_out(true)
                    .build(manager)
                    .expect("Could not build connection pool");
                Self::Sqlite(pool)
            }
            limit_config::Database::Postgres { url } => todo!("{}", url),
            limit_config::Database::Mysql { url } => todo!("{}", url),
        }
    }
}

#[macro_export]
macro_rules! run_sql {
    ($pool:expr, $e:expr, $err:expr) => {{
        let d = std::time::Instant::now();
        match &$pool {
            limit_db::DBPool::Sqlite(pool) => {
                let conn = pool.get().map_err($err)?;
                let res = $e(conn);
                metrics::histogram!("database_sqlite_execution", d.elapsed());
                res
            }
            limit_db::DBPool::Postgres => todo!(),
            limit_db::DBPool::Mysql => todo!(),
        }
    }};
}

static GLOBAL_DB_POOL: once_cell::sync::Lazy<DBPool> =
    once_cell::sync::Lazy::new(|| DBPool::new(limit_config::GLOBAL_CONFIG.get().unwrap()));

static GLOBAL_REDIS_CLIENT: once_cell::sync::Lazy<redis::Client> =
    once_cell::sync::Lazy::new(|| redis::Client::open("redis://127.0.0.1:6379/").unwrap());

#[derive(Debug, Clone)]
/// DB Service Layer
pub struct DBLayer;

impl<S> Service<hyper::Request<Body>> for DBService<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        req.extensions_mut().insert(self.pool.clone());
        req.extensions_mut().insert(self.redis_pool.clone());

        Box::pin(inner.call(req))
    }
}

impl<S> tower::Layer<S> for DBLayer {
    type Service = DBService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DBService {
            inner,
            pool: GLOBAL_DB_POOL.clone(),
            redis_pool: GLOBAL_REDIS_CLIENT.clone(),
        }
    }
}
