use chrono::{Duration, Utc};
use jsonwebtoken::*;
use serde::de::DeserializeOwned;
use serde::export::PhantomData;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum JWTValidationError {
    InvalidSignature,
    InvalidAudience,
    Expired,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Claims<AudStr, Subject: 'static> {
    #[serde(rename = "sub")]
    subject: Subject,

    #[serde(rename = "aud")]
    audience: AudStr,

    #[serde(rename = "iat")]
    issued_at: i64,

    #[serde(rename = "exp")]
    expiration: i64,
}

/// Utility structure to create and validate/decode Json Web Tokens
///
/// It wraps a random key initialized upon creation and is associated to a specific claims object.
pub struct JWT<SubjectIn, SubjectOut> {
    audience: &'static str,
    duration: Duration,

    key_enc: EncodingKey,
    key_dec: DecodingKey<'static>,

    _phantom: PhantomData<(SubjectIn, SubjectOut)>,
}

impl<'de, SubjectIn: Serialize + 'static, SubjectOut: DeserializeOwned + 'static>
    JWT<SubjectIn, SubjectOut>
{
    pub fn new(audience: &'static str, duration: Duration) -> Self {
        use rand::Rng;

        let gen: [u8; 32] = rand::rngs::OsRng.gen();

        Self {
            audience,
            duration,
            key_enc: EncodingKey::from_secret(&gen[..]),
            key_dec: DecodingKey::from_secret(&gen[..]).into_static(),
            _phantom: PhantomData,
        }
    }

    pub fn encode(&self, subject: SubjectIn) -> String {
        let now = Utc::now();

        jsonwebtoken::encode(
            &Default::default(),
            &Claims {
                subject,
                audience: self.audience,
                issued_at: now.timestamp(),
                expiration: (now + self.duration).timestamp(),
            },
            &self.key_enc,
        )
        .unwrap()
    }

    pub fn decode(&self, token: &str) -> Result<SubjectOut, JWTValidationError> {
        let data = jsonwebtoken::decode(token, &self.key_dec, &Default::default())
            .map_err(|_| JWTValidationError::InvalidSignature)?;

        let claims: Claims<String, SubjectOut> = data.claims;

        if claims.audience != self.audience {
            return Err(JWTValidationError::InvalidAudience);
        }

        if Utc::now().timestamp() > claims.expiration {
            return Err(JWTValidationError::Expired);
        }

        Ok(claims.subject)
    }
}
