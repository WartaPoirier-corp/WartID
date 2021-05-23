#![feature(
    associated_type_defaults,
    proc_macro_hygiene,
    decl_macro,
    str_split_once
)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use rocket::config::{ConfigBuilder, Environment};
use rocket::http::{Cookie, Cookies, RawStr, Status};
use rocket::request::{Form, FromForm, FromRequest, Outcome};
use rocket::response::status::{NotFound, Unauthorized};
use rocket::response::{NamedFile, Redirect, Responder};
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

const BUILD_INFO: &str = build_info::format!("{} v{} built with {} at {}", $.crate_info.name, $.crate_info.version, $.compiler, $.timestamp);
const BUILD_INFO_GIT: &str = git_version::git_version!();

/// Ructe's parser is really bad and won't let us use "complex" types (that is, types with `<>`,
/// `::`, `[]`, etc. in their syntax), so I'm type-def-ing aliases here.
pub mod ructe_types {
    pub type Flashes<'a> = &'a [(std::borrow::Cow<'static, str>, bool)];
}

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = Config::load();
}

impl<'r> rocket::response::Responder<'r> for WartIDError {
    fn respond_to(self, request: &Request) -> rocket::response::Result<'r> {
        format!("{:#?}", self).respond_to(request)
    }
}

pub type DbConnection<'a> = &'a diesel::PgConnection;

#[database("wartid")]
pub struct DbConn(diesel::PgConnection);

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

impl<'a, 'r> FromRequest<'a, 'r> for &'a LoginSession {
    type Error = LoginSessionError;

    fn from_request(request: &'a Request) -> Outcome<Self, Self::Error> {
        let mut cookies: Cookies = request.guard().unwrap();

        request
            .local_cache(|| {
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

                let db: DbConn = request.guard().map_failure(|_| {
                    (
                        Status::InternalServerError,
                        LoginSessionError::DatabaseError,
                    )
                })?;

                // TODO following 2 assignments are dirty af
                let session = model::Session::find_by_id(&db, login_session)
                    .map_err(|_| Err((Status::Forbidden, LoginSessionError::InvalidSession)))?
                    .ok_or(Err((Status::Forbidden, LoginSessionError::InvalidSession)))?;

                let user = model::User::find_by_id(&db, session)
                    .map_err(|_| {
                        (
                            Status::InternalServerError,
                            LoginSessionError::DatabaseError,
                        )
                    })
                    .map_err(|_| {
                        Err((
                            // A session token should always correspond to a user
                            Status::InternalServerError,
                            LoginSessionError::DatabaseError,
                        ))
                    })?
                    .ok_or(Err((
                        // A session token should always correspond to a user
                        Status::InternalServerError,
                        LoginSessionError::DatabaseError,
                    )))?;

                Outcome::Success(LoginSession { user })
            })
            .as_ref()
            .map_failure(Clone::clone)
            .map_forward(|()| ())
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for model::PageContext {
    type Error = WartIDError;

    fn from_request(request: &'a Request) -> Outcome<Self, Self::Error> {
        let session: &LoginSession = request
            .guard()
            .map_failure(|(s, e)| (s, WartIDError::InvalidCredentials(format!("{:?}", e))))?;

        let db: DbConn = request
            .guard()
            .map_failure(|(s, ())| (s, WartIDError::DatabaseConnection))?;

        let ctx =
            Self::new(&db, session.user.id).map_err(|e| Err((Status::InternalServerError, e)))?;

        Outcome::Success(ctx)
    }
}

#[get("/static/<file..>")]
fn static_file(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path).map_err(|e| NotFound(e.to_string()))
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

#[allow(unused_variables)]
#[get("/login?<redirect_to>")]
pub fn login(
    session: Option<&LoginSession>,
    redirect_to: Option<&RawStr>, // Not used but required for rocket to be happy
) -> Result<Ructe, Redirect> {
    if session.is_some() {
        return Err(Redirect::to("/@me"));
    }

    Ok(render!(login::login()))
}

pub struct DoLoginResponse {
    pub user: crate::model::User,
    pub redirect_to: Option<String>,
}

impl<'r> Responder<'r> for DoLoginResponse {
    fn respond_to(self, request: &Request<'_>) -> rocket::response::Result<'r> {
        let db: DbConn = match request.guard() {
            Outcome::Success(x) => x,
            Outcome::Forward(_) => unimplemented!(),
            Outcome::Failure(_) => return WartIDError::DatabaseConnection.respond_to(request),
        };

        let mut cookies: Cookies = request.guard().expect("cannot get cookies");

        let session_id = match model::Session::insert(&db, model::NewSession::new(self.user.id)) {
            Ok(x) => x,
            Err(err) => return err.respond_to(request),
        };

        cookies.add(
            Cookie::build("login_session", session_id.to_string())
                .max_age(time::Duration::days(14))
                .finish(),
        );

        Redirect::to(
            self.redirect_to
                .unwrap_or_else(|| String::from("/@me"))
                .as_str()
                .to_string(),
        )
        .respond_to(request)
    }
}

#[cfg(feature = "discord_bot")]
#[get("/login-with-discord?<token>")]
pub fn login_with_discord(
    db: DbConn,
    token: &RawStr,
) -> Result<DoLoginResponse, Result<Unauthorized<Cow<'static, str>>, WartIDError>> {
    let user = match model::User::attempt_login(&db, "", token.as_str()) {
        Ok(Some(user)) => user,
        // 😓
        Ok(None) => {
            return Err(Ok(Unauthorized(Some(Cow::Borrowed(
                "Impossible de créer un compte utilisateur.",
            )))));
        }
        Err(WartIDError::InvalidCredentials(msg)) => {
            return Err(Ok(Unauthorized(Some(Cow::Owned(format!(
                "Jeton invalide, a-t-il expiré ? Message d'erreur: {}",
                msg
            ))))));
        }
        Err(other) => return Err(Err(other)),
    };

    Ok(DoLoginResponse {
        user,
        redirect_to: None,
    })
}

#[derive(FromForm)]
struct LoginCredentials<'a> {
    username: &'a RawStr,
    password: &'a RawStr,
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
fn login_post(
    db: DbConn,
    mut cookies: Cookies,
    form: Form<LoginCredentials>,
    redirect_to: Option<String>, // TODO Refactor these 2 lines to a tagged union ?
) -> Result<Redirect, WartIDError> {
    let res = model::User::attempt_login(&db, form.username.as_str(), form.password.as_str())?;

    let user = match res {
        Some(users_) => users_,
        None => return Err(WartIDError::Todo),
    };

    let session_id = model::Session::insert(&db, model::NewSession::new(user.id))?;

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
fn logout(mut cookies: Cookies) -> Redirect {
    cookies.remove(Cookie::named("login_session"));
    // TODO faire aussi sauter la session dans la BDD

    Redirect::to("/login")
}

fn main() {
    let _ = dotenv::dotenv();

    #[cfg(feature = "discord_bot")]
    {
        ctrlc::set_handler(model::discord_login_destroy).expect("cannot set ctrl+c handler");
        model::discord_login_init();
    }

    let config = {
        use std::collections::BTreeMap;

        /// Builds a single-element BTreeMap
        #[inline]
        fn b_tree_map<T>(k: &'static str, v: T) -> BTreeMap<&'static str, T> {
            let mut x = BTreeMap::new();
            x.insert(k, v);
            x
        }

        let config = ConfigBuilder::new(Environment::active().unwrap()).extra(
            "databases",
            b_tree_map(
                "wartid",
                b_tree_map(
                    "url",
                    std::env::var("DATABASE_URL").expect("no DATABASE_URL set"),
                ),
            ),
        );

        config.unwrap()
    };

    rocket::custom(config)
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
                routes::users::avatar,
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
        .launch();
}

include!(concat!(env!("OUT_DIR"), "/templates.rs"));
