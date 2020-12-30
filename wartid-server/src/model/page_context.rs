use super::*;
use std::borrow::Cow;
use uuid::Uuid;

pub enum FlashMessage {}

pub struct PageContext {
    pub users: Vec<(Uuid, String)>,
    pub apps: Vec<(Uuid, String)>,

    pub flash_bad_request: bool,
    pub flash_messages: Vec<(Cow<'static, str>, bool)>,
}

impl PageContext {
    pub fn new(db: crate::DbConnection, user_id: Uuid) -> WartIDResult<Self> {
        Ok(Self {
            users: User::find_all(db, false)?
                .into_iter()
                .map(|u| (u.id, u.username))
                .collect(),
            apps: UserApp::find_all(db, user_id)?
                .into_iter()
                .map(|app| (app.id, app.name))
                .collect(),

            flash_bad_request: false,
            flash_messages: Vec::new(),
        })
    }

    pub fn add_flash_message(&mut self, message: Cow<'static, str>, is_error: bool) {
        self.flash_messages.push((message, is_error));
        self.flash_bad_request = self.flash_bad_request || is_error;
    }
}
