#![feature(type_alias_impl_trait)]

use anyhow::Context;
use async_channel::{Receiver, Sender};
use dashmap::DashMap;
use diesel::RunQueryDsl;
use limit_config::GLOBAL_CONFIG;
use limit_db::run_sql;
use limit_db::schema::MESSAGE;
use limit_utils::{batch_execute_background_tasks, BackgroundTask};
use tokio_util::sync::ReusableBoxFuture;
pub use volo_gen::limit::message::*;
use volo_grpc::{BoxStream, Request, Response, Status};

#[derive(Debug, Clone)]
// require db
// require background worker
// TODO: see if there is any message missing
pub struct MessageService {
    users: DashMap<
        // id
        String,
        DashMap<
            // device_id
            String,
            (
                Sender<Result<Message, Status>>,
                Receiver<Result<Message, Status>>,
            ),
        >,
    >,
}

impl MessageService {
    pub fn new() -> Self {
        Self {
            users: DashMap::new(),
        }
    }
}

fn message_to_dbmessage(m: Message) -> limit_db::message::Message {
    limit_db::message::Message {
        message_id: m.message_id,
        timestamp: m.timestamp,
        sender: m.sender,
        receiver_id: m.receiver_id,
        receiver_server: m.receiver_server,
        text: m.text,
        extensions: serde_json::to_value(m.extensions).unwrap().to_string(),
    }
}

#[volo::async_trait]
impl volo_gen::limit::message::MessageService for MessageService {
    async fn receive_messages(
        &self,
        req: Request<ReceiveMessagesRequest>,
    ) -> Result<Response<BoxStream<'static, Result<Message, Status>>>, Status> {
        // check auth is valid
        let auth = req.get_ref().token.clone().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;
        let claim = limit_server_auth::decode_jwt(&auth.jwt)?;
        let ids = claim.sub.split("/").collect::<Vec<_>>();
        let id = ids[1];
        let device_id = ids[0];
        // TODO: create background worker for clean this user
        let entry = self.users.entry(id.to_string()).or_insert_with(|| {
            let map = DashMap::new();
            map.insert(device_id.to_string(), {
                let (sender, receiver) = async_channel::bounded(
                    GLOBAL_CONFIG
                        .get()
                        .unwrap()
                        .per_user_message_on_the_fly_limit,
                );
                (sender, receiver)
            });
            map
        });
        let receiver = entry.get(device_id).unwrap().1.clone();
        Ok(Response::new(Box::pin(receiver)))
    }

    async fn send_message(
        &self,
        req: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        // check auth is valid
        let auth = req.get_ref().token.clone().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;
        let _claim = limit_server_auth::decode_jwt(&auth.jwt)?;
        let message: Message = req.get_ref().message.clone().ok_or_else(|| {
            tracing::error!("message is empty");
            Status::cancelled("message is empty")
        })?;
        let current_server_url = GLOBAL_CONFIG.get().unwrap().url.as_str();
        let message_id = uuid::Uuid::new_v4();
        let mut message = message.clone();
        message.message_id = message_id.to_string();
        let message2 = message.clone();
        if &message.receiver_server == current_server_url {
            // TODO: cluster
            let mut background_tasks = vec![];
            if let Some(devices) = self.users.get(&message.receiver_id) {
                let devices = devices.clone();
                background_tasks.push(BackgroundTask {
                    name: "send_message_to_all_local_online_devices".to_string(),
                    task: ReusableBoxFuture::new(async move {
                        let message = &message;
                        futures::future::join_all(devices.into_iter().map(
                            |(device, (s, _))| async move {
                                tracing::info!("sending message to device {:?}", device);
                                let res = s.clone().send(Ok(message.clone())).await;
                                match res {
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::error!(
                                            "sending message to device {:?} error {:?}",
                                            device,
                                            e
                                        )
                                    }
                                }
                            },
                        ))
                        .await;
                    }),
                })
            }
            // store message
            let pool = req
                .extensions()
                .get::<limit_db::DBPool>()
                .context("no db extended to service")
                .map_err(|e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                })?
                .clone();
            let sql = diesel::insert_into(MESSAGE::table).values(message_to_dbmessage(message2));
            let run_sql = async move {
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
            };
            background_tasks.push(BackgroundTask {
                name: "store_message".to_string(),
                task: ReusableBoxFuture::new(async move {
                    // TODO: save message
                    match run_sql.await {
                        Ok(_) => {
                            tracing::info!("message {:?} saved", message_id)
                        }
                        Err(e) => {
                            tracing::error!("unable to save message {:?} with {:?}", message_id, e)
                        }
                    }
                }),
            });
            batch_execute_background_tasks(background_tasks).await;
            Ok(Response::new(SendMessageResponse {
                message_id: message_id.to_string(),
            }))
        } else {
            todo!("send to other server")
        }
    }
}
