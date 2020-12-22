pub mod apps;
pub mod oauth2;
pub mod users;

/// Prelude for child modules
mod prelude {
    pub struct UuidParam(Uuid);

    impl std::ops::Deref for UuidParam {
        type Target = Uuid;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a> FromParam<'a> for UuidParam {
        type Error = uuid::Error;

        fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
            param.parse().map(UuidParam)
        }
    }

    pub use crate::model::*;
    pub use crate::ructe::*;
    pub use crate::DbConn;
    pub use crate::LoginSession;
    use rocket::http::RawStr;
    pub use rocket::request::Form;
    use rocket::request::{FromParam, Outcome};
    pub use rocket::response::Redirect;
    use rocket::Request;
    pub use uuid::Uuid;
}
