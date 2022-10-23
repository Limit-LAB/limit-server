use std::time::Duration;

use axum::{extract::Extension, Json};
use diesel::{QueryDsl, RunQueryDsl};
use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth::JWTClaim,
    schema::{USER, USER_LOGIN_PASSCODE, USER_PRIVACY_SETTINGS},
    ServerState,
};

/// although i am not expecting get user by its id
pub fn get_user_by_id() {}

/// update user's random passcode in db
/// send user random passcode encrypted with user's pubkey
pub fn user_login_request() {
    // let pubkey = RsaPublicKey::from_public_key_der(&pubkey.as_bytes()).unwrap();
    // let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
    // let decoded = pubkey.decrypt(&encrypted_passcode.as_bytes(), padding).unwrap();
    // let decrypted_passcode = todo!();
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct UserLoginRequest {
    pub id: String,
    pub passcode: String,
}

/// get user's pubkey
/// first try validate id
/// decrypt user's random passcode
/// verify by comparing with db user's random passcode
/// if verified, return user's jwt with user's jwt_expire time
/// else 401
pub async fn verify_and_auth_user(
    Extension(state): Extension<ServerState>,
    Json(UserLoginRequest { id, passcode }): Json<UserLoginRequest>,
) -> Response<Body> {
    tracing::info!("user login request: id: {}, passcode: {}", id, passcode);
    use diesel::ExpressionMethods;
    uuid::Uuid::parse_str(&id)
        .map(|_| {
            state
                .sqlite_pool
                .get()
                .map(|mut con| {
                    USER::table
                        .inner_join(USER_PRIVACY_SETTINGS::table)
                        .inner_join(USER_LOGIN_PASSCODE::table)
                        .filter(USER::ID.eq(&id))
                        .select((
                            USER::ID,
                            USER::SHAREDKEY,
                            USER_LOGIN_PASSCODE::PASSCODE,
                            USER_PRIVACY_SETTINGS::JWT_EXPIRATION,
                        ))
                        .first::<(String, String, String, String)>(&mut con)
                        .map(|(id, sharedkey, expected_passcode, duration)| {
                            let uuid = uuid::Uuid::parse_str(&id).unwrap();
                            let expire = chrono::Duration::from_std(Duration::from_secs(
                                duration.parse().unwrap(),
                            ))
                            .unwrap();
                            let decrypted =
                                limit_am::aes256_decrypt_string(&sharedkey, passcode.as_str());
                            if let Err(err) = decrypted {
                                tracing::error!("decrypt error: {}", err);
                                return Response::builder()
                                    .status(StatusCode::UNAUTHORIZED)
                                    .body(Body::empty())
                                    .unwrap();
                            }
                            if decrypted.unwrap() == expected_passcode {
                                tracing::info!("user login success: id: {}", id);
                                let jwt = jsonwebtoken::encode(
                                    &jsonwebtoken::Header::default(),
                                    &JWTClaim::new(uuid, expire),
                                    &jsonwebtoken::EncodingKey::from_secret(
                                        crate::config::GLOBAL_CONFIG
                                            .get()
                                            .unwrap()
                                            .jwt_secret
                                            .as_bytes(),
                                    ),
                                )
                                .unwrap();
                                // update random passcode for user
                                diesel::update(USER_LOGIN_PASSCODE::table)
                                    .filter(USER_LOGIN_PASSCODE::ID.eq(id))
                                    .set(
                                        USER_LOGIN_PASSCODE::PASSCODE
                                            .eq(crate::auth::generate_random_passcode()),
                                    )
                                    .execute(&mut con)
                                    .unwrap();
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .body(Body::from(jwt))
                                    .unwrap()
                            } else {
                                // invalid passcode
                                tracing::warn!("invalid passcode for id: {}", id);
                                Response::builder()
                                    .status(StatusCode::UNAUTHORIZED)
                                    .body(Body::empty())
                                    .unwrap()
                            }
                        })
                        // no such user
                        .unwrap_or_else(|_| {
                            tracing::warn!("no such user with id: {}", id);
                            Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::empty())
                                .unwrap()
                        })
                })
                // no db connection
                .unwrap_or_else(|_| {
                    tracing::error!("database went down!");
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap()
                })
        })
        // thats not a valid uuid
        .unwrap_or_else(|_| {
            tracing::warn!("invalid user id: {}", id);
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap()
        })
}
