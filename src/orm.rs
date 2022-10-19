use std::str::FromStr;

use diesel::{
    backend::Backend,
    serialize::{self, Output, ToSql},
    sql_types, AsExpression, FromSqlRow,
};

#[derive(Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct Uuid(pub String);

impl From<uuid::Uuid> for Uuid {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid.to_string())
    }
}

impl Into<Uuid> for String {
    fn into(self) -> Uuid {
        Uuid(uuid::Uuid::parse_str(self.as_str()).unwrap().to_string())
    }
}

impl<DB> ToSql<sql_types::Text, DB> for Uuid
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

#[derive(Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct Duration(pub String);

impl Into<Duration> for String {
    fn into(self) -> Duration {
        Duration(
            std::time::Duration::from_secs(self.parse().unwrap())
                .as_secs()
                .to_string(),
        )
    }
}

impl From<std::time::Duration> for Duration {
    fn from(duration: std::time::Duration) -> Self {
        Self(duration.as_secs().to_string())
    }
}

impl Into<Duration> for Option<String> {
    fn into(self) -> Duration {
        if let Some(s) = self {
            Duration(
                std::time::Duration::from_secs(s.parse().unwrap())
                    .as_secs()
                    .to_string(),
            )
        } else {
            Duration("0".into())
        }
    }
}

impl<DB> ToSql<sql_types::Text, DB> for Duration
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

#[derive(Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct DateTime(pub String);

impl Into<DateTime> for String {
    fn into(self) -> DateTime {
        DateTime(
            chrono::DateTime::parse_from_rfc3339(self.as_str())
                .unwrap()
                .to_string(),
        )
    }
}

impl From<Option<String>> for DateTime {
    fn from(s: Option<String>) -> Self {
        if let Some(s) = s {
            Self(
                chrono::DateTime::parse_from_rfc3339(s.as_str())
                    .unwrap()
                    .to_string(),
            )
        } else {
            Self(
                chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z")
                    .unwrap()
                    .to_string(),
            )
        }
    }
}

impl From<chrono::DateTime<chrono::Utc>> for DateTime {
    fn from(datetime: chrono::DateTime<chrono::Utc>) -> Self {
        Self(datetime.to_rfc3339())
    }
}

impl<DB> ToSql<sql_types::Text, DB> for DateTime
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

#[derive(Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct Visibility(pub String);

impl Into<Visibility> for String {
    fn into(self) -> Visibility {
        Visibility(
            crate::user::model::Visibility::from_str(self.as_str())
                .unwrap()
                .to_string(),
        )
    }
}

impl<DB> ToSql<sql_types::Text, DB> for Visibility
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}
