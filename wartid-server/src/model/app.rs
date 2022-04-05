use diesel::expression::exists::exists;
use diesel::{
    BoolExpressionMethods, Connection, ExpressionMethods, QueryDsl, Queryable, RunQueryDsl,
};

use crate::id::Id;
use crate::schema::{user_apps, user_apps_managers};

use super::*;

pub type UserAppId = Id<UserApp>;

type OAuthSecret = String;

#[derive(Queryable)]
pub struct UserApp {
    pub id: UserAppId,
    pub name: String,
    oauth_secret: Option<OAuthSecret>,
    pub oauth_redirect: String,
    pub description: Option<String>,
    pub hidden: bool,
}

impl UserApp {
    pub fn oauth2(&self) -> Option<(&str, &str)> {
        self.oauth_secret
            .as_ref()
            .map(|secret| (secret.as_ref(), self.oauth_redirect.as_str()))
    }

    pub fn is_oauth2_redirect_allowed(&self, uri: &str) -> bool {
        (self.oauth_redirect.len() > "https://a.bc".len())
            && (uri.starts_with(&self.oauth_redirect))
    }
}

impl UserApp {
    pub fn insert(
        db: crate::DbConnection,
        l_name: String,
        l_hidden: bool,
        creator: UserId,
    ) -> WartIDResult<UserAppId> {
        use crate::schema::user_apps::dsl::*;
        use crate::schema::user_apps_managers::dsl::*;

        // Insertions are done in a transaction so if the second one fails, the first one should in
        // theory be rolled back. Else, we would end up with an orphan app.
        db.transaction::<UserAppId, WartIDError, _>(|| {
            let app_id = diesel::insert_into(user_apps)
                .values(NewUserApp {
                    name: l_name,
                    hidden: l_hidden,
                })
                .get_result::<UserApp>(db)?
                .id;

            diesel::insert_into(user_apps_managers)
                .values(NewUserAppManager {
                    user_apps_id: app_id,
                    users_id: creator,
                })
                .execute(db)?;

            Ok(app_id)
        })
    }

    pub fn find_all(db: crate::DbConnection, view_as: UserId) -> WartIDResult<Vec<Self>> {
        use crate::schema::user_apps::dsl::*;
        use crate::schema::user_apps_managers::dsl::*;

        user_apps
            .filter(hidden.eq(false).or(exists(
                user_apps_managers.filter(user_apps_id.eq(id).and(users_id.eq(view_as))),
            )))
            .load::<Self>(db)
            .map_err(Into::into)
    }

    pub fn find_by_id(db: crate::DbConnection, l_app_id: UserAppId) -> WartIDResult<Option<Self>> {
        use crate::schema::user_apps::dsl::*;

        // TODO honor "hidden"

        match user_apps.filter(id.eq(l_app_id)).first(db) {
            Ok(app) => Ok(Some(app)),
            Err(diesel::NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn set_oauth(db: crate::DbConnection, app: UserAppId, enable: bool) -> WartIDResult<Self> {
        use crate::schema::user_apps::dsl::*;

        diesel::update(user_apps)
            .filter(id.eq(app))
            .set(oauth_secret.eq(if enable {
                Some(crate::utils::gen_alphanumeric(64))
            } else {
                None
            }))
            .get_result(db)
            .map_err(Into::into)
    }

    pub fn set_oauth_redirect_uri(
        db: crate::DbConnection,
        app: UserAppId,
        uri: String,
    ) -> WartIDResult<Self> {
        use crate::schema::user_apps::dsl::*;

        diesel::update(user_apps)
            .filter(id.eq(app))
            .set(oauth_redirect.eq(uri))
            .get_result(db)
            .map_err(Into::into)
    }

    pub fn set_name_description(
        db: crate::DbConnection,
        app: UserAppId,
        l_name: &str,
        l_description: &str,
    ) -> WartIDResult<Self> {
        use crate::schema::user_apps::dsl::*;

        diesel::update(user_apps)
            .filter(id.eq(app))
            .set((name.eq(l_name), description.eq(l_description)))
            .get_result(db)
            .map_err(Into::into)
    }
}

#[derive(Insertable)]
#[table_name = "user_apps"]
struct NewUserApp {
    name: String,
    hidden: bool,
}

#[derive(Insertable)]
#[table_name = "user_apps_managers"]
struct NewUserAppManager {
    user_apps_id: UserAppId,
    users_id: UserId,
}
