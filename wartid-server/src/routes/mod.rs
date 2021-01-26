pub mod apps;
pub mod oauth2;
pub mod users;

/// Prelude for child modules
mod prelude {
    pub use std::borrow::Cow;

    pub use rocket::request::Form;
    pub use rocket::response::Redirect;
    pub use uuid::Uuid;

    pub use crate::model::*;
    pub use crate::ructe::*;
    pub use crate::DbConn;
    pub use crate::LoginSession;
}
