#![feature(type_alias_impl_trait)]

use limit_deps::*;

use anyhow::Context;
use limit_db::get_db_layer;
use volo_grpc::{Request, Response, Status};

pub use volo_gen::limit::subs::{self, *};

pub struct SubsService;

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

        let (_, _, _) = get_db_layer!(req);

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

        let (_, _, _) = get_db_layer!(req);

        Err(Status::internal("no implementation"))
    }
}
