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

#[derive(Debug, thiserror::Error)]
pub enum WartIDError {
    #[error("oauth2 error: {0}")]
    OAuth2Error(&'static str),

    #[error("database connection error")]
    DatabaseConnection,

    #[error("database error: {0}")]
    Database(#[from] Error),

    #[error("invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("invalid form fields: {0}")]
    InvalidForm(String),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + 'static>),

    #[error("TODO")]
    #[deprecated]
    Todo,
}
