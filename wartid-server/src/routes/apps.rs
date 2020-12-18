use super::prelude::*;

#[get("/apps")]
pub fn list(db: DbConn) -> WartIDResult<Ructe> {
    Ok(render!(panel::apps_list(&Menu::build(&db)?)))
}

#[get("/apps/new")]
pub fn new() {}
