#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;

#[macro_use]
mod ructe;
mod model;
mod schema;

use crate::model::WartIDError;
use crate::ructe::Ructe;
use crate::schema::sessions::dsl::sessions;
use diesel::prelude::*;
use rocket::config::{ConfigBuilder, Environment};
use rocket::fairing::Info;
use rocket::http::route::Kind::Static;
use rocket::http::{Cookie, Cookies, RawStr, Status};
use rocket::request::{
    Form, FormItem, FormItems, FormParseError, FromForm, FromParam, FromRequest, Outcome,
};
use rocket::response::status::NotFound;
use rocket::response::{NamedFile, Redirect};
use rocket::{Request, Rocket};
use std::path::{Path, PathBuf};
use uuid::Uuid;

impl<'r> rocket::response::Responder<'r> for WartIDError {
    fn respond_to(self, request: &Request) -> rocket::response::Result<'r> {
        format!("{:#?}", self).respond_to(request)
    }
}

pub type DbConnection<'a> = &'a diesel::PgConnection;

#[database("wartid")]
pub struct DbConn(diesel::PgConnection);

struct OIDCState {}

struct LoginSession {
    user: model::User,
}

#[derive(Debug)]
enum LoginSessionError {
    NoCookie,
    InvalidCookie,
    InvalidSession,
    DatabaseError,
}

impl<'a, 'r> FromRequest<'a, 'r> for LoginSession {
    type Error = LoginSessionError;

    fn from_request(request: &Request) -> Outcome<Self, Self::Error> {
        let mut cookies: Cookies = request.guard().unwrap();

        // TODO const cookie name
        let login_session = match cookies.get("login_session") {
            None => return Outcome::Failure((Status::Forbidden, LoginSessionError::NoCookie)),
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
    }
}

#[get("/static/<file..>")]
fn static_file(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path).map_err(|e| NotFound(e.to_string()))
}

#[get("/")]
fn hello() -> &'static str {
    r#"
    Bonjour et bienvenue sur WartID
    "#
}

struct UserIdParam<'a>(&'a str);

impl<'a> FromParam<'a> for UserIdParam<'a> {
    type Error = (); // TODO

    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
        if param.starts_with('@') {
            Ok(UserIdParam(&param.as_str()[1..]))
        } else {
            Err(())
        }
    }
}

#[get("/@me")]
fn user_view_me(session: LoginSession) -> Redirect {
    Redirect::to(format!("/@{}", session.user.id))
}

#[get("/<user_id>")]
fn user_view(_guard: LoginSession, db: DbConn, user_id: UserIdParam) -> Option<Ructe> {
    let user_id: Uuid = user_id.0.parse().ok()?; // TODO proper 404

    let user = model::User::find_by_id(&*db, user_id).ok()??; // TODO proper handling

    let username_ = format!(
        "@{}",
        user.username
            .as_ref()
            .unwrap_or(&String::from("Utilisateur·ice inconnu·e"))
    );
    return Some(render!(panel::users(
        &model::Menu::build(&*db).ok()?, // TODO ok -> actual result
        &username_,
        user
    )));

    //return Some(session.user.username.unwrap_or(String::from("<?>")));

    //Some(format!("{:#?}", &users_[0]))
}

#[get("/login")]
fn login(session: Option<LoginSession>) -> Result<Ructe, Redirect> {
    if session.is_some() {
        return Err(Redirect::to("/@me"));
    }

    Ok(render!(login::login()))
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
#[post("/login?<redirect_to>&<oidc_flow..>", data = "<form>")]
fn login_post(
    db: DbConn,
    mut cookies: Cookies,
    form: Form<LoginCredentials>,
    redirect_to: Option<&RawStr>, // TODO Refactor these 2 lines to a tagged union ?
    oidc_flow: Option<Form<AuthorizeQuery>>,
) -> Result<Redirect, WartIDError> {
    use schema::users::dsl::*;

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
            .unwrap_or(RawStr::from_str("/@me"))
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

#[derive(FromForm, Debug)]
struct AuthorizeQuery<'a> {
    client_id: &'a RawStr,
    redirect_uri: &'a RawStr,
    scope: &'a RawStr,
    response_type: &'a RawStr,
    response_mode: &'a RawStr,
    state: Option<&'a RawStr>,
    nonce: Option<&'a RawStr>,
}

//register
//{"application_type":"web","client_id":"ezaF2UVDLiak","client_id_issued_at":1606665967,"client_secret":"880006882680470fa6b281a3a13e7fc6","client_secret_expires_at":0,"redirect_uris":["https://oidcdebugger.com/debug"],"response_types":["code"]}

#[get("/oidc/authorize?<authorize..>")]
fn oidc_authorize(
    authorize: Result<Form<AuthorizeQuery>, FormParseError>,
) -> Result<Redirect, String> {
    match authorize {
        Ok(authorize) => Ok({
            let mut redirect_uri = authorize
                .redirect_uri
                .url_decode()
                .map_err(|e| e.to_string())?;

            redirect_uri.push_str("?code=c177b7b88e014d92980534cc544380cc");

            if let Some(state) = authorize.state {
                redirect_uri.push_str("&state=");
                redirect_uri.push_str(state);
            }

            println!("{}", &redirect_uri);

            Redirect::to(redirect_uri)
        }),
        Err(e) => Err(format!("error: {:?}", e)),
    }
}

#[derive(FromForm, Debug)]
struct TokenQuery<'a> {
    grant_type: &'a RawStr,
    code: &'a RawStr,
    client_id: &'a RawStr,
    client_secret: &'a RawStr,
    redirect_uri: &'a RawStr,
}

/// Other auth methods: https://darutk.medium.com/oauth-2-0-client-authentication-4b5f929305d4
#[derive(Clone, Debug, Eq, PartialEq)]
struct BasicAuthorization {
    username: String,
    password: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for BasicAuthorization {
    type Error = String;

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let auth = match request.headers().get("Authorization").next() {
            Some(auth) => auth,
            None => {
                return Outcome::Failure((
                    Status::Forbidden,
                    String::from("No Authorization header"),
                ))
            }
        };

        if !auth.starts_with("Basic ") {
            return Outcome::Failure((
                Status::BadRequest,
                String::from("Only basic auth is supported"),
            ));
        }

        let auth = &auth[6..];
        let auth = base64::decode(auth).map_err(|e| e.to_string()).unwrap(); // FIXME
        let auth = std::str::from_utf8(&auth)
            .map_err(|e| e.to_string())
            .unwrap(); // FIXME

        let colon = auth
            .find(':')
            .ok_or(String::from("Bad format, missing colon"))
            .unwrap(); // FIXME

        Outcome::Success(BasicAuthorization {
            username: String::from(&auth[..colon]),
            password: String::from(&auth[(colon + 1)..]),
        })
    }
}

// {"access_token":"f592dd2cb2614284989bdffa288df8d9","expires_in":3600,"id_token":"eyJhbGciOiJSUzI1NiJ9.eyJpc3MiOiAiaHR0cHM6Ly9sb2NhbGhvc3Q6OTA5MCIsICJzdWIiOiAiYTgyMGJjY2Y5YTdiZTI1ZDQwMGFhN2U3YjdhNDZiMTMzZTFjYTk1MDhlNDQyZmJhN2VhNTBiOTRiODkzZGQwNyIsICJhdWQiOiBbImV6YUYyVVZETGlhayJdLCAiaWF0IjogMTYwNjY3MDMyOSwgImV4cCI6IDE2MDY2NzM5MjksICJhdF9oYXNoIjogIkVJQk9oVkx4eXNUZXczeTdaSzZiM2ciLCAibm9uY2UiOiAiMGUwdWI5bnQzYWF1In0.D74Ek9v_97UJum0rmpb_9vpKkdPOE4sCspuGHA58nbgU65jYsovp70t6c44XBBZqY9J93TvZqFbSKVGPnfJlUFhyegmq8IUUcC-L-DrLULU4IUnu-M4BDjsuXrmc4lZdNZFRj3ZBheuiRDMEKEalOiSzgou4e8kQW7vIScnnmuI","token_type":"Bearer"}
#[post("/oidc/token", data = "<token>")]
fn oidc_token<'a>(
    auth: BasicAuthorization,
    token: Option<Form<TokenQuery>>,
) -> Result<String, String> {
    let token = token.unwrap(); // FIXME
    Ok(String::from(auth.username))
}

fn main() {
    dotenv::dotenv().unwrap();

    #[cfg(feature = "discord_bot")]
    {
        ctrlc::set_handler(model::discord_login_destroy).expect("cannot set ctrl+c handler");
        model::discord_login_init();
    }

    // let db_conn = DbConn::fairing();

    /*let config = {
        let config = ConfigBuilder::new(Environment::active().unwrap());
        config.config.unwrap()
    };*/

    let rocket = rocket::ignite()
        .attach(DbConn::fairing())
        .mount(
            "/",
            routes![
                static_file,
                hello,
                user_view,
                user_view_me,
                login,
                login_post,
                logout,
                oidc_authorize,
                oidc_token,
            ],
        )
        .launch();
}

include!(concat!(env!("OUT_DIR"), "/templates.rs"));
