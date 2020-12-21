use super::*;
use crate::model::User;
use uuid::Uuid;

pub struct Menu {
    pub users: Vec<(Uuid, String)>,
    pub apps: Vec<(Uuid, String)>,
}

impl Menu {
    pub fn build(db: crate::DbConnection, user_id: Uuid) -> WartIDResult<Self> {
        // TODO cache somehow. ok found out: rocket's local cache
        // TODO implement some DB queries as `FromRequest` structs
        Ok(Self {
            users: User::find_all(db, false)?
                .into_iter()
                .map(|u| (u.id, u.username))
                .collect(),
            apps: UserApp::find_all(db, user_id)?
                .into_iter()
                .map(|app| (app.id, app.name))
                .collect(),
        })
    }
}
