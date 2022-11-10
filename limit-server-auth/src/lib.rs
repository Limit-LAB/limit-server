#![feature(type_alias_impl_trait)]

use jsonwebtoken::Algorithm;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::Validation;
pub use volo_gen::limit::auth::*;

use anyhow::Context;
use chrono::{Duration, Utc};
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use limit_config::GLOBAL_CONFIG;
use limit_db::run_sql;
use limit_db::schema::USER;
use limit_db::schema::USER_LOGIN_PASSCODE;
use limit_db::schema::USER_PRIVACY_SETTINGS;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use volo_grpc::{Request, Response, Status};

#[derive(Debug, Clone, PartialEq)]
pub struct JWTSub {
    pub id: Uuid,
    pub device_id: String,
}

impl JWTSub {
    pub fn to_sub(&self) -> String {
        format!("{}/{}", self.device_id, self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JWTClaim {
    // device_id/uuid
    pub sub: String,
    // expiration
    pub exp: i64,
    // issue at
    pub iat: i64,
}

impl JWTClaim {
    pub fn new(sub: JWTSub, expire: Duration) -> Self {
        let iat = Utc::now();
        let exp = iat + expire;

        Self {
            sub: sub.to_sub(),
            iat: iat.timestamp(),
            exp: exp.timestamp(),
        }
    }
}

pub fn decode_jwt(token: &str) -> Result<JWTClaim, Status> {
    let validate = Validation::new(Algorithm::HS256);
    jsonwebtoken::decode::<JWTClaim>(
        token,
        &DecodingKey::from_secret(GLOBAL_CONFIG.get().unwrap().jwt_secret.as_bytes()),
        &validate,
    )
    .map_err(|e| {
        tracing::error!("{}", e);
        Status::unauthenticated(e.to_string())
    })
    .map(|token| token.claims)
}
pub fn encode_jwt(claim: JWTClaim) -> Result<String, Status> {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claim,
        &jsonwebtoken::EncodingKey::from_secret(GLOBAL_CONFIG.get().unwrap().jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!("{}", e);
        Status::internal(e.to_string())
    })
}

#[test]
fn test_encode_decode() {
    use limit_test_utils::mock_config;
    mock_config();
    let claim = JWTClaim::new(
        JWTSub {
            id: Uuid::new_v4(),
            device_id: "test".to_string(),
        },
        Duration::days(1),
    );
    let token = encode_jwt(claim.clone()).unwrap();
    let decoded = decode_jwt(&token).unwrap();
    assert_eq!(claim, decoded);
}

fn generate_random_passcode() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut passcode = String::new();
    let pool = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'A', 'B',
        'C', 'D', 'E', 'F', '!', '@', '#', '$', '%', '^', '&', '*', '_', '=', '+',
    ];
    for _ in 0..6 {
        passcode.push(pool[rng.gen_range(0..33)]);
    }
    passcode
}

/// requires DB connection
pub struct AuthService;

#[volo::async_trait]
impl volo_gen::limit::auth::AuthService for AuthService {
    async fn request_auth(
        &self,
        req: Request<RequestAuthRequest>,
    ) -> Result<Response<RequestAuthResponse>, Status> {
        tracing::info!("request_auth: {:?}", req.get_ref().id);
        let pool = req
            .extensions()
            .get::<limit_db::DBPool>()
            .context("no db extended to service")
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?
            .clone();
        let id = &req.get_ref().id;
        let passcode = generate_random_passcode();

        // update random passcode for user
        let sql = diesel::update(USER_LOGIN_PASSCODE::table)
            .filter(USER_LOGIN_PASSCODE::ID.eq(&id))
            .set(USER_LOGIN_PASSCODE::PASSCODE.eq(&passcode));

        let res = Ok(Response::new(RequestAuthResponse {
            rand_text: passcode.clone(),
        }));

        let _row_effected = run_sql!(
            pool,
            |mut conn| {
                sql.execute(&mut conn).map_err(|e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                })
            },
            |e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            }
        )?;
        res
    }

    async fn do_auth(&self, req: Request<DoAuthRequest>) -> Result<Response<Auth>, Status> {
        tracing::info!(
            "do auth: {:?} at {:?}",
            req.get_ref().id,
            req.get_ref().device_id
        );
        let pool = req
            .extensions()
            .get::<limit_db::DBPool>()
            .context("no db extended to service")
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?
            .clone();
        let id = &req.get_ref().id;
        let passcode = &req.get_ref().validated;
        // get needed user info
        let sql_get_user_info = USER::table
            .inner_join(USER_PRIVACY_SETTINGS::table)
            .inner_join(USER_LOGIN_PASSCODE::table)
            .filter(USER::ID.eq(&id))
            .select((
                USER::ID,
                USER::SHAREDKEY,
                USER_LOGIN_PASSCODE::PASSCODE,
                USER_PRIVACY_SETTINGS::JWT_EXPIRATION,
            ));
        // update random passcode for user
        let sql_update_tmp_passcode = diesel::update(USER_LOGIN_PASSCODE::table)
            .filter(USER_LOGIN_PASSCODE::ID.eq(&id))
            .set(USER_LOGIN_PASSCODE::PASSCODE.eq(generate_random_passcode()));

        let (id, sharedkey, expected_passcode, duration) = run_sql!(
            pool,
            |mut conn| {
                sql_get_user_info
                    .first::<(String, String, String, String)>(&mut conn)
                    .map_err(|e| {
                        tracing::error!("{}", e);
                        Status::internal(e.to_string())
                    })
            },
            |e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            }
        )?;

        let uuid = uuid::Uuid::parse_str(&id).unwrap();
        let expire =
            chrono::Duration::from_std(std::time::Duration::from_secs(duration.parse().unwrap()))
                .unwrap();
        let decrypted =
            limit_am::aes256_decrypt_string(&sharedkey, passcode.as_str()).map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?;

        if decrypted == expected_passcode {
            tracing::info!("user login success: id: {}", id);
            let jwt = encode_jwt(JWTClaim::new(
                JWTSub {
                    id: uuid,
                    device_id: req.get_ref().device_id.clone(),
                },
                expire,
            ))?;
            // update random passcode for user
            let _row_effected = run_sql!(
                pool,
                |mut conn| {
                    sql_update_tmp_passcode.execute(&mut conn).map_err(|e| {
                        tracing::error!("{}", e);
                        Status::internal(e.to_string())
                    })
                },
                |e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                }
            )?;

            Ok(Response::new(Auth { jwt }))
        } else {
            // invalid passcode
            tracing::warn!("invalid passcode for id: {}", id);
            Err(Status::unauthenticated("invalid passcode"))
        }
    }
}
