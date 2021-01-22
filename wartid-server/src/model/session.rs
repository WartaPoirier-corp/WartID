use chrono::{Duration, NaiveDateTime, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use uuid::Uuid;

use crate::schema::sessions;

use super::WartIDResult;

#[derive(Debug, Queryable)]
pub struct Session {
    pub id: Uuid,
    pub users_id: Uuid,
    pub expiration: NaiveDateTime,
}

impl Session {
    pub fn insert(db: crate::DbConnection, new: NewSession) -> WartIDResult<Uuid> {
        use crate::schema::sessions::dsl::*;

        let session: Session = diesel::insert_into(sessions).values(new).get_result(db)?;

        Ok(session.id)
    }

    /// Returns the UUID of the user corresponding to this session ID
    pub fn find_by_id(db: crate::DbConnection, l_id: Uuid) -> WartIDResult<Option<Uuid>> {
        use crate::schema::sessions::dsl::*;

        Ok(sessions
            .filter(id.eq(&l_id).and(expiration.ge(Utc::now().naive_utc())))
            .limit(1)
            .load::<Self>(db)?
            .into_iter()
            .next()
            .map(|session| session.users_id))
    }
}

#[derive(Insertable)]
#[table_name = "sessions"]
pub struct NewSession {
    pub users_id: Uuid,
    pub expiration: NaiveDateTime,
}

impl NewSession {
    pub fn new(account: Uuid) -> Self {
        NewSession {
            users_id: account,
            expiration: Utc::now().naive_utc() + Duration::days(14),
        }
    }
}
