use super::*;
use crate::model::User;
use uuid::Uuid;

pub struct Menu {
    pub users: Vec<(Uuid, String)>,
}

impl Menu {
    pub fn build(db: crate::DbConnection) -> WartIDResult<Self> {
        // TODO cache somehow
        Ok(Self {
            users: User::find_all(db, false)?
                .into_iter()
                .map(|u| (u.id, u.username.unwrap_or_else(|| String::from("<?>"))))
                .collect(),
        })
    }
}
