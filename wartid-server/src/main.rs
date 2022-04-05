#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate rocket;

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::fs::NamedFile;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::outcome::{try_outcome, IntoOutcome};
use rocket::request::{FromRequest, Outcome};
use rocket::response::status::NotFound;
use rocket::response::Redirect;
use rocket::Request;
use uuid::Uuid;

use crate::config::Config;
use crate::model::{WartIDError, WartIDResult};
use crate::ructe::Ructe;

#[macro_use]
mod id;

#[macro_use]
mod ructe;
mod config;
mod model;
mod routes;
mod schema;
mod utils;

embed_migrations!();

const BUILD_INFO: &str = build_info::format!("{} v{} built with {} at {}", $.crate_info.name, $.crate_info.version, $.compiler, $.timestamp);
const BUILD_INFO_GIT: Option<&'static str> = std::option_env!("GIT_REV");

const SESSION_COOKIE_EXPIRATION: time::Duration = time::Duration::days(14);

#[macro_export]
macro_rules! db_await {
    ($($call_path:ident)::* ($db:ident, $($arg:expr),*$(,)?)) => {
        $db.run(move |$db| $($call_path)::*($db, $($arg),*)).await
    };
}

/// Ructe's parser is really bad and won't let us use "complex" types (that is, types with `<>`,
/// `::`, `[]`, etc. in their syntax), so I'm type-def-ing aliases here.
pub mod ructe_types {
    pub type Flashes<'a> = &'a [(std::borrow::Cow<'static, str>, bool)];
}

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = Config::load();
}

impl<'r> rocket::response::Responder<'r, 'static> for WartIDError {
    fn respond_to(self, request: &Request) -> rocket::response::Result<'static> {
        format!("{:#?}", self).respond_to(request)
    }
}

pub type DbConnection<'a> = &'a diesel::PgConnection;

#[rocket_sync_db_pools::database("wartid")]
pub struct DbConn(rocket_sync_db_pools::diesel::PgConnection);

pub struct LoginSession {
    user: model::User,
}

#[derive(Clone, Copy, Debug)]
pub enum LoginSessionError {
    NoCookie,
    InvalidCookie,
    InvalidSession,
    DatabaseError,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r LoginSession {
    type Error = LoginSessionError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies: &CookieJar = request.guard().await.unwrap();

        request
            .local_cache_async(async {
                // TODO const cookie name
                let login_session = match cookies.get("login_session") {
                    None => {
                        return Outcome::Failure((Status::Forbidden, LoginSessionError::NoCookie));
                    }
                    Some(cookie) => {
                        match cookie.value().parse::<Uuid>() {
                            Ok(session_uuid) => session_uuid,
                            Err(_) => {
                                cookies.remove(Cookie::named("login_session")); // TODO check if works
                                return Outcome::Failure((
                                    Status::Forbidden,
                                    LoginSessionError::InvalidCookie,
                                ));
                            }
                        }
                    }
                };

                let db: DbConn = try_outcome!(request.guard().await.map_failure(|_| {
                    (
                        Status::InternalServerError,
                        LoginSessionError::DatabaseError,
                    )
                }));

                // TODO dirty af
                let session = {
                    let opt_session =
                        try_outcome!(db_await!(model::Session::find_by_id(db, login_session))
                            .map_err(|_| LoginSessionError::InvalidSession)
                            .into_outcome(Status::Forbidden));

                    match opt_session {
                        Some(session) => session,
                        None => {
                            return Err(LoginSessionError::InvalidSession)
                                .into_outcome(Status::BadRequest);
                        }
                    }
                };

                let user = {
                    let opt_user = try_outcome!(db_await!(model::User::find_by_id(db, session))
                        .map_err(|_| LoginSessionError::DatabaseError) // Request failure
                        .into_outcome(Status::InternalServerError));

                    try_outcome!(opt_user
                        // A session token should always correspond to a user
                        .ok_or(LoginSessionError::DatabaseError)
                        .into_outcome(Status::InternalServerError))
                };

                Outcome::Success(LoginSession { user })
            })
            .await
            .as_ref()
            .map_failure(Clone::clone)
            .map_forward(|()| ())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for model::PageContext {
    type Error = WartIDError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let session: &LoginSession = try_outcome!(request
            .guard()
            .await
            .map_failure(|(s, e)| (s, WartIDError::InvalidCredentials(format!("{:?}", e)))));

        let db: DbConn = try_outcome!(request
            .guard()
            .await
            .map_failure(|(s, ())| (s, WartIDError::DatabaseConnection)));

        let user_id = session.user.id;
        let ctx = try_outcome!(
            db_await!(Self::new(db, user_id)).into_outcome(Status::InternalServerError)
        );

        Outcome::Success(ctx)
    }
}

#[get("/static/<file..>")]
async fn static_file(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path)
        .await
        .map_err(|e| NotFound(e.to_string()))
}

#[get("/")]
fn root(session: Option<&LoginSession>) -> Redirect {
    if session.is_some() {
        Redirect::to("/home")
    } else {
        Redirect::to("/login")
    }
}

#[get("/home")]
fn home(ctx: model::PageContext) -> WartIDResult<Ructe> {
    Ok(render!(panel::home(&ctx)))
}

#[get("/login?<redirect_to>")]
pub fn login(
    session: Option<&LoginSession>,
    #[allow(unused_variables)] redirect_to: Option<&str>, // Not used but required for rocket to be happy
) -> Result<Ructe, Redirect> {
    if session.is_some() {
        return Err(Redirect::to("/@me"));
    }

    Ok(render!(login::login()))
}

#[cfg(feature = "discord_bot")]
#[get("/login-with-discord?<token>")]
pub async fn login_with_discord(
    db: DbConn,
    cookies: &CookieJar<'_>,
    token: String,
) -> Result<Redirect, Result<(Status, Cow<'static, str>), WartIDError>> {
    let user = match db_await!(model::User::attempt_login(db, "", &token)) {
        Ok(Some(user)) => user,
        // 😓
        Ok(None) => {
            return Err(Ok((
                Status::Unauthorized,
                Cow::Borrowed("Impossible de créer un compte utilisateur."),
            )));
        }
        Err(WartIDError::InvalidCredentials(msg)) => {
            return Err(Ok((
                Status::Unauthorized,
                Cow::Owned(format!(
                    "Jeton invalide, a-t-il expiré ? Message d'erreur: {}",
                    msg
                )),
            )));
        }
        Err(other) => return Err(Err(other)),
    };

    let user_id = user.id;
    let session_id = match db_await!(model::Session::insert(db, model::NewSession::new(user_id))) {
        Ok(x) => x,
        Err(err) => return Err(Err(err)),
    };

    cookies.add(
        Cookie::build("login_session", session_id.to_string())
            .max_age(SESSION_COOKIE_EXPIRATION)
            .finish(),
    );

    Ok(Redirect::to("/@me"))
}

#[derive(FromForm)]
struct LoginCredentials {
    username: String,
    password: String,
}

/// ### Main login route
///
/// Depending on the query parameters, redirects to different pages:
///   * `?redirect_to=` exists: redirects to the given URL
///   * OIDC `authorize` request's query parameters present: redirects to [redirect_uri](AuthorizeQuery::redirect_uri)
///   * _else:_ Redirects to `/@<profile>`
/// These cases are mutually exclusive. Errors may be reported if conflicting query parameters are
/// received.
#[post("/login?<redirect_to>", data = "<form>")]
async fn login_post(
    db: DbConn,
    cookies: &CookieJar<'_>,
    form: Form<LoginCredentials>,
    redirect_to: Option<String>, // TODO Refactor these 2 lines to a tagged union ?
) -> Result<Redirect, WartIDError> {
    let res = db_await!(model::User::attempt_login(
        db,
        &form.username,
        &form.password
    ))?;

    let user = match res {
        Some(users_) => users_,
        None => return Err(WartIDError::Todo),
    };

    let user_id = user.id;
    let session_id = db_await!(model::Session::insert(db, model::NewSession::new(user_id)))?;

    cookies.add(
        Cookie::build("login_session", session_id.to_string())
            .max_age(time::Duration::days(14))
            .finish(),
    );

    Ok(Redirect::to(
        redirect_to
            .unwrap_or_else(|| String::from("/@me"))
            .as_str()
            .to_string(),
    ))
}

// TODO CSRF
#[post("/logout")]
fn logout(cookies: &CookieJar) -> Redirect {
    cookies.remove(Cookie::named("login_session"));
    // TODO faire aussi sauter la session dans la BDD

    Redirect::to("/login")
}

#[rocket::launch]
async fn launch() -> _ {
    let _ = dotenv::dotenv();

    #[cfg(feature = "discord_bot")]
    {
        ctrlc::set_handler(model::discord_login_destroy).expect("cannot set ctrl+c handler");
        model::discord_login_init();
    }

    rocket::build()
        .attach(DbConn::fairing())
        .mount(
            "/",
            routes![
                static_file,
                root,
                home,
                routes::apps::list,
                routes::apps::new,
                routes::apps::view,
                routes::apps::view_update,
                routes::oauth2::authorize,
                routes::oauth2::token,
                routes::oauth2::userinfo,
                routes::users::view,
                routes::users::view_me,
                routes::users::view_update,
                login,
                login_post,
                logout,
            ],
        )
        .mount("/", {
            #[cfg(feature = "discord_bot")]
            let routes = routes![login_with_discord];
            #[cfg(not(feature = "discord_bot"))]
            let routes = routes![];
            routes
        })
        .attach(AdHoc::on_liftoff("migration runner", |rocket| {
            Box::pin(async move {
                let conn = DbConn::get_one(rocket)
                    .await
                    .expect("no database available for running migrations");

                conn.run(|c| embedded_migrations::run_with_output(c, &mut std::io::stdout()))
                    .await
                    .unwrap();
            })
        }))
}

include!(concat!(env!("OUT_DIR"), "/templates.rs"));
