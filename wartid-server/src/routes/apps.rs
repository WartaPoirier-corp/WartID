use super::prelude::*;
use rocket::http::RawStr;
use rocket::request::{FormItems, FormParseError, FromForm};
use std::ops::Deref;

#[get("/apps")]
pub fn list(menu: Menu, session: &LoginSession, db: DbConn) -> WartIDResult<Ructe> {
    let apps = UserApp::find_all(&db, session.user.id)?;

    Ok(render!(panel::apps_list(&menu, &apps[..])))
}

#[derive(FromForm)]
pub struct FormNewApp {
    name: String,
    hidden: bool,
}

#[post("/apps/new", data = "<data>")]
pub fn new(session: &LoginSession, db: DbConn, data: Form<FormNewApp>) -> WartIDResult<Redirect> {
    let data = data.into_inner();

    let id = UserApp::insert(&db, data.name, data.hidden, session.user.id)?;

    Ok(Redirect::to(format!("/apps/{}", id)))
}

fn view_render(menu: Menu, app: UserApp) -> WartIDResult<Option<Ructe>> {
    Ok(Some(render!(panel::app_view(&menu, &app))))
}

#[get("/apps/<app_id>")]
pub fn view(menu: Menu, db: DbConn, app_id: UuidParam) -> WartIDResult<Option<Ructe>> {
    let app = match UserApp::find_by_id(&db, *app_id)? {
        Some(app) => app,
        None => return Ok(None),
    };

    view_render(menu, app)
}

#[derive(Debug)]
pub enum FormUpdateIntent {
    UpdateGeneral { name: String, description: String },
    OAuthSetRedirectUri(String),
    OAuthEnable,
    OAuthDisable,
}

impl<'a> FromForm<'a> for FormUpdateIntent {
    type Error = FormParseError<'a>;

    fn from_form(it: &mut FormItems<'a>, strict: bool) -> Result<Self, Self::Error> {
        #[derive(Debug, FromForm)]
        struct FormUpdateIntentRaw<'a> {
            name: Option<String>,
            description: Option<String>,
            #[form(field = "oauth-redirect")]
            oauth_redirect_uri: Option<String>,

            // Buttons (mutually exclusive)
            #[form(field = "update-general")]
            update_general: Option<&'a RawStr>,
            #[form(field = "oauth-enable")]
            oauth_enable: Option<&'a RawStr>,
            #[form(field = "oauth-disable")]
            oauth_disable: Option<&'a RawStr>,
            #[form(field = "oauth-update-redirect")]
            oauth_update_redirect: Option<&'a RawStr>,
        }

        Ok(match FormUpdateIntentRaw::from_form(it, strict)? {
            FormUpdateIntentRaw {
                name: Some(name),
                description: Some(description),
                oauth_redirect_uri: None,
                update_general: Some(_),
                oauth_enable: None,
                oauth_disable: None,
                oauth_update_redirect: None,
            } => FormUpdateIntent::UpdateGeneral { name, description },
            FormUpdateIntentRaw {
                name: None,
                description: None,
                oauth_redirect_uri: None,
                update_general: None,
                oauth_enable: Some(_),
                oauth_disable: None,
                oauth_update_redirect: None,
            } => FormUpdateIntent::OAuthEnable,
            FormUpdateIntentRaw {
                name: None,
                description: None,
                oauth_redirect_uri: None,
                update_general: None,
                oauth_enable: None,
                oauth_disable: Some(_),
                oauth_update_redirect: None,
            } => FormUpdateIntent::OAuthDisable,
            FormUpdateIntentRaw {
                name: None,
                description: None,
                oauth_redirect_uri: Some(uri),
                update_general: None,
                oauth_enable: None,
                oauth_disable: None,
                oauth_update_redirect: Some(_),
            } => FormUpdateIntent::OAuthSetRedirectUri(uri),
            _ => Err(FormParseError::Unknown("?".into(), "?".into()))?,
        })
    }
}

#[post("/apps/<app_id>", data = "<data>")]
pub fn view_update(
    menu: Menu,
    db: DbConn,
    app_id: UuidParam,
    data: Form<FormUpdateIntent>,
) -> WartIDResult<Option<Ructe>> {
    let app_id = *app_id;

    let app = match data.into_inner() {
        FormUpdateIntent::UpdateGeneral { name, description } => {
            UserApp::set_name_description(&db, app_id, &name, &description)
        }
        FormUpdateIntent::OAuthEnable => UserApp::set_oauth(&db, app_id, true),
        FormUpdateIntent::OAuthDisable => UserApp::set_oauth(&db, app_id, false),
        FormUpdateIntent::OAuthSetRedirectUri(uri) => {
            UserApp::set_oauth_redirect_uri(&db, app_id, uri)
        }
    }?;

    view_render(menu, app)
}
