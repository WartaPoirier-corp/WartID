use std::marker::PhantomData;

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::*;
use serde::{de::DeserializeOwned, Serialize};

#[cfg(test)]
static mut TEST_NOW: DateTime<Utc> = chrono::MIN_DATETIME;

#[inline]
fn now() -> DateTime<Utc> {
    #[cfg(not(test))]
    return Utc::now();
    #[cfg(test)]
    return unsafe { TEST_NOW };
}

#[derive(Debug, Eq, PartialEq)]
pub enum JWTValidationError {
    InvalidSignature,
    InvalidAudience,
    Expired,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct BaseClaims<AudStr, Claims: 'static> {
    #[serde(flatten)]
    ext_claims: Claims,

    #[serde(rename = "iss")]
    issuer: AudStr,

    #[serde(rename = "iat")]
    issued_at: i64,

    #[serde(rename = "exp")]
    expiration: i64,
}

/// Utility structure to create and validate/decode Json Web Tokens
///
/// It wraps a random key initialized upon creation and is associated to a specific claims object.
pub struct JWT<ClaimsIn, ClaimsOut> {
    issuer: &'static str,
    duration: Duration,

    key_enc: EncodingKey,
    key_dec: DecodingKey<'static>,

    _phantom: PhantomData<(ClaimsIn, ClaimsOut)>,
}

impl<'de, ClaimsIn: Serialize + 'static, ClaimsOut: DeserializeOwned + 'static>
    JWT<ClaimsIn, ClaimsOut>
{
    pub fn new(issuer: &'static str, duration: Duration) -> Self {
        use rand::Rng;

        let gen: [u8; 32] = rand::rngs::OsRng.gen();

        Self {
            issuer,
            duration,
            key_enc: EncodingKey::from_secret(&gen[..]),
            key_dec: DecodingKey::from_secret(&gen[..]).into_static(),
            _phantom: PhantomData,
        }
    }

    pub fn encode(&self, ext_claims: ClaimsIn) -> String {
        let now = now();

        jsonwebtoken::encode(
            &Default::default(),
            &BaseClaims {
                ext_claims,
                issuer: self.issuer,
                issued_at: now.timestamp(),
                expiration: (now + self.duration).timestamp(),
            },
            &self.key_enc,
        )
        .unwrap()
    }

    pub fn decode(&self, token: &str) -> Result<ClaimsOut, JWTValidationError> {
        let data = jsonwebtoken::decode(
            token,
            &self.key_dec,
            &Validation {
                validate_exp: false,
                ..Validation::default()
            },
        )
        .map_err(|e| {
            println!("{:?}", e);
            JWTValidationError::InvalidSignature
        })?;

        let claims: BaseClaims<String, ClaimsOut> = data.claims;

        if claims.issuer != self.issuer {
            return Err(JWTValidationError::InvalidAudience);
        }

        if now().timestamp() > claims.expiration {
            return Err(JWTValidationError::Expired);
        }

        Ok(claims.ext_claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sets the faked time as minutes from an arbitrary reference point
    fn set_time(minutes: i64) {
        unsafe { TEST_NOW = chrono::MIN_DATETIME + Duration::minutes(minutes) }
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Claims {
        sub: String,
    }

    #[test]
    fn expiration() {
        set_time(0);

        let jwt: JWT<Claims, Claims> = JWT::new("test suite", chrono::Duration::minutes(2));

        let token = jwt.encode(Claims {
            sub: String::from("Patrice"),
        });

        {
            set_time(1);
            let decoded = jwt.decode(&token).unwrap();
            assert_eq!(decoded.sub, String::from("Patrice"));
        }

        {
            set_time(5);
            let decoded_err = jwt.decode(&token).err().unwrap();
            assert_eq!(decoded_err, JWTValidationError::Expired);
        }
    }
}
