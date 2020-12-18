use std::convert::TryInto;
use std::fmt::Write;

use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use serde::export::Formatter;
use uuid::Uuid;

use super::*;

struct OAuthSecret([u8; 64]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OAuthSecretParseError {
    UnexpectedAscii,
    InvalidLength,
}

impl std::fmt::Display for OAuthSecretParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnexpectedAscii => "oauth_secret was stored in invalid format: expected ascii",
            Self::InvalidLength => {
                "oauth_secret was stored in invalid format: expected length of 64"
            }
        })
    }
}

impl std::error::Error for OAuthSecretParseError {}

impl<ST, DB: Backend> FromSql<ST, DB> for OAuthSecret
where
    *const str: FromSql<ST, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> diesel::deserialize::Result<Self> {
        let string = String::from_sql(bytes)?;

        match (string.len(), string.is_ascii()) {
            (64, true) => {
                let bytes: [u8; 64] = string.into_bytes().try_into().expect("incoherent match");
                Ok(Self(bytes))
            }
            (64, false) => Err(Box::new(OAuthSecretParseError::UnexpectedAscii)),
            (_, _) => Err(Box::new(OAuthSecretParseError::InvalidLength)),
        }
    }
}

pub struct UserApp {
    id: Uuid,
    name: String,
    oauth_secret: Option<OAuthSecret>,
    description: Option<String>,
    hidden: bool,
}

pub struct NewUserApp {
    name: String,
    hidden: bool,
}
