use std::{future::Future, pin::Pin};

use diesel::RunQueryDsl;
use futures::StreamExt;
use limit_config::GLOBAL_CONFIG;
use limit_db::{
    event::EventSubscriptions,
    run_sql,
    schema::{EVENT_SUBSCRIPTIONS, USER, USER_LOGIN_PASSCODE, USER_PRIVACY_SETTINGS},
    DBLayer, DBPool,
};
use limit_deps::{tonic::transport::Server, *};
use limit_server_auth::{
    auth_service_client::AuthServiceClient, auth_service_server::AuthServiceServer, AuthService,
    DoAuthRequest,
};
use limit_server_event::{
    event_service_client::EventServiceClient, event_service_server::EventServiceServer, Detail,
    Event, EventService, From, Message, ReceiveEventsRequest, SendEventRequest, SynchronizeRequest,
    To,
};
use limit_test_utils::{do_with_port, test_service, test_tasks};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub async fn test_send_message(port: u16) -> anyhow::Result<()> {
    tracing::info!("\t- test {}::test_send_message started", module_path!());
    let _device_id = uuid::Uuid::new_v4().to_string();
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
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
    let device_id = uuid::Uuid::new_v4().to_string();
    // set up user1
    let id = uuid::Uuid::new_v4().to_string();
    let id1 = id.clone();
    {
        let user = limit_db::user::User {
            id: id.clone(),
            pubkey: user_pubkey.clone(),
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
    }
    // set up user2
    let id = uuid::Uuid::new_v4().to_string();
    let id2 = id.clone();
    {
        let user = limit_db::user::User {
            id: id.clone(),
            pubkey: user_pubkey.clone(),
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
            run_sql!(
                pool,
                |mut con| diesel::insert_into(EVENT_SUBSCRIPTIONS::table)
                    .values(EventSubscriptions {
                        user_id: id.clone(),
                        sub_to: id.clone(),
                        channel_type: "message".to_string(),
                    })
                    .execute(&mut con)
                    .unwrap(),
                |e| {
                    tracing::error!("Error: {}", e);
                }
            );
            Ok::<(), ()>(())
        };
        config().unwrap();
    }
    let addr = format!("http://127.0.0.1:{port}");
    let mut auth_client = AuthServiceClient::connect(addr.clone()).await?;
    let passcode = limit_am::aes256_encrypt_string(&shared_key, "123456").unwrap();
    let res = auth_client
        .do_auth(DoAuthRequest {
            id: id1.clone(),
            device_id: device_id.clone(),
            validated: passcode.clone(),
        })
        .await;
    let auth1 = res.unwrap();
    let res = auth_client
        .do_auth(DoAuthRequest {
            id: id2.clone(),
            device_id: device_id.clone(),
            validated: passcode,
        })
        .await;
    let auth2 = res.unwrap();
    let mut client1 = EventServiceClient::connect(addr.clone()).await?;
    let mut client2 = EventServiceClient::connect(addr.clone()).await?;
    let receive = client2
        .receive_events(ReceiveEventsRequest {
            token: Some(auth2.get_ref().clone()),
        })
        .await;
    assert!(receive.is_ok());
    tracing::info!("client {:?} online", id2);
    tracing::info!("client {:?} sending message", id1);
    let send_message = client1
        .send_event(SendEventRequest {
            token: Some(auth1.get_ref().clone()),
            event: Some(Event {
                event_id: "".to_string(),
                ts: chrono::Utc::now().timestamp_millis() as u64,
                sender: id1.clone(),
                detail: Some(Detail::Message(Message {
                    receiver_id: id2,
                    receiver_server: GLOBAL_CONFIG.get().unwrap().url.clone(),
                    text: "hello".to_string(),
                    extensions: Default::default(),
                })),
            }),
        })
        .await;
    assert!(send_message.is_ok());
    tracing::info!("client {:?} message sent", id1);

    let received = receive
        .unwrap()
        .get_mut()
        .next()
        .await
        .unwrap()
        .unwrap()
        .detail
        .unwrap();

    let received = match received {
        Detail::Message(ref m) => &m.text,
    };

    assert_eq!(received, "hello");
    tracing::info!("\t- test {}::test_send_message finished", module_path!());
    Ok(())
}

pub async fn test_sync_message(port: u16) -> anyhow::Result<()> {
    tracing::info!("\t- test {}::test_sync_message started", module_path!());
    let send_ts = chrono::Utc::now().timestamp_millis();
    let _device_id = uuid::Uuid::new_v4().to_string();
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
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
    let device_id = uuid::Uuid::new_v4().to_string();
    // set up user1
    let id = uuid::Uuid::new_v4().to_string();
    let id1 = id.clone();
    {
        let user = limit_db::user::User {
            id: id.clone(),
            pubkey: user_pubkey.clone(),
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
    }
    // set up user2
    let id = uuid::Uuid::new_v4().to_string();
    let id2 = id.clone();
    {
        let user = limit_db::user::User {
            id: id.clone(),
            pubkey: user_pubkey.clone(),
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
            run_sql!(
                pool,
                |mut con| diesel::insert_into(EVENT_SUBSCRIPTIONS::table)
                    .values(EventSubscriptions {
                        user_id: id.clone(),
                        sub_to: id.clone(),
                        channel_type: "message".to_string(),
                    })
                    .execute(&mut con)
                    .unwrap(),
                |e| {
                    tracing::error!("Error: {}", e);
                }
            );
            Ok::<(), ()>(())
        };
        config().unwrap();
    }
    let addr = format!("http://127.0.0.1:{port}");
    let mut auth_client = AuthServiceClient::connect(addr.clone()).await?;
    let passcode = limit_am::aes256_encrypt_string(&shared_key, "123456").unwrap();
    let res = auth_client
        .do_auth(DoAuthRequest {
            id: id1.clone(),
            device_id: device_id.clone(),
            validated: passcode.clone(),
        })
        .await;
    let auth1 = res.unwrap();
    let res = auth_client
        .do_auth(DoAuthRequest {
            id: id2.clone(),
            device_id: device_id.clone(),
            validated: passcode,
        })
        .await;
    let auth2 = res.unwrap();
    let mut client1 = EventServiceClient::connect(addr.clone()).await?;
    let mut client2 = EventServiceClient::connect(addr).await?;

    tracing::info!("client {:?} online", id2);
    tracing::info!("client {:?} sending message", id1);
    let send_message = client1
        .send_event(SendEventRequest {
            token: Some(auth1.get_ref().clone()),
            event: Some(Event {
                event_id: "".to_string(),
                ts: chrono::Utc::now().timestamp_millis() as u64,
                sender: id1.clone(),
                detail: Some(Detail::Message(Message {
                    receiver_id: id2.clone(),
                    receiver_server: GLOBAL_CONFIG.get().unwrap().url.clone(),
                    text: "1".to_string(),
                    extensions: Default::default(),
                })),
            }),
        })
        .await;
    assert!(send_message.is_ok());
    tracing::info!("client {:?} message sent", id1);
    let send_message = client1
        .send_event(SendEventRequest {
            token: Some(auth1.get_ref().clone()),
            event: Some(Event {
                event_id: "".to_string(),
                ts: chrono::Utc::now().timestamp_millis() as u64,
                sender: id1.clone(),
                detail: Some(Detail::Message(Message {
                    receiver_id: id2.clone(),
                    receiver_server: GLOBAL_CONFIG.get().unwrap().url.clone(),
                    text: "2".to_string(),
                    extensions: Default::default(),
                })),
            }),
        })
        .await;
    assert!(send_message.is_ok());
    tracing::info!("client {:?} message sent", id1);
    let send_message = client1
        .send_event(SendEventRequest {
            token: Some(auth1.get_ref().clone()),
            event: Some(Event {
                event_id: "".to_string(),
                ts: chrono::Utc::now().timestamp_millis() as u64,
                sender: id1.clone(),
                detail: Some(Detail::Message(Message {
                    receiver_id: id2.clone(),
                    receiver_server: GLOBAL_CONFIG.get().unwrap().url.clone(),
                    text: "3".to_string(),
                    extensions: Default::default(),
                })),
            }),
        })
        .await;
    assert!(send_message.is_ok());
    tracing::info!("client {:?} message sent", id1);
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let sync = client2
        .synchronize(SynchronizeRequest {
            token: Some(auth2.get_ref().clone()),
            count: 50,
            from: Some(From::TsFrom(send_ts as u64)),
            to: Some(To::TsTo(chrono::Utc::now().timestamp_millis() as u64)),
        })
        .await;
    assert!(sync.is_ok());
    let sync = sync.unwrap();
    assert!(sync.get_ref().events.len() >= 3);
    tracing::info!("sync messages: {:#?}", sync.get_ref().events);

    tracing::info!("\t- test {}::test_sync_message finished", module_path!());
    Ok(())
}

pub async fn integration_test() {
    do_with_port(|port| async move {
        let tasks: Vec<_> = test_tasks![port, test_send_message, test_sync_message];

        test_service! {
            port,
            Server::builder()
                .layer(DBLayer)
                .add_service(AuthServiceServer::new(AuthService))
                .add_service(EventServiceServer::new(EventService)),
            tasks
        };
    })
    .await
    .await;
}
