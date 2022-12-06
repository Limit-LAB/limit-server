use anyhow::Context;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use futures::StreamExt;
use limit_config::GLOBAL_CONFIG;
use limit_db::schema::{EVENT, EVENT_SUBSCRIPTIONS, MESSAGE};
use limit_db::{get_db_layer, run_sql, RedisClient};
use limit_deps::diesel::JoinOnDsl;
use limit_deps::*;
use limit_utils::{execute_background_task, BackgroundTask};
use tokio_util::sync::ReusableBoxFuture;
use tonic::{codegen::BoxStream, Request, Response, Status};

pub use tonic_gen::event::{event::*, synchronize_request::*, types::*, *};

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
            id: m.event_id.clone(),
            timestamp: m.ts as i64,
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
            let limit_db::event::SREventBody::Message(body) = m.body;
            Ok(Event {
                event_id: m.head.id,
                ts: m.head.timestamp as u64,
                sender: m.head.sender,
                detail: Some(Detail::Message(Message {
                    receiver_id: body.receiver_id,
                    receiver_server: body.receiver_server,
                    text: body.text,
                    extensions: serde_json::from_str(&body.extensions).unwrap(),
                })),
            })
        }
        _ => Err(Status::internal("event type not supported")),
    }
}

#[tonic::async_trait]
impl tonic_gen::event::event_service_server::EventService for EventService {
    type ReceiveEventsStream = BoxStream<Event>;
    async fn receive_events(
        &self,
        req: Request<ReceiveEventsRequest>,
    ) -> Result<Response<Self::ReceiveEventsStream>, Status> {
        // check auth is valid
        let auth = req.get_ref().token.clone().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;
        let claim = limit_server_auth::decode_jwt(&auth.jwt)?;
        let (_, id) = claim.sub.split_once("/").ok_or_else(|| {
            tracing::error!("invalid uuid");
            Status::unauthenticated("invalid uuid")
        })?;

        let (_, redis, pool) = get_db_layer!(req);
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
                    sql.load::<(String, String, String)>(&mut conn)
                        .map_err(|e| {
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
            .map(|(_, a, c)| format!("{}:{}", c, a))
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
        let res = pubsub.into_on_message().map(|msg| {
            Ok(dbmessage_to_message(
                msg.get_payload()
                    .map(|payload: String| serde_json::from_str(&payload).unwrap())
                    .map_err(|e| {
                        tracing::error!("{}", e);
                        Status::internal(e.to_string())
                    })?,
            )?)
        });
        Ok(Response::new(Box::pin(res)))
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
                    let limit_db::event::SREventBody::Message(body) = &message.body;
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
                        diesel::insert_into(EVENT::table).values(message.head.clone());
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
                    let event_id = message.head.id.clone();
                    let event_id2 = message.head.id.clone();
                    let event_id3 = message.head.id.clone();
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
        let (_, _, db_pool) = get_db_layer!(req);
        // check auth is valid
        let auth = sync_req.token.as_ref().ok_or_else(|| {
            tracing::error!("no auth token");
            Status::unauthenticated("no auth token")
        })?;

        let claim = limit_server_auth::decode_jwt(&auth.jwt)?;
        let id = claim
            .sub
            .split_once("/")
            .map(|(_, id)| id.to_string())
            .ok_or_else(|| {
                tracing::error!("invalid uuid");
                Status::unauthenticated("invalid uuid")
            })?;

        let from = sync_req.from.as_ref().ok_or_else(|| {
            tracing::error!("no from");
            Status::invalid_argument("no from")
        })?;
        let to = sync_req.to.as_ref().ok_or_else(|| {
            tracing::error!("no to");
            Status::invalid_argument("no to")
        })?;
        let sql = EVENT::table
            .left_join(
                MESSAGE::table
                    .inner_join(EVENT_SUBSCRIPTIONS::table.on(EVENT_SUBSCRIPTIONS::USER_ID.eq(id))),
            )
            // filter message
            .filter(MESSAGE::RECEIVER_ID.eq(EVENT_SUBSCRIPTIONS::SUBSCRIBED_TO))
            .filter(EVENT_SUBSCRIPTIONS::CHANNEL_TYPE.eq("message"))
            .order(EVENT::ID.desc());

        let mut sql_id_id = None;
        let mut sql_id_ts = None;
        let mut sql_ts_id = None;
        let mut sql_ts_ts = None;

        let count = match sync_req.count {
            1..=8192 => sync_req.count as i64,
            _ => 50,
        };

        match from {
            From::IdFrom(from_id) => {
                let sql = sql.filter(EVENT::ID.gt(from_id));
                match to {
                    To::IdTo(to_id) => {
                        sql_id_id = Some(sql.filter(EVENT::ID.le(to_id)).limit(count));
                    }
                    To::TsTo(to_ts) => {
                        sql_id_ts = Some(sql.filter(EVENT::TS.le((*to_ts) as i64)).limit(count));
                    }
                }
            }
            From::TsFrom(from_ts) => {
                let sql = sql.filter(EVENT::TS.gt((*from_ts) as i64));
                match to {
                    To::IdTo(to_id) => {
                        sql_ts_id = Some(sql.filter(EVENT::ID.le(to_id)).limit(count));
                    }
                    To::TsTo(to_ts) => {
                        sql_ts_ts = Some(sql.filter(EVENT::TS.le((*to_ts) as i64)).limit(count));
                    }
                }
            }
        };

        macro_rules! send_sync {
            ($e:expr) => {
                if let Some(sql) = $e {
                    let res : Vec<(limit_db::event::Event, Option<(limit_db::event::Message, limit_db::event::EventSubscriptions)>)> = run_sql!(
                        db_pool,
                        |mut conn| {
                            // tracing::info!("sql: {:?}", diesel::debug_query::<diesel::sqlite::Sqlite, _>(&sql));
                            sql.load::<(limit_db::event::Event, Option<(limit_db::event::Message, limit_db::event::EventSubscriptions)>)>(&mut conn).map_err(|e| {
                                tracing::error!("{}", e);
                                Status::internal(e.to_string())
                            })
                        },
                        |e| {
                            tracing::error!("{}", e);
                            Status::internal(e.to_string())
                        }
                    )?;
                    let mut ret = Vec::with_capacity(count as usize);
                    res.into_iter().map(|(event, message)| {
                        match (message,) {
                            (Some((body, _)),) => {
                                Event {
                                    event_id : event.id,
                                    ts : event.timestamp as u64,
                                    sender : event.sender,
                                    detail : Some(Detail::Message(Message {
                                        receiver_id: body.receiver_id,
                                        receiver_server: body.receiver_server,
                                        text: body.text,
                                        extensions: serde_json::from_str(&body.extensions).unwrap(),
                                    })),
                                }
                            }
                            _ => todo!()
                        }
                    })
                    .collect_into(&mut ret);
                    return Ok(Response::new(SynchronizeResponse {
                        events: ret,
                    }));
                };
            };
        }

        send_sync!(sql_id_id);
        send_sync!(sql_id_ts);
        send_sync!(sql_ts_id);
        send_sync!(sql_ts_ts);

        Err(Status::internal("no implementation"))
    }
}
