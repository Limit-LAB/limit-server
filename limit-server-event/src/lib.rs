#![feature(type_alias_impl_trait)]

use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use limit_config::GLOBAL_CONFIG;
use limit_db::schema::{EVENT, EVENT_SUBSCRIPTIONS, MESSAGE};
use limit_db::{run_sql, RedisClient};
use limit_utils::{execute_background_task, BackgroundTask};
use tokio_util::sync::ReusableBoxFuture;
use volo_grpc::codegen::StreamExt;
use volo_grpc::{BoxStream, Request, Response, Status};

pub use volo_gen::limit::event::{event::*, synchronize_request::*, types::*, *};

#[derive(Debug, Clone)]
// require db
// require background worker
// TODO: see if there is any message missing
pub struct EventService;

fn message_to_dbmessage(m: Event) -> limit_db::event::SREvent {
    let msg = match m.detail {
        Some(Detail::Message(ref m)) => m,
        _ => panic!(),
    };
    (
        limit_db::event::Event {
            message_id: m.event_id.clone(),
            timestamp: m.timestamp,
            sender: m.sender,
            event_type: "message".to_string(),
        },
        limit_db::event::Message {
            event_id: m.event_id,
            receiver_id: msg.receiver_id.to_owned(),
            receiver_server: msg.receiver_server.to_owned(),
            text: msg.text.to_owned(),
            extensions: serde_json::to_value(msg.extensions.to_owned())
                .unwrap()
                .to_string(),
        },
    )
        .into()
}
fn dbmessage_to_message(m: limit_db::event::SREvent) -> Result<Event, Status> {
    match m.head.event_type.as_str() {
        "message" => {
            if let limit_db::event::SREventBody::Message(body) = m.body {
                Ok(Event {
                    event_id: m.head.message_id,
                    timestamp: m.head.timestamp,
                    sender: m.head.sender,
                    r#type: 1,
                    detail: Some(Detail::Message(Message {
                        receiver_id: body.receiver_id,
                        receiver_server: body.receiver_server,
                        text: body.text,
                        extensions: serde_json::from_str(&body.extensions).unwrap(),
                    })),
                })
            } else {
                Err(Status::internal("event type not match"))
            }
        }
        _ => Err(Status::internal("event type not supported")),
    }
}

#[volo::async_trait]
impl volo_gen::limit::event::EventService for EventService {
    async fn receive_events(
        &self,
        req: Request<ReceiveEventsRequest>,
    ) -> Result<Response<BoxStream<'static, Result<Event, Status>>>, Status> {
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
        let subscriptions: Option<Vec<String>> = redis::cmd("GET")
            .arg(format!("{}:subscribed", id))
            .query(&mut redis_connection)
            .map_err(|e| {
                tracing::error!("{}", e);
                Status::internal(e.to_string())
            })?;
        let subscriptions = if subscriptions.is_none() {
            tracing::info!("🈚 receive message cache miss");
            let sql = EVENT_SUBSCRIPTIONS::table.filter(EVENT_SUBSCRIPTIONS::USER_ID.eq(id));
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
            )?)
        }));
        Ok(Response::new(res))
    }

    async fn send_event(
        &self,
        req: Request<SendEventRequest>,
    ) -> Result<Response<SendEventResponse>, Status> {
        // check auth is valid
        let auth = req.get_ref().token.clone().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;
        let _claim = limit_server_auth::decode_jwt(&auth.jwt)?;
        let event = req.get_ref().event.clone().ok_or_else(|| {
            tracing::error!("message is empty");
            Status::cancelled("message is empty")
        })?;
        let current_server_url = GLOBAL_CONFIG.get().unwrap().url.as_str();
        let mut message = event.clone();
        message.event_id = uuid::Uuid::new_v4().to_string();
        let message2 = message.clone();

        let msg_detail = match event.detail {
            Some(Detail::Message(ref msg)) => msg,
            _ => return Err(Status::internal("no implementation")),
        };

        if &msg_detail.receiver_server == current_server_url {
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

            let message = message_to_dbmessage(message2);
            match message.head.event_type.as_str() {
                "message" => {
                    if let limit_db::event::SREventBody::Message(body) = &message.body {
                        redis::cmd("PUBLISH")
                            .arg(format!("message:{}", body.receiver_id))
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
                        let insert_event_sql =
                            diesel::insert_into(EVENT::table).values(message.head);
                        let insert_message_sql =
                            diesel::insert_into(MESSAGE::table).values(body.clone());
                        let pool2 = pool.clone();
                        let run_sql = async move {
                            run_sql!(
                                pool,
                                |mut conn| {
                                    insert_event_sql.execute(&mut conn).map_err(|e| {
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
                        let event_id = event.event_id.clone();
                        let event_id2 = event.event_id.clone();
                        let event_id3 = event.event_id.clone();
                        execute_background_task(BackgroundTask {
                            name: "store_event".to_string(),
                            task: ReusableBoxFuture::new(async move {
                                match run_sql.await {
                                    Ok(_) => {
                                        tracing::info!("event {:?} saved", event_id);
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "unable to save event {:?} with {:?}",
                                            event_id,
                                            e
                                        )
                                    }
                                }
                            }),
                        })
                        .await;
                        let run_sql = async move {
                            run_sql!(
                                pool2,
                                |mut conn| {
                                    insert_message_sql.execute(&mut conn).map_err(|e| {
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
                                        tracing::info!("message {:?} saved", event_id2);
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "unable to save message {:?} with {:?}",
                                            event_id2,
                                            e
                                        )
                                    }
                                }
                            }),
                        })
                        .await;
                        Ok(Response::new(SendEventResponse {
                            event_id: event_id3,
                        }))
                    } else {
                        Err(Status::internal("message type mismatched"))
                    }
                }
                _ => Err(Status::internal("message type not supported")),
            }
        } else {
            todo!("send to other server")
        }
    }

    async fn synchronize(
        &self,
        req: Request<SynchronizeRequest>,
    ) -> Result<Response<SynchronizeResponse>, Status> {
        let sync_req = req.get_ref();

        // check auth is valid
        let auth = sync_req.token.as_ref().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;

        let claim = limit_server_auth::decode_jwt(&auth.jwt)?;
        let ids = claim.sub.split("/").collect::<Vec<_>>();
        let id = ids[1];
        let starting_point = sync_req
            .starting_point
            .as_ref()
            .unwrap_or(&StartingPoint::Timestamp(chrono::Utc::now().timestamp()));
        let offset = match sync_req.offset {
            0 => 50, // default value of offset
            _ => sync_req.offset,
        };

        let subscription = &sync_req.subscription;
        let filter = sync_req.filter_flags;

        Err(Status::internal("no implementation"))
    }
}
