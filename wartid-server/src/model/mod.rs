use std::fmt::Debug;

use diesel::result::Error;

pub use app::*;
pub use menu::*;
pub use session::*;
pub use user::*;

mod app;
mod menu;
mod session;
mod user;

pub type WartIDResult<T> = Result<T, WartIDError>;

#[derive(Debug)]
pub enum WartIDError {
    DatabaseConnection,

    Database(diesel::result::Error),

    InvalidCredentials(String),

    #[deprecated]
    Todo,
}

impl From<diesel::result::Error> for WartIDError {
    fn from(e: Error) -> Self {
        WartIDError::Database(e)
    }
}
