use crate::schema::*;
use diesel::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

/// A message
#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = MESSAGE)]
pub struct Message {
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
    /// the receiver uuid
    #[diesel(column_name = "RECEIVER_ID")]
    pub receiver_id: String,
    /// the receiver server
    #[diesel(column_name = "RECEIVER_SERVER")]
    pub receiver_server: String,
    /// text message
    #[diesel(column_name = "TEXT")]
    pub text: String,
    /// extensions in string json
    #[diesel(column_name = "EXTENSIONS")]
    pub extensions: String,
}

/// user subscribe to message queue
#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = MESSAGE_SUBSCRIPTIONS)]
pub struct MessageSubscriptions {
    #[diesel(column_name = "USER_ID")]
    #[diesel(serialize_as = crate::orm::Uuid)]
    pub user_id: String,
    #[diesel(column_name = "SUBSCRIBED_TO")]
    pub subscribed_to: String,
}
