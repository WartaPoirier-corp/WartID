use super::prelude::*;
use rocket::http::RawStr;
use rocket::request::{FormItems, FormParseError, FromForm, FromParam};
use uuid::Error;

pub struct UuidParamWithAt(UuidParam);

#[derive(Debug)]
pub enum UuidParamWithAtError {
    NoAtSymbol,
    Uuid(uuid::Error),
}

impl From<uuid::Error> for UuidParamWithAtError {
    fn from(e: Error) -> Self {
        Self::Uuid(e)
    }
}

impl<'a> FromParam<'a> for UuidParamWithAt {
    type Error = UuidParamWithAtError;

    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
        if let Some(param) = param.strip_prefix('@') {
            Ok(UuidParamWithAt(UuidParam::from_param(param.into())?))
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
pub fn view(
    ctx: PageContext,
    session: &LoginSession,
    db: DbConn,
    user_id: UuidParamWithAt,
) -> WartIDResult<Option<Ructe>> {
    let user_id = *user_id.0;

    let user = match User::find_by_id(&*db, user_id) {
        Ok(Some(user)) => user,
        Ok(None) => return Ok(None),
        Err(err) => return Err(err),
    };

    return Ok(Some(render!(panel::user_view(
        &ctx;
        &user,
        session.user.id == user_id
    ))));
}

#[derive(Debug)]
pub enum FormUpdateIntent {
    UpdateName(String),
    UpdateEmail(String),
    UpdatePassword(String),
}

impl<'a> FromForm<'a> for FormUpdateIntent {
    type Error = FormParseError<'a>;

    fn from_form(it: &mut FormItems<'a>, strict: bool) -> Result<Self, Self::Error> {
        #[derive(FromForm)]
        struct FormUpdateIntentRaw<'a> {
            name: Option<String>,
            email: Option<String>,
            password: Option<String>,

            // Buttons (mutually exclusive)
            #[form(field = "update-name")]
            update_name: Option<&'a RawStr>,
            #[form(field = "update-email")]
            update_email: Option<&'a RawStr>,
            #[form(field = "update-password")]
            oauth_password: Option<&'a RawStr>,
        }

        Ok(match FormUpdateIntentRaw::from_form(it, strict)? {
            FormUpdateIntentRaw {
                name: Some(name),
                email: None,
                password: None,
                update_name: Some(_),
                update_email: None,
                oauth_password: None,
            } => FormUpdateIntent::UpdateName(name),
            FormUpdateIntentRaw {
                name: None,
                email: Some(email),
                password: None,
                update_name: None,
                update_email: Some(_),
                oauth_password: None,
            } => FormUpdateIntent::UpdateEmail(email),
            FormUpdateIntentRaw {
                name: None,
                email: None,
                password: Some(password),
                update_name: None,
                update_email: None,
                oauth_password: Some(_),
            } => FormUpdateIntent::UpdatePassword(password),
            _ => Err(FormParseError::Unknown("?".into(), "?".into()))?,
        })
    }
}

#[post("/<user_id>", data = "<data>")]
pub fn view_update(
    mut ctx: PageContext,
    session: &LoginSession,
    db: DbConn,
    user_id: UuidParamWithAt,
    data: Form<FormUpdateIntent>,
) -> WartIDResult<Ructe> {
    let user_id = *user_id.0;

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
                return Ok(render!(panel::user_view(&ctx; &session.user, true)));
            };

            (
                User::update_username(&db, user_id, &name)?,
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
                return Ok(render!(panel::user_view(&ctx; &session.user, true)));
            };

            (
                User::update_email(&db, user_id, &email)?,
                "Adresse e-mail mise à jour avec succès !",
            )
        }
        FormUpdateIntent::UpdatePassword(password) => {
            if password.len() < 8 {
                ctx.add_flash_message(
                    Cow::Borrowed("Le mot de passe doit faire minimum 8 caractères."),
                    true,
                );
                return Ok(render!(panel::user_view(&ctx; &session.user, true)));
            };

            (
                User::update_password(&db, user_id, &password)?,
                "Mot de passe mis à jour avec succès !",
            )
        }
    };

    ctx.add_flash_message(Cow::Borrowed(success_message), false);

    return Ok(render!(panel::user_view(&ctx; &user, true)));
}
