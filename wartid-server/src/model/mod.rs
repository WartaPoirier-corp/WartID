mod app;
mod menu;
mod oauth2session;
mod scopes;
mod session;
mod user;

pub use app::*;
pub use menu::*;
pub use oauth2session::*;
pub use scopes::*;
pub use session::*;
pub use user::*;

use diesel::result::Error;
use std::fmt::Debug;

pub type WartIDResult<T> = Result<T, WartIDError>;

#[derive(Debug)]
pub enum WartIDError {
    OAuth2Error(&'static str),

    DatabaseConnection,

    Database(diesel::result::Error),

    InvalidCredentials(String),

    Any(Box<dyn std::error::Error>),

    #[deprecated]
    Todo,
}

impl From<diesel::result::Error> for WartIDError {
    fn from(e: Error) -> Self {
        WartIDError::Database(e)
    }
}

macro_rules! ext_impl {
    ($(for <$($ctx:tt),*>)? fn <$base:ty>.$name:ident($($params:tt)*) $(-> $ret:ty)? {$($r:tt)*}) => {
    #[allow(non_camel_case_types)]
    pub trait $name {
        type Ret = ();
        fn $name($($params)*) -> Self::Ret;
    }

    impl$(<$($ctx),*>)? $name for $base {
        $(type Ret = $ret;)?
        fn $name ($($params)*) -> Self::Ret {$($r)*}
    }
    };
}

ext_impl! {
for <T> fn <diesel::QueryResult<T>>.extract_not_found(self) -> WartIDResult<Option<T>> {
    match self {
        Ok(ok) => Ok(Some(ok)),
        Err(diesel::NotFound) => Ok(None),
        Err(err) => Err(WartIDError::Database(err)),
    }
}
}
