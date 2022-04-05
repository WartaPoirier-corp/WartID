use std::fmt::Debug;

use diesel::result::Error;

pub use app::*;
pub use oauth2session::*;
pub use page_context::*;
pub use scopes::*;
pub use session::*;
pub use user::*;

pub use crate::db_await;

mod app;
mod oauth2session;
mod page_context;
mod scopes;
mod session;
mod user;

pub type WartIDResult<T> = Result<T, WartIDError>;

#[derive(Debug)]
pub enum WartIDError {
    OAuth2Error(&'static str),

    DatabaseConnection,

    Database(diesel::result::Error),

    InvalidCredentials(String),

    InvalidForm(String),

    Any(Box<dyn std::error::Error + Send + 'static>),

    #[deprecated]
    Todo,
}

impl From<diesel::result::Error> for WartIDError {
    fn from(e: Error) -> Self {
        WartIDError::Database(e)
    }
}
