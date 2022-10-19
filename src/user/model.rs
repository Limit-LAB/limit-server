use crate::schema::*;

use diesel::prelude::*;
use std::{str::FromStr, time::Duration};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A user
#[derive(Serialize, Deserialize, ToSchema, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = USER)]
struct User {
    /// should be unique
    #[diesel(column_name = "ID")]
    #[diesel(serialize_as = crate::orm::Uuid)]
    id: String,
    // TODO: web3 approach
    /// the RSA public key of the user
    #[diesel(column_name = "PUBKEY")]
    pubkey: String,
}

/// A user's profile
#[derive(Serialize, Deserialize, ToSchema, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = USER_PROFILE)]
struct Profile {
    /// forgien key to [`User`]
    #[diesel(column_name = "ID")]
    #[diesel(serialize_as = crate::orm::Uuid)]
    id: String,
    /// the user's name
    #[diesel(column_name = "NAME")]
    name: String,
    /// the user name on the server for @ and login
    #[diesel(column_name = "USER_NAME")]
    username: String,
    /// the user's bio
    #[diesel(column_name = "BIO")]
    bio: Option<String>,
    // TODO: url or base64?
    /// the user's avatar
    /// if the avatar is not set, the client will use the None
    /// when query without permission, the client will return the None
    #[diesel(column_name = "AVATAR")]
    avatar: Option<String>,
    /// last login time
    /// if the user never login, the server will return the register time
    /// when query without permission, the server will return the None
    #[diesel(column_name = "LAST_SEEN")]
    #[diesel(serialize_as = crate::orm::Duration)]
    last_seen: Option<String>,
    /// the last time the user update the profile
    /// client should use this to check whether the profile is updated
    #[diesel(column_name = "LAST_MODIFIED")]
    #[diesel(serialize_as = crate::orm::DateTime)]
    last_modified: Option<String>,
}

/// A user's private settings
#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct PrivacySettings {
    /// forgien key to [`User`]
    id: uuid::Uuid,
    /// check profile
    avatar: Visibility,
    /// last time online
    last_seen: Visibility,
    /// group invites
    groups: Visibility,
    /// could foward messages to other users
    forwards: Visibility,
    /// minimum 24 hours, maximum 1 week
    jwt_expiration: Duration,
}

/// The visibility of a field
#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub enum Visibility {
    Public,
    FrendsOnly,
    Private,
}

impl FromStr for Visibility {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(Self::Public),
            "friends_only" => Ok(Self::FrendsOnly),
            "private" => Ok(Self::Private),
            _ => Err(()),
        }
    }
}

impl ToString for Visibility {
    fn to_string(&self) -> String {
        match self {
            Self::Public => "public".to_string(),
            Self::FrendsOnly => "friends_only".to_string(),
            Self::Private => "private".to_string(),
        }
    }
}

#[test]
fn test_user_model() {
    use chrono::Utc;

    let id = crate::orm::Uuid::from(uuid::Uuid::new_v4()).0;
    let dummy_user = User {
        id: id.clone(),
        pubkey: "xdddd".to_string(),
    };

    let dummy_user_profile = Profile {
        id: id.clone(),
        name: "xdddd".to_string(),
        username: "xdddd".to_string(),
        bio: Some("xdddd".to_string()),
        avatar: Some("xdddd".to_string()),
        last_seen: Some(crate::orm::Duration::from(Duration::from_secs(100)).0),
        last_modified: Some(crate::orm::DateTime::from(Utc::now()).0),
    };

    let mut con = diesel::sqlite::SqliteConnection::establish("test.sqlite").unwrap();
    let rows_inserted = diesel::insert_into(USER::table)
        .values(dummy_user)
        .execute(&mut con)
        .unwrap();
    assert_eq!(rows_inserted, 1);

    let rows_inserted = diesel::insert_into(USER_PROFILE::table)
        .values(dummy_user_profile)
        .execute(&mut con)
        .unwrap();
    assert_eq!(rows_inserted, 1);

    let users = USER::table.load::<User>(&mut con).unwrap();
    assert!(users.len() >= 1);
    let profile_of_username = USER_PROFILE::table
        .inner_join(USER::table)
        .filter(USER::PUBKEY.eq("xdddd"))
        .filter(USER_PROFILE::ID.eq(id))
        .select(USER_PROFILE::all_columns)
        .load::<Profile>(&mut con)
        .unwrap();
    assert!(profile_of_username.len() >= 1);
}
