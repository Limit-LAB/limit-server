#![feature(string_remove_matches)]

use anyhow::Context;
use chrono::{Duration, Utc};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use limit_config::GLOBAL_CONFIG;
use limit_db::{
    get_db_layer, run_sql,
    schema::{USER, USER_LOGIN_PASSCODE, USER_PRIVACY_SETTINGS},
};
use limit_deps::{metrics::increment_counter, *};
use limit_utils::{execute_background_task, BackgroundTask, Measurement};
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};
pub use tonic_gen::auth::*;
use uuid::Uuid;

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
#[serde(crate = "limit_deps::serde")]
pub struct JWTClaim {
    /// device_id/uuid
    pub sub: String,
    /// expiration
    pub exp: i64,
    /// issue at
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
    const POOL: &[u8] = &[
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e',
        b'f', b'A', b'B', b'C', b'D', b'E', b'F', b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*',
        b'_', b'=', b'+',
    ];
    const POOL_SIZE: usize = POOL.len();

    use rand::Rng;

    let mut rng = rand::thread_rng();
    let mut passcode = [0; 6];

    passcode
        .iter_mut()
        .for_each(|b| *b = POOL[rng.gen_range(0..POOL_SIZE)]);

    // SAFETY: passcode is generated from a pool of ascii bytes
    unsafe { std::str::from_utf8_unchecked(&passcode) }.to_string()
}

#[test]
fn test_generate_random_passcode() {
    let passcode = generate_random_passcode();
    println!("{passcode}");
    assert_eq!(passcode.len(), 6);
}

/// requires DB connection
pub struct AuthService;

#[tonic::async_trait]
impl tonic_gen::auth::auth_service_server::AuthService for AuthService {
    async fn request_auth(
        &self,
        req: Request<RequestAuthRequest>,
    ) -> Result<Response<RequestAuthResponse>, Status> {
        tracing::info!("request_auth: {:?}", req.get_ref().id);
        let mut m = Measurement::start("request_auth_generate_passcode");
        let (_, redis, pool) = get_db_layer!(req);

        let id = req.get_ref().id.clone();
        let _id_guard = id.parse::<Uuid>().map_err(|e| {
            tracing::error!("{}", e);
            Status::invalid_argument(e.to_string())
        })?;

        let passcode = generate_random_passcode();

        m.renew("request_auth_update_cache");

        // update random passcode for user
        let _update_cache = redis::cmd("SET")
            .arg(format!("{id}:passcode"))
            .arg(&passcode)
            .execute(&mut redis.get_connection().map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?);

        m.renew("request_auth_update_diesel");

        let sql = diesel::update(USER_LOGIN_PASSCODE::table)
            .filter(USER_LOGIN_PASSCODE::ID.eq(id))
            .set(USER_LOGIN_PASSCODE::PASSCODE.eq(passcode.clone()));

        execute_background_task(BackgroundTask::new(
            "request_auth_update_user_passcode_db",
            async move {
                run_sql!(
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
                )
            },
        ))
        .await;

        m.end();

        Ok(Response::new(RequestAuthResponse {
            rand_text: passcode.clone(),
        }))
    }

    async fn do_auth(&self, req: Request<DoAuthRequest>) -> Result<Response<Auth>, Status> {
        tracing::info!(
            "do auth: {:?} at {:?}",
            req.get_ref().id,
            req.get_ref().device_id
        );
        let mut m = Measurement::start("do_auth_load_auth");
        let (_, mut redis, pool) = get_db_layer!(req);
        let id = req.get_ref().id.clone();
        let uuid = uuid::Uuid::parse_str(&id).map_err(|e| {
            tracing::error!("{}", e);
            Status::invalid_argument(e.to_string())
        })?;
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

        let res: (Option<String>, Option<String>, Option<String>) = redis::pipe()
            .cmd("GET")
            .arg(format!("{id}:sharedkey"))
            .cmd("GET")
            .arg(format!("{id}:passcode"))
            .cmd("GET")
            .arg(format!("{id}:duration"))
            .query(&mut redis)
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?;
        let (sharedkey, expected_passcode, duration) = if let (Some(sk), Some(ep), Some(dur)) = res
        {
            increment_counter!("do_auth_cache_hit");
            tracing::info!("do_auth: cache hit for id {:?}", id);
            (sk, ep, dur)
        } else {
            increment_counter!("do_auth_cache_miss");
            tracing::info!("do_auth: cache miss for id {:?}", id);
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
            // update cache
            redis::pipe()
                .cmd("SET")
                .arg(format!("{id}:sharedkey"))
                .arg(&sharedkey)
                .cmd("SET")
                .arg(format!("{id}:passcode"))
                .arg(&expected_passcode)
                .cmd("SET")
                .arg(format!("{id}:duration"))
                .arg(&duration)
                .query(&mut redis)
                .map_err(|e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                })?;
            (sharedkey, expected_passcode, duration)
        };

        let expire =
            chrono::Duration::from_std(std::time::Duration::from_secs(duration.parse().unwrap()))
                .unwrap();
        let decrypted =
            limit_am::aes256_decrypt_string(&sharedkey, passcode.as_str()).map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?;

        if decrypted == expected_passcode {
            m.renew("do_auth_gen_token");
            tracing::info!("user login success: id: {}", id);
            let jwt = encode_jwt(JWTClaim::new(
                JWTSub {
                    id: uuid,
                    device_id: req.get_ref().device_id.clone(),
                },
                expire,
            ))?;
            // update random passcode for user
            let _update_cache = redis::cmd("SET")
                .arg(format!("{id}:passcode"))
                .arg(passcode)
                .execute(&mut redis);
            let sql = diesel::update(USER_LOGIN_PASSCODE::table)
                .filter(USER_LOGIN_PASSCODE::ID.eq(id))
                .set(USER_LOGIN_PASSCODE::PASSCODE.eq(generate_random_passcode()));

            execute_background_task(BackgroundTask::new(
                "do_auth_update_user_passcode_db",
                async move {
                    run_sql!(
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
                    )
                },
            ))
            .await;
            m.end();
            Ok(Response::new(Auth { jwt }))
        } else {
            // invalid passcode
            tracing::warn!("invalid passcode for id: {}", id);
            m.end();
            Err(Status::unauthenticated("invalid passcode"))
        }
    }
}
