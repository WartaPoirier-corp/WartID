use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};

#[cfg(feature = "discord_bot")]
pub use discord_login::{destroy as discord_login_destroy, init as discord_login_init};

use crate::id::Id;
use crate::schema::users;

use super::*;

pub type UserId = Id<User>;

#[derive(Clone, Debug, Queryable)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password: Option<String>,
    pub email: Option<String>,
    pub discord_id: Option<i64>,
}

impl User {
    fn insert(db: crate::DbConnection, new: NewUser) -> WartIDResult<User> {
        use crate::schema::users::dsl::*;

        let user: User = diesel::insert_into(users).values(new).get_result(db)?;

        Ok(user)
    }

    pub fn find_all(db: crate::DbConnection, include_guests: bool) -> WartIDResult<Vec<User>> {
        use crate::schema::users::dsl::*;

        if include_guests {
            users.load::<Self>(db)
        } else {
            users.filter(discord_id.is_not_null()).load::<Self>(db)
        }
        .map_err(Into::into)
    }

    pub fn find_by_id(db: crate::DbConnection, l_id: UserId) -> WartIDResult<Option<User>> {
        use crate::schema::users::dsl::*;

        Ok(users
            .filter(id.eq(&l_id))
            .limit(1)
            .load::<Self>(db)?
            .into_iter()
            .next())
    }

    #[cfg(feature = "discord_bot")]
    fn find_or_create_by_discord_id(
        db: crate::DbConnection,
        l_discord_id: u64,
        l_discord_name: String,
    ) -> WartIDResult<User> {
        use crate::schema::users::dsl::*;

        // TODO handle not found
        if let Ok(user) = users
            .filter(discord_id.eq(unsafe { std::mem::transmute::<u64, i64>(l_discord_id) }))
            .first::<User>(db)
        {
            return Ok(user);
        }

        let new_user = NewUser {
            username: l_discord_name,
            password: None,
            email: None,
            discord_id: Some(unsafe { std::mem::transmute(l_discord_id) }),
        };

        User::insert(db, new_user)
    }

    pub fn attempt_login(
        db: crate::DbConnection,
        l_username: &str,
        l_password: &str,
    ) -> WartIDResult<Option<User>> {
        use crate::schema::users::dsl::*;

        // Attempt JWT / discord login
        #[cfg(feature = "discord_bot")]
        if l_username.is_empty() {
            let (l_discord_id, l_discord_name) = discord_login::verify_jwt(l_password)
                .map_err(|err| WartIDError::InvalidCredentials(err))?;

            return User::find_or_create_by_discord_id(db, l_discord_id, l_discord_name).map(Some);
        }

        match users
            .filter(username.eq(l_username))
            .first::<User>(db)
            .optional()
            .map_err(Into::into)
        {
            Ok(Some(user)) => {
                if let Some(db_password) = &user.password {
                    if bcrypt::verify(l_password, &db_password).expect("bcrypt cannot verify") {
                        return Ok(Some(user));
                    }
                }

                Err(WartIDError::InvalidCredentials(String::from(
                    "invalid password",
                )))
            }
            other => other,
        }
    }

    pub fn update_username(
        db: crate::DbConnection,
        user_id: UserId,
        new_username: &str,
    ) -> WartIDResult<User> {
        use crate::schema::users::dsl::*;

        diesel::update(users)
            .filter(id.eq(user_id))
            .set(username.eq(new_username))
            .get_result(db)
            .map_err(Into::into)
    }

    pub fn update_email(
        db: crate::DbConnection,
        user_id: UserId,
        new_email: &str,
    ) -> WartIDResult<User> {
        use crate::schema::users::dsl::*;

        diesel::update(users)
            .filter(id.eq(user_id))
            .set(email.eq(new_email))
            .get_result(db)
            .map_err(Into::into)
    }

    pub fn update_password(
        db: crate::DbConnection,
        user_id: UserId,
        new_password: &str,
    ) -> WartIDResult<User> {
        use crate::schema::users::dsl::*;

        let new_password = if new_password.is_empty() {
            None
        } else {
            Some(
                {
                    let start = std::time::Instant::now();

                    let l_password = bcrypt::hash(new_password, bcrypt::DEFAULT_COST);

                    let elapsed = start.elapsed();
                    log::debug!(target: file!(), "generated password in {:?}", elapsed);

                    l_password
                }
                .map_err(|e| WartIDError::Any(Box::new(e)))?,
            )
        };

        diesel::update(users)
            .filter(id.eq(user_id))
            .set(password.eq(new_password))
            .get_result(db)
            .map_err(Into::into)
    }
}

#[derive(Insertable)]
#[table_name = "users"]
struct NewUser {
    pub username: String,
    pub password: Option<String>,
    pub email: Option<String>,
    pub discord_id: Option<i64>,
}

#[cfg(feature = "discord_bot")]
mod discord_login {
    use jsonwebtoken::{DecodingKey, TokenData, Validation};

    lazy_static::lazy_static! {
        static ref KEY: DecodingKey = {
            use rand::Rng;

            let gen: &'static _ = Box::<[u8; 32]>::leak(Box::new(rand::rngs::OsRng.gen()));

            std::fs::write(crate::CONFIG.discord_key_file, gen).expect("cannot write key file");

            DecodingKey::from_secret(gen)
        };
    }

    pub fn init() {
        let _ = &*KEY;
    }

    pub fn destroy() {
        std::fs::remove_file(crate::CONFIG.discord_key_file).expect("cannot remove key file");
        println!("Removed discord bot JWT key file");
        std::process::exit(0)
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Claims {
        exp: i64,

        /// Subject (Discord user ID)
        sub: u64,

        name: String,
    }

    pub fn verify_jwt(token: &str) -> Result<(u64, String), String> {
        let validation = &{
            let mut v = Validation::default();
            v.validate_exp = true;
            v
        };

        jsonwebtoken::decode(token, &*KEY, validation)
            .map_err(|err| err.to_string())
            .map(|data: TokenData<Claims>| (data.claims.sub, data.claims.name))
    }
}
