use chrono::{Duration, NaiveDateTime, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};

use crate::model::OAuth2Scopes;
use crate::schema::sessions_oauth2;

use super::WartIDResult;
use super::*;

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sessions_oauth2)]
pub struct OAuth2Session {
    pub token: String,
    pub users_id: UserId,
    pub user_apps_id: UserAppId,
    pub initial_scopes: String,
    pub expiration: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = sessions_oauth2)]
pub struct NewOAuth2Session<'a> {
    pub token: &'a str,
    pub users_id: UserId,
    pub user_apps_id: UserAppId,
    pub initial_scopes: &'a str,
    pub expiration: NaiveDateTime,
}

impl OAuth2Session {
    pub fn insert_or_refresh(
        db: crate::DbConnection,
        user: UserId,
        app: UserAppId,
        scopes: &OAuth2Scopes,
    ) -> WartIDResult<String> {
        use crate::schema::sessions_oauth2::dsl::*;

        let l_token = crate::utils::gen_alphanumeric(32);

        let new = OAuth2Session {
            token: l_token,
            users_id: user,
            user_apps_id: app,
            initial_scopes: format!("{scopes}"),
            expiration: Utc::now().naive_utc() + Duration::days(6 * 30),
        };

        let session: OAuth2Session = diesel::insert_into(sessions_oauth2)
            .values(&new)
            .on_conflict((users_id, user_apps_id))
            .do_update()
            .set(&new)
            .get_result(db)?;

        Ok(session.token)
    }

    pub fn find_by_token(db: crate::DbConnection, l_token: &str) -> WartIDResult<Option<Self>> {
        use crate::schema::sessions_oauth2::dsl::*;

        sessions_oauth2
            .filter(token.eq(l_token).and(expiration.ge(Utc::now().naive_utc())))
            .first::<Self>(db)
            .optional()
            .map_err(Into::into)
    }
}
