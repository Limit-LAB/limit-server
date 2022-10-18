use std::{convert::Infallible, error::Error};

use crate::auth::JWTClaim;
use axum::response::Response;
use hyper::{Body, Request};
use tokio_util::sync::ReusableBoxFuture;
use tower::Service;

#[derive(Clone)]
pub struct DummyService;

impl<Request> Service<Request> for DummyService {
    type Response = ();
    type Error = Box<dyn Error + Send + Sync>;

    type Future = ReusableBoxFuture<'static, Result<(), Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request) -> Self::Future {
        ReusableBoxFuture::new(async { Ok(()) })
    }
}

#[derive(Clone)]
pub struct DummyAuthService;

impl Service<(JWTClaim, Request<Body>)> for DummyAuthService {
    type Response = Response<Body>;

    type Error = Infallible;

    type Future = ReusableBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: (JWTClaim, Request<Body>)) -> Self::Future {
        ReusableBoxFuture::new(async {
            tower::service_fn(|_: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .body(Body::from("Hello authed user!"))
                        .unwrap(),
                )
            })
            .call(req.1)
            .await
        })
    }
}
