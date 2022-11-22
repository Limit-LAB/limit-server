#![feature(type_alias_impl_trait)]

use limit_deps::*;

use anyhow::Context;
use volo_grpc::{Request, Response, Status};

pub use volo_gen::limit::subs::{self, *};

pub struct SubsService;

macro_rules! redis {
    ($req:ident) => {
        $req.extensions()
            .get::<limit_db::RedisClient>()
            .context("no redis extended to service")
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?
            .clone()
    };
}

macro_rules! dbpool {
    ($req:ident) => {
        $req.extensions()
            .get::<limit_db::DBPool>()
            .context("no db extended to service")
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?
            .clone()
    };
}

#[volo::async_trait]
impl subs::SubsService for SubsService {
    async fn get_subscribable_objects(
        &self,
        req: Request<GetObjectsRequest>,
    ) -> Result<Response<GetSubscribableObjectsResponse>, Status> {
        let subs_req = req.get_ref();

        let auth = subs_req.token.as_ref().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;
        let _claim = limit_server_auth::decode_jwt(&auth.jwt)?;

        let redis = redis!(req);

        let pool = dbpool!(req);

        Err(Status::internal("no implementation"))
    }

    async fn get_profiles(
        &self,
        req: Request<GetObjectsRequest>,
    ) -> Result<Response<GetProfilesResponse>, Status> {
        let subs_req = req.get_ref();

        let auth = subs_req.token.as_ref().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;
        let _claim = limit_server_auth::decode_jwt(&auth.jwt)?;

        let redis = redis!(req);

        let pool = dbpool!(req);

        Err(Status::internal("no implementation"))
    }
}
