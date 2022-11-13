use diesel::RunQueryDsl;
use limit_config::GLOBAL_CONFIG;
use std::{future::Future, net::SocketAddr, pin::Pin};

use futures::StreamExt;
use limit_db::{
    message::MessageSubscriptions,
    run_sql,
    schema::{MESSAGE_SUBSCRIPTIONS, USER, USER_LOGIN_PASSCODE, USER_PRIVACY_SETTINGS},
    DBLayer, DBPool,
};
use limit_server_auth::{AuthService, AuthServiceClientBuilder, AuthServiceServer, DoAuthRequest};
use limit_server_message::MessageService;
use limit_server_message::{
    Message, MessageServiceClientBuilder, MessageServiceServer, ReceiveMessagesRequest,
    SendMessageRequest,
};
use limit_test_utils::{do_with_port, test_service, test_tasks};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub async fn test_send_message(port: u16) -> anyhow::Result<()> {
    tracing::info!("\t- test {}::test_send_message started", module_path!());
    do_with_port(|port2| async move {
        let auth_addr_addr: SocketAddr = format!("127.0.0.1:{}", port2).parse().unwrap();
        let auth_addr = volo::net::Address::from(auth_addr_addr.clone());
        let _auth_server = tokio::spawn(async move {
            let _server = AuthServiceServer::new(AuthService)
                .layer_front(DBLayer)
                .run(auth_addr)
                .await
                .unwrap();
        });
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
                jwt_expiration: limit_db::orm::Duration::from(std::time::Duration::from_secs(
                    114514,
                ))
                .0,
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
                jwt_expiration: limit_db::orm::Duration::from(std::time::Duration::from_secs(
                    114514,
                ))
                .0,
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
                    |mut con| diesel::insert_into(MESSAGE_SUBSCRIPTIONS::table)
                        .values(MessageSubscriptions {
                            user_id: id.clone(),
                            subscribed_to: format!("message:{}", id.clone())
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
        let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        let mut auth_client = AuthServiceClientBuilder::new(format!("{}:auth", module_path!()))
            .address(auth_addr_addr)
            .build();
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
        let mut client1 = MessageServiceClientBuilder::new(format!("{:?}:1", module_path!()))
            .address(addr)
            .build();
        let mut client2 = MessageServiceClientBuilder::new(format!("{:?}:1", module_path!()))
            .address(addr)
            .build();
        let receive = client2
            .receive_messages(ReceiveMessagesRequest {
                token: Some(auth2.get_ref().clone()),
            })
            .await;
        assert!(receive.is_ok());
        tracing::info!("client {:?} online", id2);
        tracing::info!("client {:?} sending message", id1);
        let send_message = client1
            .send_message(SendMessageRequest {
                token: Some(auth1.get_ref().clone()),
                message: Some(Message {
                    message_id: "".to_string(),
                    timestamp: 0,
                    sender: id1.clone(),
                    receiver_id: id2,
                    receiver_server: GLOBAL_CONFIG.get().unwrap().url.clone(),
                    text: "hello".to_string(),
                    extensions: Default::default(),
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
            .text
            .clone();
        assert_eq!(received, "hello");
    })
    .await
    .await;

    tracing::info!("\t- test {}::test_send_message finished", module_path!());
    Ok(())
}

pub async fn integration_test() {
    do_with_port(|port| async move {
        let tasks: Vec<_> = test_tasks![port, test_send_message];

        test_service! {
            port,
            MessageServiceServer::new(MessageService)
                .layer_front(DBLayer),
            tasks
        };
    })
    .await
    .await;
}
