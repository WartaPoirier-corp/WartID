use chrono::{Duration, NaiveDateTime, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use uuid::Uuid;

use crate::id::Id;
use crate::schema::sessions;

use super::*;

pub type SessionId = Id<Session>;

#[derive(Debug, Queryable)]
pub struct Session {
    pub id: SessionId,
    pub users_id: UserId,
    pub expiration: NaiveDateTime,
}

impl Session {
    pub fn insert(db: crate::DbConnection, new: NewSession) -> WartIDResult<SessionId> {
        use crate::schema::sessions::dsl::*;

        let session: Session = diesel::insert_into(sessions).values(new).get_result(db)?;

        Ok(session.id)
    }

    pub fn find_by_id(db: crate::DbConnection, l_id: Uuid) -> WartIDResult<Option<UserId>> {
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
#[diesel(table_name = sessions)]
pub struct NewSession {
    pub users_id: UserId,
    pub expiration: NaiveDateTime,
}

impl NewSession {
    pub fn new(user_id: UserId) -> Self {
        NewSession {
            users_id: user_id,
            expiration: Utc::now().naive_utc() + Duration::days(14),
        }
    }
}
