#![feature(type_alias_impl_trait)]

use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use limit_config::GLOBAL_CONFIG;
use limit_db::schema::{MESSAGE, MESSAGE_SUBSCRIPTIONS};
use limit_db::{run_sql, RedisClient};
use limit_utils::{execute_background_task, BackgroundTask};
use tokio_util::sync::ReusableBoxFuture;
pub use volo_gen::limit::message::{
    sync::{synchronize_request::*, *},
    *,
};
use volo_grpc::codegen::StreamExt;
use volo_grpc::{Request, Response, Status};

#[derive(Debug, Clone)]
pub struct SynchronizeService;

#[volo::async_trait]
impl volo_gen::limit::message::sync::SynchronizeService for SynchronizeService {
    async fn synchronize(
        &self,
        req: Request<SynchronizeRequest>,
    ) -> Result<Response<SynchronizeResponse>, Status> {
        let sync_req = req.get_ref();

        // check auth is valid
        let auth = sync_req.token.as_ref().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;

        let claim = limit_server_auth::decode_jwt(&auth.jwt)?;
        let ids = claim.sub.split("/").collect::<Vec<_>>();
        let id = ids[1];
        let starting_point = sync_req
            .starting_point
            .as_ref()
            .unwrap_or(&StartingPoint::Timestamp(chrono::Utc::now().timestamp()));
        let offset = match sync_req.offset {
            0 => 50,    // default value of offset
            _ => sync_req.offset
        };

        let pool = req
            .extensions()
            .get::<limit_db::DBPool>()
            .context("no db extended to service")
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?
            .clone();

        let redis = req
            .extensions()
            .get::<RedisClient>()
            .context("no redis extended to service")
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?
            .clone();

        let mut redis_connection = redis.get_connection().map_err(|e| {
            tracing::error!("{}", e);
            Status::internal(e.to_string())
        })?;
        let redis_async_connection = redis.get_async_connection().await.map_err(|e| {
            tracing::error!("{}", e);
            Status::internal(e.to_string())
        })?;
        let subscriptions: Option<Vec<String>> = redis::cmd("GET")
            .arg(format!("{}:subscribed", id))
            .query(&mut redis_connection)
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?;

        if let Some(subscriptions) = subscriptions {
            // todo
        } else {
            // todo
        }

        Err(Status::internal("no implementation"))
    }
}
