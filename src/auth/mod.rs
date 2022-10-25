use chrono::{Duration, Utc};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod auth_layer;
pub mod http_layer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JWTClaim {
    // uuid of user
    pub sub: Uuid,
    // expiration
    pub exp: i64,
    // issue at
    pub iat: i64,
}

impl JWTClaim {
    pub fn new(id: uuid::Uuid, expire: Duration) -> Self {
        let iat = Utc::now();
        let exp = iat + expire;

        Self {
            sub: id,
            iat: iat.timestamp(),
            exp: exp.timestamp(),
        }
    }
}

pub fn mock_token(expire: chrono::Duration) -> String {
    // mock config
    crate::config::mock::mock();

    // create mock jwt
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &JWTClaim::new(Uuid::new_v4(), expire),
        &jsonwebtoken::EncodingKey::from_secret(
            crate::config::GLOBAL_CONFIG
                .get()
                .unwrap()
                .jwt_secret
                .as_bytes(),
        ),
    )
    .unwrap()
}
