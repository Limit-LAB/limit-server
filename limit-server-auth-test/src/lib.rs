use std::{future::Future, net::SocketAddr, pin::Pin};

use limit_deps::*;

use diesel::RunQueryDsl;
use limit_config::GLOBAL_CONFIG;
use limit_db::{run_sql, schema::*, DBLayer, DBPool};
use limit_server_auth::{
    AuthService, AuthServiceClientBuilder, AuthServiceServer, DoAuthRequest, RequestAuthRequest,
};
use limit_test_utils::{do_with_port, test_service, test_tasks};

pub async fn test_request_auth(port: u16) -> anyhow::Result<()> {
    tracing::info!("\t- test {}::test_request_auth started", module_path!());

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let client = AuthServiceClientBuilder::new(module_path!())
        .address(addr)
        .build();
    let id = uuid::Uuid::new_v4().to_string();

    let rand_str = client.clone().request_auth(RequestAuthRequest { id }).await;

    tracing::info!("rand_str: {:?}", rand_str);
    assert!(rand_str.is_ok());

    tracing::info!("\t- test {}::test_request_auth finished", module_path!());
    Ok(())
}

pub async fn test_do_auth(port: u16) -> anyhow::Result<()> {
    tracing::info!("\t- test {}::test_do_auth started", module_path!());

    let id = uuid::Uuid::new_v4().to_string();
    let device_id = uuid::Uuid::new_v4().to_string();
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

    let user = limit_db::user::User {
        id: id.clone(),
        pubkey: user_pubkey,
        sharedkey: shared_key.clone(),
    };

    let user_privacy_settings = limit_db::user::PrivacySettings {
        id: id.clone(),
        avatar: limit_db::orm::Visibility::from(limit_db::user::Visibility::Private).0,
        last_seen: limit_db::orm::Visibility::from(limit_db::user::Visibility::Private).0,
        groups: limit_db::orm::Visibility::from(limit_db::user::Visibility::Private).0,
        forwards: limit_db::orm::Visibility::from(limit_db::user::Visibility::Private).0,
        jwt_expiration: limit_db::orm::Duration::from(std::time::Duration::from_secs(114514)).0,
    };
    let user_login_passcode = limit_db::user::UserLoginPasscode {
        id: id.clone(),
        passcode: "123456".to_string(),
    };

    // set up db
    let config = || {
        let pool = DBPool::new(limit_config::GLOBAL_CONFIG.get().unwrap());
        run_sql!(
            pool,
            |mut con| diesel::insert_into(USER::table)
                .values(user)
                .execute(&mut con)
                .unwrap(),
            |e| {
                tracing::error!("Error: {}", e);
            }
        );
        run_sql!(
            pool,
            |mut con| diesel::insert_into(USER_PRIVACY_SETTINGS::table)
                .values(user_privacy_settings)
                .execute(&mut con)
                .unwrap(),
            |e| {
                tracing::error!("Error: {}", e);
            }
        );
        run_sql!(
            pool,
            |mut con| diesel::insert_into(USER_LOGIN_PASSCODE::table)
                .values(user_login_passcode)
                .execute(&mut con)
                .unwrap(),
            |e| {
                tracing::error!("Error: {}", e);
            }
        );
        Ok::<(), ()>(())
    };
    config().unwrap();

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let mut client = AuthServiceClientBuilder::new(module_path!())
        .address(addr)
        .build();

    // passcode is correct
    let passcode = limit_am::aes256_encrypt_string(&shared_key, "123456").unwrap();
    let res = client
        .do_auth(DoAuthRequest {
            id: id.clone(),
            device_id: device_id.clone(),
            validated: passcode,
        })
        .await;
    tracing::info!("res: {:?}", res);
    assert!(res.is_ok());

    // passcode is incorrect
    let passcode = limit_am::aes256_encrypt_string(&shared_key, "1234567").unwrap();
    let res = client
        .do_auth(DoAuthRequest {
            id: id.clone(),
            device_id: device_id.clone(),
            validated: passcode,
        })
        .await;
    tracing::info!("res: {:?}", res);
    assert!(res.is_err());

    // passcode is empty
    let passcode = "".to_string();
    let res = client
        .do_auth(DoAuthRequest {
            id: id.clone(),
            device_id: device_id.clone(),
            validated: passcode,
        })
        .await;
    tracing::info!("res: {:?}", res);
    assert!(res.is_err());

    // passcode failed to decrypt
    let passcode = "123456".to_string();
    let res = client
        .do_auth(DoAuthRequest {
            id: id.clone(),
            device_id: device_id.clone(),
            validated: passcode,
        })
        .await;
    tracing::info!("res: {:?}", res);
    assert!(res.is_err());

    tracing::info!("\t- test {}::test_do_auth finished", module_path!());
    Ok(())
}

pub async fn integration_test() {
    do_with_port(|port| async move {
        let tasks: Vec<_> = test_tasks![port, test_request_auth, test_do_auth,];

        test_service! {
            port,
            AuthServiceServer::new(AuthService)
                .layer_front(DBLayer),
            tasks
        };
    })
    .await
    .await;
}
