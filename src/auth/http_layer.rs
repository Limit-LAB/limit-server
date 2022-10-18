use axum::{
    headers::{authorization::Bearer, Authorization, HeaderMapExt},
    http::{Request, Response},
};
use hyper::{Body, StatusCode};
use std::{convert::Infallible, error::Error};
use tokio_util::sync::ReusableBoxFuture;
use tower::Service;

#[derive(Clone)]
pub struct AuthHttpService<T> {
    /// The inner service
    pub inner: T,
}

impl<T> AuthHttpService<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<ReqBody, T> Service<Request<ReqBody>> for AuthHttpService<T>
where
    ReqBody: Send + 'static,
    T: Service<(String, Request<ReqBody>)>,
    T::Future: Send + 'static,
    T::Error: Into<Box<dyn Error + Send + Sync>> + 'static,
    T::Response: Into<Response<Body>> + 'static,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = ReusableBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            .map_err(|err| panic!("{:#?}", err.into()))
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        req.headers().typed_get::<Authorization<Bearer>>().map_or(
            ReusableBoxFuture::new(async {
                tracing::warn!("No Authorization header found");
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::from("NoAuthorizationHeader"))
                        .unwrap(),
                )
            }),
            |Authorization(header)| {
                let res = self.inner.call((header.token().to_string(), req));
                let res = async move {
                    Ok(match res.await {
                        Ok(res) => res.into(),
                        Err(err) => Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .body(Body::from(err.into().to_string()))
                            .unwrap(),
                    })
                };
                ReusableBoxFuture::new(res)
            },
        )
    }
}

#[derive(Clone)]
pub struct AuthHttpLayer;

impl<S> tower::layer::Layer<S> for AuthHttpLayer {
    type Service = AuthHttpService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthHttpService::new(inner)
    }
}
