#![feature(type_alias_impl_trait)]

use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use limit_config::GLOBAL_CONFIG;
use limit_db::schema::{MESSAGE, MESSAGE_SUBSCRIPTIONS};
use limit_db::{run_sql, RedisClient};
use limit_utils::{execute_background_task, BackgroundTask};
use tokio_util::sync::ReusableBoxFuture;
pub use volo_gen::limit::message::*;
use volo_grpc::codegen::StreamExt;
use volo_grpc::{BoxStream, Request, Response, Status};

#[derive(Debug, Clone)]
// require db
// require background worker
// TODO: see if there is any message missing
pub struct MessageService;

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
fn dbmessage_to_message(m: limit_db::message::Message) -> Message {
    Message {
        message_id: m.message_id,
        timestamp: m.timestamp,
        sender: m.sender,
        receiver_id: m.receiver_id,
        receiver_server: m.receiver_server,
        text: m.text,
        extensions: serde_json::from_str(&m.extensions).unwrap(),
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
        let pool = req
            .extensions()
            .get::<limit_db::DBPool>()
            .context("no db extended to service")
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?
            .clone();
        let redis = req
            .extensions()
            .get::<RedisClient>()
            .context("no redis extended to service")
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?
            .clone();
        let mut redis_connection = redis.get_connection().map_err(|e| {
            tracing::error!("{}", e);
            Status::internal(e.to_string())
        })?;
        let redis_async_connection = redis.get_async_connection().await.map_err(|e| {
            tracing::error!("{}", e);
            Status::internal(e.to_string())
        })?;
        // TODO: specify subscription channel
        let subscriptions: Option<Vec<String>> = redis::cmd("GET")
            .arg(format!("{}:subscribed", id))
            .query(&mut redis_connection)
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?;
        let subscriptions = if subscriptions.is_none() {
            tracing::info!("🈚 receive message cache miss");
            let sql = MESSAGE_SUBSCRIPTIONS::table.filter(MESSAGE_SUBSCRIPTIONS::USER_ID.eq(id));
            let subs = run_sql!(
                pool,
                |mut conn| {
                    sql.load::<(String, String)>(&mut conn).map_err(|e| {
                        tracing::error!("{}", e);
                        Status::internal(e.to_string())
                    })
                },
                |e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                }
            )?
            .into_iter()
            .map(|(_, a)| a)
            .collect::<Vec<_>>();
            subs
        } else {
            tracing::info!("🈶 receive message cache hit");
            subscriptions.unwrap()
        };
        let mut pubsub = redis_async_connection.into_pubsub();
        for sub in subscriptions {
            pubsub.subscribe(sub).await.map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?;
        }
        let res = Box::pin(pubsub.into_on_message().map(|msg| {
            Ok(dbmessage_to_message(
                msg.get_payload()
                    .map(|payload: String| serde_json::from_str(&payload).unwrap())
                    .map_err(|e| {
                        tracing::error!("{}", e);
                        Status::internal(e.to_string())
                    })?,
            ))
        }));
        Ok(Response::new(res))
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
        // encrypted message
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
            let mut redis = req
                .extensions()
                .get::<RedisClient>()
                .context("no redis extended to service")
                .map_err(|e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                })?
                .clone()
                .get_connection()
                .map_err(|e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                })?;

            // TODO: specify subscription channel
            let message = message_to_dbmessage(message2);
            redis::cmd("PUBLISH")
                .arg(format!("message:{}", message.receiver_id))
                .arg(serde_json::to_string(&message).unwrap())
                .execute(&mut redis);

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
            let sql = diesel::insert_into(MESSAGE::table).values(message);
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
            execute_background_task(BackgroundTask {
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
            })
            .await;
            Ok(Response::new(SendMessageResponse {
                message_id: message_id.to_string(),
            }))
        } else {
            todo!("send to other server")
        }
    }
}
