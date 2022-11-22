#![feature(type_alias_impl_trait)]

use limit_deps::*;

use diesel::{r2d2::ConnectionManager, SqliteConnection};
use motore::Service;
use r2d2::Pool;
use volo_grpc::Request;

pub mod event;
pub mod macros;
pub mod orm;
pub mod schema;
pub mod user;

pub type RedisClient = redis::Client;

/// add extension DB to `volo_grpc::Request`
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
                let manager = ConnectionManager::<SqliteConnection>::new(path);
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
    ($pool: expr, $e: expr, $err: expr) => {
        match &$pool {
            limit_db::DBPool::Sqlite(pool) => {
                let conn = pool.get().map_err($err)?;
                $e(conn)
            }
            limit_db::DBPool::Postgres => todo!(),
            limit_db::DBPool::Mysql => todo!(),
        }
    };
}

#[motore::service]
impl<Cx, Req, I> Service<Cx, Request<Req>> for DBService<I>
where
    Req: Send + 'static,
    I: Service<Cx, Request<Req>> + Send + 'static,
    Cx: Send + 'static,
{
    async fn call(&mut self, cx: &mut Cx, mut req: Request<Req>) -> Result<I::Response, I::Error> {
        req.extensions_mut().insert(self.pool.clone());
        req.extensions_mut().insert(self.redis_pool.clone());
        self.inner.call(cx, req).await
    }
}

static GLOBAL_DB_POOL: once_cell::sync::Lazy<DBPool> =
    once_cell::sync::Lazy::new(|| DBPool::new(limit_config::GLOBAL_CONFIG.get().unwrap()));

static GLOBAL_REDIS_CLIENT: once_cell::sync::Lazy<redis::Client> =
    once_cell::sync::Lazy::new(|| redis::Client::open("redis://127.0.0.1:6379/").unwrap());

#[derive(Debug, Clone)]
/// DB Service Layer
pub struct DBLayer;

impl<S> volo::Layer<S> for DBLayer {
    type Service = DBService<S>;

    fn layer(self, inner: S) -> Self::Service {
        DBService {
            inner,
            pool: GLOBAL_DB_POOL.clone(),
            redis_pool: GLOBAL_REDIS_CLIENT.clone(),
        }
    }
}
