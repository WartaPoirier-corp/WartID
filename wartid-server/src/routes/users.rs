use rocket::form::error::ErrorKind;
use rocket::form::{DataField, FromForm, Options, ValueField};
use rocket::request::FromParam;
use uuid::Error as UuidError;

use super::prelude::*;

pub struct UuidParamWithAt(UserId);

#[derive(Debug, thiserror::Error)]
pub enum UuidParamWithAtError {
    #[error("missing '@' prefix")]
    NoAtSymbol,

    #[error(transparent)]
    Uuid(#[from] UuidError),
}

impl<'a> FromParam<'a> for UuidParamWithAt {
    type Error = UuidParamWithAtError;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        if let Some(param) = param.strip_prefix('@') {
            Ok(UuidParamWithAt(UserId::from_param(param)?))
        } else {
            Err(UuidParamWithAtError::NoAtSymbol)
        }
    }
}

#[get("/@me")]
pub fn view_me(session: &LoginSession) -> Redirect {
    Redirect::to(format!("/@{}", session.user.id))
}

#[get("/<user_id>")]
pub async fn view(
    ctx: PageContext,
    session: &LoginSession,
    db: DbConn,
    user_id: UuidParamWithAt,
) -> WartIDResult<Option<Ructe>> {
    let user_id = user_id.0;

    let user = match db_await!(User::find_by_id(db, user_id)) {
        Ok(Some(user)) => user,
        Ok(None) => return Ok(None),
        Err(err) => return Err(err),
    };

    Ok(Some(render!(panel::user_view_html(
        &ctx;
        &user,
        session.user.id == user_id
    ))))
}

#[derive(Debug)]
pub enum FormUpdateIntent {
    UpdateName(String),
    UpdateEmail(String),
    UpdatePassword(String),
}

#[derive(FromForm)]
pub struct FormUpdateIntentRaw {
    name: Option<String>,
    email: Option<String>,
    password: Option<String>,

    // Buttons (mutually exclusive)
    #[field(name = "update-name", default = false)]
    update_name: bool,
    #[field(name = "update-email", default = false)]
    update_email: bool,
    #[field(name = "update-password", default = false)]
    oauth_password: bool,
}

#[rocket::async_trait]
impl<'r> FromForm<'r> for FormUpdateIntent {
    type Context = <FormUpdateIntentRaw as FromForm<'r>>::Context;

    fn init(opts: Options) -> Self::Context {
        FormUpdateIntentRaw::init(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) {
        FormUpdateIntentRaw::push_value(ctxt, field);
    }

    #[must_use]
    async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>) {
        FormUpdateIntentRaw::push_data(ctxt, field).await;
    }

    fn push_error(ctxt: &mut Self::Context, error: rocket::form::Error<'r>) {
        FormUpdateIntentRaw::push_error(ctxt, error);
    }

    fn finalize(ctxt: Self::Context) -> rocket::form::Result<'r, Self> {
        Ok(match FormUpdateIntentRaw::finalize(ctxt)? {
            FormUpdateIntentRaw {
                name: Some(name),
                email: None,
                password: None,
                update_name: true,
                update_email: false,
                oauth_password: false,
            } => FormUpdateIntent::UpdateName(name),
            FormUpdateIntentRaw {
                name: None,
                email: Some(email),
                password: None,
                update_name: false,
                update_email: true,
                oauth_password: false,
            } => FormUpdateIntent::UpdateEmail(email),
            FormUpdateIntentRaw {
                name: None,
                email: None,
                password: Some(password),
                update_name: false,
                update_email: false,
                oauth_password: true,
            } => FormUpdateIntent::UpdatePassword(password),
            _ => Err(ErrorKind::Duplicate)?,
        })
    }
}

#[post("/<user_id>", data = "<data>")]
pub async fn view_update(
    mut ctx: PageContext,
    session: &LoginSession,
    db: DbConn,
    user_id: UuidParamWithAt,
    data: Form<FormUpdateIntent>,
) -> WartIDResult<Ructe> {
    let user_id = user_id.0;

    if user_id != session.user.id {
        return Err(WartIDError::InvalidCredentials(String::from(
            "invalid account",
        )));
    }

    let (user, success_message) = match data.into_inner() {
        FormUpdateIntent::UpdateName(name) => {
            if name.len() < 3 {
                ctx.add_flash_message(
                    Cow::Borrowed("Le nom doit faire minimum 3 caractères."),
                    true,
                );
                return Ok(render!(panel::user_view_html(&ctx; &session.user, true)));
            };

            (
                db_await!(User::update_username(db, user_id, &name))?,
                "Nom mis à jour avec succès !",
            )
        }
        FormUpdateIntent::UpdateEmail(email) => {
            // TODO real verification
            if email.len() < 6 && !email.contains('@') {
                ctx.add_flash_message(
                    Cow::Borrowed("Merci de rentrer une adresse e-mail valide."),
                    true,
                );
                return Ok(render!(panel::user_view_html(&ctx; &session.user, true)));
            };

            (
                db_await!(User::update_email(db, user_id, &email))?,
                "Adresse e-mail mise à jour avec succès !",
            )
        }
        FormUpdateIntent::UpdatePassword(password) => {
            if password.len() < 8 {
                ctx.add_flash_message(
                    Cow::Borrowed("Le mot de passe doit faire minimum 8 caractères."),
                    true,
                );
                return Ok(render!(panel::user_view_html(&ctx; &session.user, true)));
            };

            (
                db_await!(User::update_password(db, user_id, &password))?,
                "Mot de passe mis à jour avec succès !",
            )
        }
    };

    ctx.add_flash_message(Cow::Borrowed(success_message), false);

    Ok(render!(panel::user_view_html(&ctx; &user, true)))
}
