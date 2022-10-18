use jsonwebtoken::{DecodingKey, Validation};
use std::error::Error;
use tokio_util::sync::ReusableBoxFuture;
use tower::Service;

use super::JWTClaim;

#[derive(Clone)]
/// A middleware that verifies the JWT token
pub struct AuthService<T> {
    /// The inner service
    pub inner: T,
}

impl<T> AuthService<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T, Request> Service<(String, Request)> for AuthService<T>
where
    T: Service<(JWTClaim, Request)>,
    T::Future: Send + 'static,
    T::Error: Into<Box<dyn Error + Send + Sync>> + 'static + Send,
    T::Response: 'static,
{
    type Response = T::Response;
    type Error = Box<dyn Error + Send + Sync>;
    type Future = ReusableBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, (jwt_token, req): (String, Request)) -> Self::Future {
        // first try to decode the token
        jsonwebtoken::decode::<JWTClaim>(
            &jwt_token,
            &DecodingKey::from_secret(
                crate::config::GLOBAL_CONFIG
                    .get()
                    .unwrap()
                    .jwt_secret
                    .as_bytes(),
            ),
            &Validation::default(),
        )
        .map_err(|err| Into::<Self::Error>::into(err))
        .map(|token| {
            tracing::info!("auth service accept token: {:?}", &token.claims);
            // try to validate the token
            if token.claims.exp < chrono::Utc::now().timestamp() {
                tracing::warn!("auth service expired: {:?}", &token.claims);
                ReusableBoxFuture::new(async { Err("TokenExpired".into()) })
            } else {
                // pass the token to the inner service
                let fut = self.inner.call((token.claims, req));
                let res = async move { fut.await.map_err(|err| err.into()) };
                ReusableBoxFuture::new(res)
            }
        })
        .or_else(|err| {
            tracing::error!("auth service reject token: {:?}", err);
            let res = async move { Err(err) };
            Ok::<_, Self::Error>(ReusableBoxFuture::new(res))
        })
        .unwrap()
    }
}

/// A middleware that verifies the JWT token
pub struct AuthLayer;

impl<S> tower::layer::Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService::new(inner)
    }
}

#[test]
fn test_auth_service() {
    use crate::auth::mock_token;
    use crate::test::dummy_services::*;
    use tower::{Layer, Service};

    let jwt = mock_token(chrono::Duration::days(1));

    // create auth service
    let auth_layer = AuthLayer;
    let mut service = auth_layer.layer(DummyService);

    // call service
    let fut = service.call((jwt, ()));
    let res = tokio::runtime::Runtime::new().unwrap().block_on(fut);

    assert!(res.is_ok());
}
