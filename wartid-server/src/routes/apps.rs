use rocket::form::error::ErrorKind;
use rocket::form::{DataField, FromForm, Options, ValueField};

use super::prelude::*;

#[get("/apps")]
pub async fn list(ctx: PageContext, session: &LoginSession, db: DbConn) -> WartIDResult<Ructe> {
    let user_id = session.user.id;
    let apps = db_await!(UserApp::find_all(db, user_id))?;

    Ok(render!(panel::apps_list(&ctx, &apps[..])))
}

#[derive(FromForm)]
pub struct FormNewApp {
    name: String,
    hidden: bool,
}

#[post("/apps/new", data = "<data>")]
pub async fn new(
    session: &LoginSession,
    db: DbConn,
    data: Form<FormNewApp>,
) -> WartIDResult<Redirect> {
    let data = data.into_inner();

    let user_id = session.user.id;
    let id = db_await!(UserApp::insert(db, data.name, data.hidden, user_id))?;

    Ok(Redirect::to(format!("/apps/{}", id)))
}

fn view_render(ctx: PageContext, app: UserApp) -> WartIDResult<Option<Ructe>> {
    Ok(Some(render!(panel::app_view(&ctx, &app))))
}

#[get("/apps/<app_id>")]
pub async fn view(ctx: PageContext, db: DbConn, app_id: UserAppId) -> WartIDResult<Option<Ructe>> {
    let app = match db_await!(UserApp::find_by_id(db, app_id))? {
        Some(app) => app,
        None => return Ok(None),
    };

    view_render(ctx, app)
}

#[derive(Debug)]
pub enum FormUpdateIntent {
    UpdateGeneral { name: String, description: String },
    OAuthSetRedirectUri(String),
    OAuthEnable,
    OAuthDisable,
}

#[derive(Debug, FromForm)]
pub struct FormUpdateIntentRaw {
    name: Option<String>,
    description: Option<String>,
    #[field(name = "oauth-redirect")]
    oauth_redirect_uri: Option<String>,

    // Buttons (mutually exclusive)
    #[field(name = "update-general", default = false)]
    update_general: bool,
    #[field(name = "oauth-enable", default = false)]
    oauth_enable: bool,
    #[field(name = "oauth-disable", default = false)]
    oauth_disable: bool,
    #[field(name = "oauth-update-redirect", default = false)]
    oauth_update_redirect: bool,
}

#[rocket::async_trait]
impl<'r> FromForm<'r> for FormUpdateIntent {
    type Context = <FormUpdateIntentRaw as FromForm<'r>>::Context;

    fn init(opts: Options) -> Self::Context {
        FormUpdateIntentRaw::init(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) {
        FormUpdateIntentRaw::push_value(ctxt, field)
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>) {
        FormUpdateIntentRaw::push_data(ctxt, field).await;
    }

    fn finalize(ctxt: Self::Context) -> rocket::form::Result<'r, Self> {
        Ok(match FormUpdateIntentRaw::finalize(ctxt)? {
            FormUpdateIntentRaw {
                name: Some(name),
                description: Some(description),
                oauth_redirect_uri: None,
                update_general: true,
                oauth_enable: false,
                oauth_disable: false,
                oauth_update_redirect: false,
            } => FormUpdateIntent::UpdateGeneral { name, description },
            FormUpdateIntentRaw {
                name: None,
                description: None,
                oauth_redirect_uri: None,
                update_general: false,
                oauth_enable: true,
                oauth_disable: false,
                oauth_update_redirect: false,
            } => FormUpdateIntent::OAuthEnable,
            FormUpdateIntentRaw {
                name: None,
                description: None,
                oauth_redirect_uri: None,
                update_general: false,
                oauth_enable: false,
                oauth_disable: true,
                oauth_update_redirect: false,
            } => FormUpdateIntent::OAuthDisable,
            FormUpdateIntentRaw {
                name: None,
                description: None,
                oauth_redirect_uri: Some(uri),
                update_general: false,
                oauth_enable: false,
                oauth_disable: false,
                oauth_update_redirect: true,
            } => FormUpdateIntent::OAuthSetRedirectUri(uri),
            _ => Err(ErrorKind::Duplicate)?,
        })
    }

    fn push_error(ctxt: &mut Self::Context, error: rocket::form::Error<'r>) {
        FormUpdateIntentRaw::push_error(ctxt, error)
    }
}

#[post("/apps/<app_id>", data = "<data>")]
pub async fn view_update(
    mut ctx: PageContext,
    db: DbConn,
    app_id: UserAppId,
    data: Form<FormUpdateIntent>,
) -> WartIDResult<Option<Ructe>> {
    let (app, success_message) = match data.into_inner() {
        FormUpdateIntent::UpdateGeneral { name, description } => {
            if name.len() < 3 {
                ctx.add_flash_message(
                    Cow::Borrowed("Le nom de la WartApp doit faire minimum 3 caractères de long."),
                    true,
                );
                return view_render(
                    ctx,
                    match db_await!(UserApp::find_by_id(db, app_id)) {
                        Ok(Some(app)) => app,
                        Ok(None) => return Ok(None),
                        Err(err) => return Err(err),
                    },
                );
            }

            (
                db_await!(UserApp::set_name_description(
                    db,
                    app_id,
                    &name,
                    &description
                ))?,
                "Nom et/ou description de l'app mis·es à jour avec succès.",
            )
        }
        FormUpdateIntent::OAuthEnable => (
            db_await!(UserApp::set_oauth(db, app_id, true))?,
            "Secret OAuth2 généré.",
        ),
        FormUpdateIntent::OAuthDisable => (
            db_await!(UserApp::set_oauth(db, app_id, false))?,
            "OAuth2 désactivé.",
        ),
        FormUpdateIntent::OAuthSetRedirectUri(uri) => (
            db_await!(UserApp::set_oauth_redirect_uri(db, app_id, uri))?,
            "URI de redirection OAuth2 autorisé mis à jour.",
        ),
    };

    ctx.add_flash_message(Cow::Borrowed(success_message), false);

    view_render(ctx, app)
}
