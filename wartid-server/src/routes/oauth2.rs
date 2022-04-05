use std::borrow::Cow;

use rocket::form::error::ErrorKind;
use rocket::form::{FromFormField, ValueField};
use rocket::http::uri::Origin;
use rocket::http::Status;
use rocket::outcome::{try_outcome, IntoOutcome};
use rocket::request::FromRequest;
use rocket::request::Outcome;
use rocket::serde::json::Json;
use rocket::Request;

use crate::utils::jwt::JWT;

use super::prelude::*;

impl<'r> FromFormField<'r> for OAuth2Scopes {
    fn from_value(field: ValueField<'r>) -> rocket::form::Result<'r, Self> {
        field.value.parse().map_err(|()| ErrorKind::Unknown.into())
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct AuthorizeState<Str, O2S> {
    #[serde(rename = "aud")]
    client: UserAppId,

    #[serde(rename = "sub")]
    user: UserId,

    /// Scopes granted by the user. When requesting access tokens later, the access tokens will only
    /// be able to request a subset of these scopes.
    #[serde(rename = "scopes")]
    initial_scopes: O2S,

    redirect_uri: Str,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct AccessState {
    #[serde(rename = "aud")]
    client: UserAppId,

    #[serde(rename = "sub")]
    user: UserId,

    scopes: OAuth2Scopes,
}

lazy_static::lazy_static! {
    static ref ACCESS_TOKEN_EXPIRATION: chrono::Duration = chrono::Duration::hours(1);

    static ref JWT_AUTHORIZE: JWT<AuthorizeState<&'static str, &'static OAuth2Scopes>, AuthorizeState<String, OAuth2Scopes>> = JWT::new("wartid-authorize", chrono::Duration::minutes(10));
    static ref JWT_ACCESS: JWT<AccessState, AccessState> = JWT::new("wartid-access-token", *ACCESS_TOKEN_EXPIRATION);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, rocket::FromFormField)]
pub enum AuthorizeResponseType {
    #[field(value = "code")]
    Code,
}

#[derive(FromForm, Debug)]
pub struct AuthorizeQuery<'a> {
    client_id: UserAppId,
    redirect_uri: String,
    scope: Option<OAuth2Scopes>,
    response_type: AuthorizeResponseType,
    state: Option<String>,
    nonce: Option<&'a str>,
}

macro_rules! implies {
    ($p:expr => $q:expr) => {
        !$p || $q
    };
}

#[cfg(test)]
#[test]
fn implies_test() {
    assert!(implies!(true => true));
    assert!(!implies!(true => false));
    assert!(implies!(false => true));
    assert!(implies!(false => false));
}

#[get("/oauth2/authorize?<authorize..>")]
pub async fn authorize(
    current_uri: &Origin<'_>,
    session: Option<&LoginSession>,
    db: DbConn,
    authorize: Result<AuthorizeQuery<'_>, rocket::form::error::Errors<'_>>,
) -> WartIDResult<Result<Ructe, Redirect>> {
    match authorize {
        Ok(authorize) => Ok({
            //let authorize = authorize.into_inner();

            if authorize.response_type != AuthorizeResponseType::Code {
                /* FIXME Even though this code is literally unreachable, implement correctly in case
                things change later */
                unreachable!()
            }

            if let Some(session) = session {
                let redirect_uri = &authorize.redirect_uri;

                let authorize_client_id = authorize.client_id;
                let app = db_await!(UserApp::find_by_id(db, authorize_client_id))?
                    .filter(|app| app.oauth2().is_some())
                    .ok_or(WartIDError::OAuth2Error("client not found"))?;

                if !app.is_oauth2_redirect_allowed(redirect_uri) {
                    return Err(WartIDError::OAuth2Error("redirect uri is not configured"));
                }

                let redirect_uri_short = redirect_uri
                    .split_once("//")
                    .and_then(|(_, right)| right.split_once('/'))
                    .map(|(left, _)| left)
                    .unwrap_or(redirect_uri.as_str());

                let scopes = authorize.scope.unwrap_or_default();

                let code = if implies!(scopes.contains(OAuth2Scope::Email) => session.user.email.is_some())
                {
                    Some(JWT_AUTHORIZE.encode(AuthorizeState {
                        client: app.id,
                        user: session.user.id,
                        initial_scopes: unsafe { std::mem::transmute(&scopes) },
                        // :see_no_evil: (FIXME obv)
                        redirect_uri: unsafe { std::mem::transmute(redirect_uri.as_str()) },
                    }))
                } else {
                    None
                };

                // TODO X-Frame-Options: Deny

                Ok(render!(oauth2::authorize(
                    &session.user,
                    &app,
                    redirect_uri_short,
                    redirect_uri,
                    code.as_deref(),
                    authorize.state.as_deref(),
                    &scopes
                )))
            } else {
                let uri = format!("{}{}", crate::CONFIG.http_base_url, current_uri.to_string());
                Err(Redirect::to(uri!(crate::login(Some(uri)))))
            }
        }),
        Err(err) => Err(WartIDError::InvalidForm(format!("{:?}", err))),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, rocket::FromFormField)]
enum GrantType {
    #[field(value = "authorization_code")]
    AuthorizationCode,

    #[field(value = "refresh_token")]
    RefreshToken,
}

#[derive(FromForm, Debug)]
pub struct TokenQuery<'a> {
    grant_type: GrantType,
    code: Option<&'a str>,
    refresh_token: Option<String>,
    redirect_uri: &'a str,
    scope: Option<&'a str>,

    client_id: Option<&'a str>,
    client_secret: Option<&'a str>,
}

/// Other auth methods: https://darutk.medium.com/oauth-2-0-client-authentication-4b5f929305d4
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasicAuthorization {
    username: String,
    password: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BasicAuthorization {
    type Error = String;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth = match request.headers().get("Authorization").next() {
            Some(auth) => auth,
            None => {
                return Outcome::Failure((
                    Status::Forbidden,
                    String::from("No Authorization header"),
                ));
            }
        };

        let auth = try_outcome!(auth
            .strip_prefix("Basic ")
            .ok_or_else(|| String::from("Only basic auth is supported"))
            .into_outcome(Status::BadRequest));

        let auth = base64::decode(auth).map_err(|e| e.to_string()).unwrap(); // FIXME
        let auth = std::str::from_utf8(&auth)
            .map_err(|e| e.to_string())
            .unwrap(); // FIXME

        let (username, password) = try_outcome!(auth
            .split_once(':')
            .ok_or(String::from("Bad format, missing colon"))
            .into_outcome(Status::BadRequest));

        Outcome::Success(BasicAuthorization {
            username: String::from(username),
            password: String::from(password),
        })
    }
}

enum TokenType {
    Bearer,
}

impl serde::Serialize for TokenType {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Bearer => serializer.serialize_str("Bearer"),
        }
    }
}

#[derive(serde::Serialize)]
pub struct TokenResponse {
    access_token: String,
    expires_in: u64,
    token_type: TokenType,

    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
}

pub struct BearerSession {
    user: User,
    scopes: OAuth2Scopes,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BearerSession {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let bearer = try_outcome!(request
            .headers()
            .get_one("Authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or("missing bearer authentication header")
            .into_outcome(Status::BadRequest));

        let db: crate::DbConn = request.guard().await.unwrap();

        let token_access = try_outcome!(JWT_ACCESS
            .decode(bearer)
            .map_err(|_| "cannot validate access token")
            .into_outcome(Status::Unauthorized));
        let token_access_user = token_access.user;
        let user = try_outcome!(db_await!(User::find_by_id(db, token_access_user))
            .map_err(|_| "database error")
            .into_outcome(Status::InternalServerError));
        let user = try_outcome!(user
            .ok_or("authentication successful but user not in database")
            .into_outcome(Status::InternalServerError));

        Outcome::Success(BearerSession {
            user,
            scopes: token_access.scopes,
        })
    }
}

#[post("/oauth2/token", data = "<data>")]
pub async fn token(
    db: DbConn,
    auth: Option<BasicAuthorization>,
    data: Form<TokenQuery<'_>>,
) -> Result<Json<TokenResponse>, String> {
    let (client_id, client_password) = match (auth, data.client_id, data.client_secret) {
        (Some(auth), None, None) => (auth.username.parse(), Cow::Owned(auth.password)),
        (None, Some(client_id), Some(client_password)) => {
            (client_id.parse(), Cow::Borrowed(client_password))
        }
        _ => return Err(String::from("multiple auth methods used simultaneously")),
    };

    let client_id = client_id.map_err(|_| String::from("cannot parse client uuid"))?;

    let app = match db_await!(UserApp::find_by_id(db, client_id)) {
        Ok(Some(app)) => app,
        Ok(None) => return Err(String::from("unknown client id")),
        Err(e) => return Err(format!("{:?}", e)),
    };

    if Some(client_password.as_ref()) != app.oauth2().map(|(secret, _)| secret) {
        return Err(String::from("invalid client secret"));
    }

    let (user, scopes) = match {
        let TokenQuery { grant_type, code, refresh_token, .. } = data.into_inner();
        (grant_type, code, refresh_token)
    } {
        (GrantType::AuthorizationCode, Some(code), None) => {
            let authorize = JWT_AUTHORIZE
                .decode(code)
                .map_err(|e| format!("invalid code, might have expired: {:?}", e))?; // TODO

            if authorize.client != app.id {
                return Err(String::from("invalid client"));
            }

            // TODO check redirect URI

            (authorize.user, authorize.initial_scopes)
        }
        (GrantType::RefreshToken, None, Some(refresh_token)) => {
            let session = db_await!(OAuth2Session::find_by_token(db, &refresh_token)).map_err(|e| format!("{:?}", e))?;

            match session {
                Some(session) => {
                    if session.user_apps_id != app.id {
                        return Err(String::from("Forbidden app"));
                    }

                    (session.users_id, session.initial_scopes.parse().unwrap_or_default())
                }
                None => return Err(String::from("No session found for this refresh token")), // TODO proper JSON errors
            }
        }
        _ => return Err(String::from("authorization_code grant type can only be used with ?code, and refresh_token grant type can only be used with ?refresh_token")),
    };

    let scopes2 = scopes.clone();
    let refresh_token =
        match db_await!(OAuth2Session::insert_or_refresh(db, user, app.id, &scopes2)) {
            Ok(refresh_token) => Some(refresh_token),
            Err(e) => {
                log::error!("couldn't insert refresh token: {:?}", e);
                None
            }
        };

    let access_token = JWT_ACCESS.encode(AccessState {
        user,
        client: client_id,
        scopes,
    });

    Ok(Json(TokenResponse {
        access_token,
        expires_in: ACCESS_TOKEN_EXPIRATION.num_seconds() as _,
        token_type: TokenType::Bearer,
        refresh_token,
    }))
}

#[derive(serde::Serialize)]
pub struct UserInfo {
    sub: UserId,
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
}

#[get("/oauth2/userinfo")]
pub fn userinfo(session: BearerSession) -> Json<UserInfo> {
    let BearerSession { user, scopes } = session;

    println!("scopes: {:?}", scopes);

    Json(UserInfo {
        sub: user.id,
        name: user.username,
        email: user.email.filter(|_| scopes.contains(OAuth2Scope::Email)),
    })
}
