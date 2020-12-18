pub mod apps;

/// Prelude for child modules
mod prelude {
    pub use crate::model::*;
    pub use crate::ructe::*;
    pub use crate::DbConn;
}
