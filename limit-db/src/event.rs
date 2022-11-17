use crate::schema::*;
use diesel::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

/// A event for sending and receiving
#[derive(Serialize, Deserialize)]
pub struct SREvent {
    pub head: Event,
    pub body: SREventBody,
}

#[derive(Serialize, Deserialize)]
pub enum SREventBody {
    Message(Message),
}

impl From<(Event, Message)> for SREvent {
    fn from(value: (Event, Message)) -> Self {
        Self {
            head: value.0,
            body: SREventBody::Message(value.1),
        }
    }
}

/// A event
#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = EVENT)]
pub struct Event {
    /// should be unique
    #[diesel(column_name = "ID")]
    #[diesel(serialize_as = crate::orm::Uuid)]
    pub message_id: String,
    /// the timestamp UTC of the message
    #[diesel(column_name = "TS")]
    pub timestamp: i64,
    /// the sender uuid
    #[diesel(column_name = "SENDER")]
    pub sender: String,
    /// the event type
    #[diesel(column_name = "EVENT_TYPE")]
    pub event_type: String,
}

/// A message
#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = MESSAGE)]
pub struct Message {
    /// should be unique
    #[diesel(column_name = "EVENT_ID")]
    #[diesel(serialize_as = crate::orm::Uuid)]
    pub event_id: String,
    #[diesel(column_name = "RECEIVER_ID")]
    pub receiver_id: String,
    /// the receiver server
    #[diesel(column_name = "RECEIVER_SERVER")]
    pub receiver_server: String,
    /// text message
    #[diesel(column_name = "TEXT")]
    pub text: String, // TODO: encrypted text message
    /// extensions in string json
    #[diesel(column_name = "EXTENSIONS")]
    pub extensions: String,
}

/// user subscribe to message queue
#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = EVENT_SUBSCRIPTIONS)]
pub struct EventSubscriptions {
    #[diesel(column_name = "USER_ID")]
    #[diesel(serialize_as = crate::orm::Uuid)]
    pub user_id: String,
    #[diesel(column_name = "SUBSCRIBED_TO")]
    pub sub_to: String,
}
