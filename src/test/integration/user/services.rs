use crate::config::GLOBAL_CONFIG;
use crate::schema::{USER, USER_LOGIN_PASSCODE, USER_PRIVACY_SETTINGS};
use crate::user::model::*;
use crate::user::services::{user_login_request, UserLoginRequest, UserRequestLoginRequest};
use axum::{routing::get, Extension, Router};
use diesel::RunQueryDsl;
use diesel::{r2d2::ConnectionManager, SqliteConnection};
use hyper::StatusCode;
use r2d2::Pool;

use crate::{user::services::verify_and_auth_user, ServerState};

pub async fn test_verify_and_auth_user(pool: Pool<ConnectionManager<SqliteConnection>>) {
    tracing::info!("💪 test {} started", module_path!());
    crate::test::integration::do_with_port(|p| async move {
        tracing::info!("🚀 test {} on port {}", module_path!(), p);
        let id = uuid::Uuid::new_v4().to_string();
        let (user_sec_key, user_pubkey) = limit_am::create_random_secret().unwrap();
        let pubkey = limit_am::decode_public(&user_pubkey).unwrap();
        let shared_key = limit_am::key_exchange(
            limit_am::decode_secret(&GLOBAL_CONFIG.get().unwrap().server_secret_key).unwrap(),
            pubkey,
        );
        assert_eq!(
            shared_key,
            limit_am::key_exchange(
                limit_am::decode_secret(&user_sec_key).unwrap(),
                limit_am::decode_public(&GLOBAL_CONFIG.get().unwrap().server_public_key).unwrap()
            )
        );

        let user = User {
            id: id.clone(),
            pubkey: user_pubkey,
            sharedkey: shared_key.clone(),
        };

        let user_privacy_settings = PrivacySettings {
            id: id.clone(),
            avatar: crate::orm::Visibility::from(Visibility::Private).0,
            last_seen: crate::orm::Visibility::from(Visibility::Private).0,
            groups: crate::orm::Visibility::from(Visibility::Private).0,
            forwards: crate::orm::Visibility::from(Visibility::Private).0,
            jwt_expiration: crate::orm::Duration::from(std::time::Duration::from_secs(114514)).0,
        };
        let user_login_passcode = UserLoginPasscode {
            id: id.clone(),
            passcode: "123456".to_string(),
        };

        diesel::insert_into(USER::table)
            .values(user)
            .execute(&mut pool.get().unwrap())
            .unwrap();
        diesel::insert_into(USER_PRIVACY_SETTINGS::table)
            .values(user_privacy_settings)
            .execute(&mut pool.get().unwrap())
            .unwrap();
        diesel::insert_into(USER_LOGIN_PASSCODE::table)
            .values(user_login_passcode)
            .execute(&mut pool.get().unwrap())
            .unwrap();

        let app = Router::new()
            .route("/", get(verify_and_auth_user))
            .layer(Extension(ServerState { sqlite_pool: pool }));
        crate::config::mock::mock();

        let server = tokio::spawn(async move {
            axum::Server::bind(&format!("0.0.0.0:{}", p).parse().unwrap())
                .serve(app.into_make_service())
                .await
                .unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let server_addr = format!("http://localhost:{}", p);

        // correct
        let login_request = UserLoginRequest {
            id: id.clone(),
            passcode: limit_am::aes256_encrypt_string(&shared_key, "123456").unwrap(),
        };
        let res = reqwest::Client::new()
            .get(&server_addr.clone())
            .json(&login_request)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        tracing::info!("{:?}", res.text().await);

        // wrong passcode
        let login_request = UserLoginRequest {
            id: id.clone(),
            passcode: limit_am::aes256_encrypt_string(&shared_key, "123457").unwrap(),
        };
        let res = reqwest::Client::new()
            .get(&server_addr.clone())
            .json(&login_request)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        tracing::info!("{:?}", res.text().await);

        // failed to decrypt
        let login_request = UserLoginRequest {
            id: id.clone(),
            passcode: "1234567".to_string(),
        };
        let res = reqwest::Client::new()
            .get(&server_addr.clone())
            .json(&login_request)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        tracing::info!("{:?}", res.text().await);

        // no such user
        let login_request = UserLoginRequest {
            id: uuid::Uuid::new_v4().to_string(),
            passcode: "123456".to_string(),
        };
        let res = reqwest::Client::new()
            .get(&server_addr.clone())
            .json(&login_request)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        tracing::info!("{:?}", res.text().await);

        // not even uuid
        let login_request = UserLoginRequest {
            id: "fuck you!".to_string(),
            passcode: "123456".to_string(),
        };
        let res = reqwest::Client::new()
            .get(&server_addr.clone())
            .json(&login_request)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        tracing::info!("{:?}", res.text().await);
        server.abort();
        tracing::info!("🎉test {} finished🎉", module_path!());
    })
    .await
    .await;
}

pub async fn test_integrate_auth_user(pool: Pool<ConnectionManager<SqliteConnection>>) {
    tracing::info!("💪 test {} started", module_path!());
    crate::test::integration::do_with_port(|p| async move {
        tracing::info!("🚀 test {} on port {}", module_path!(), p);
        let id = uuid::Uuid::new_v4().to_string();
        let (user_sec_key, user_pubkey) = limit_am::create_random_secret().unwrap();
        let pubkey = limit_am::decode_public(&user_pubkey).unwrap();
        let shared_key = limit_am::key_exchange(
            limit_am::decode_secret(&GLOBAL_CONFIG.get().unwrap().server_secret_key).unwrap(),
            pubkey,
        );
        assert_eq!(
            shared_key,
            limit_am::key_exchange(
                limit_am::decode_secret(&user_sec_key).unwrap(),
                limit_am::decode_public(&GLOBAL_CONFIG.get().unwrap().server_public_key).unwrap()
            )
        );

        let user = User {
            id: id.clone(),
            pubkey: user_pubkey,
            sharedkey: shared_key.clone(),
        };

        let user_privacy_settings = PrivacySettings {
            id: id.clone(),
            avatar: crate::orm::Visibility::from(Visibility::Private).0,
            last_seen: crate::orm::Visibility::from(Visibility::Private).0,
            groups: crate::orm::Visibility::from(Visibility::Private).0,
            forwards: crate::orm::Visibility::from(Visibility::Private).0,
            jwt_expiration: crate::orm::Duration::from(std::time::Duration::from_secs(114514)).0,
        };
        let user_login_passcode = UserLoginPasscode {
            id: id.clone(),
            passcode: "123456".to_string(),
        };

        diesel::insert_into(USER::table)
            .values(user)
            .execute(&mut pool.get().unwrap())
            .unwrap();
        diesel::insert_into(USER_PRIVACY_SETTINGS::table)
            .values(user_privacy_settings)
            .execute(&mut pool.get().unwrap())
            .unwrap();
        diesel::insert_into(USER_LOGIN_PASSCODE::table)
            .values(user_login_passcode)
            .execute(&mut pool.get().unwrap())
            .unwrap();

        let app = Router::new()
            .route("/req", get(user_login_request))
            .layer(Extension(ServerState {
                sqlite_pool: pool.clone(),
            }))
            .route("/", get(verify_and_auth_user))
            .layer(Extension(ServerState { sqlite_pool: pool }));

        let server = tokio::spawn(async move {
            axum::Server::bind(&format!("0.0.0.0:{}", p).parse().unwrap())
                .serve(app.into_make_service())
                .await
                .unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let server_addr = format!("http://localhost:{}", p);

        let re = UserRequestLoginRequest { id: id.clone() };
        let res = reqwest::Client::new()
            .get(format!("{}/req", server_addr))
            .json(&re)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let res = res.text().await.unwrap();
        tracing::info!("got {:?}", res);

        let login_request = UserLoginRequest {
            id: id.clone(),
            passcode: limit_am::aes256_encrypt_string(&shared_key, &res).unwrap(),
        };
        let res = reqwest::Client::new()
            .get(&server_addr.clone())
            .json(&login_request)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        tracing::info!("{:?}", res.text().await);

        server.abort();
        tracing::info!("🎉test {} finished🎉", module_path!());
    })
    .await
    .await;
}
