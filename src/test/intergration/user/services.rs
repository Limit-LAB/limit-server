use crate::schema::{USER, USER_LOGIN_PASSCODE, USER_PRIVACY_SETTINGS};
use crate::user::model::*;
use crate::user::services::UserLoginRequest;
use axum::{routing::get, Extension, Router};
use diesel::RunQueryDsl;
use diesel::{r2d2::ConnectionManager, SqliteConnection};
use hyper::StatusCode;
use r2d2::Pool;

use crate::{user::services::verify_and_auth_user, ServerState};

pub async fn test_verify_and_auth_user(pool: Pool<ConnectionManager<SqliteConnection>>) {
    tracing::info!("💪 test {} started", module_path!());
    crate::test::intergration::do_with_port(|p| async move {
        tracing::info!("🚀 test {} on port {}", module_path!(), p);
        let id = uuid::Uuid::new_v4().to_string();

        let user = User {
            id: id.clone(),
            pubkey: "test".to_string(),
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
            passcode: "123456".to_string(),
        };
        let res = reqwest::Client::new()
            .get(&server_addr.clone())
            .json(&login_request)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        println!("{:?}", res.text().await);

        // wrong passcode
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
        println!("{:?}", res.text().await);

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
        println!("{:?}", res.text().await);

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
        println!("{:?}", res.text().await);
        server.abort();
        tracing::info!("🎉test {} finished🎉", module_path!());
    })
    .await
    .await;
}
