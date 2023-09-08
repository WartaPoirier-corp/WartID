use std::collections::HashSet;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OAuth2Scope {
    /// Grants absolutely nothing special, just here to support clients who ask for it
    Basic,

    /// Requires the user to have an email address linked to their account
    Email,

    /// Requires nothing, but allows the login form to authenticate us as fake accounts anyone for
    /// testing purposes
    Dev,
}

impl FromStr for OAuth2Scope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Self::Basic),
            "email" => Ok(Self::Email),
            "dev" => Ok(Self::Dev),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for OAuth2Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Basic => "basic",
            Self::Email => "email",
            Self::Dev => "dev",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct OAuth2Scopes(HashSet<OAuth2Scope>);

impl OAuth2Scopes {
    pub fn contains(&self, scope: OAuth2Scope) -> bool {
        self.0.contains(&scope)
    }
}

impl FromStr for OAuth2Scopes {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(OAuth2Scopes(
            s.split_ascii_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl std::fmt::Display for OAuth2Scopes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        for (idx, scope) in self.0.iter().enumerate() {
            if idx != 0 {
                f.write_char(' ')?;
            }
            std::fmt::Display::fmt(scope, f)?;
        }

        Ok(())
    }
}

impl<'de> serde::Deserialize<'de> for OAuth2Scopes {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let str = <&str as serde::Deserialize>::deserialize(deserializer)?;
        str.parse()
            .map_err(|_| D::Error::custom("cannot parse scopes"))
    }
}

impl serde::Serialize for OAuth2Scopes {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{self}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let scopes = OAuth2Scopes({
            let mut set: HashSet<OAuth2Scope> = Default::default();
            set.insert(OAuth2Scope::Basic);
            set.insert(OAuth2Scope::Email);
            set
        });

        let scopes_json = serde_json::to_string(&scopes).unwrap();

        assert!(&scopes_json == "\"basic email\"" || &scopes_json == "\"email basic\"");
    }

    #[test]
    fn deserialize() {
        let scopes: OAuth2Scopes = serde_json::from_str("\"basic email\"").unwrap();

        assert_eq!(scopes.0.len(), 2);
        assert!(scopes.0.contains(&OAuth2Scope::Basic));
        assert!(scopes.0.contains(&OAuth2Scope::Email));
    }
}
