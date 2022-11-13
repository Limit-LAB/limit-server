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
        Err(Status::internal("no implementation"))
    }
}
