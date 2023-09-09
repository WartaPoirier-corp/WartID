use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_base_url")]
    pub base_url: String,

    pub discord: Option<DiscordConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub token: String,

    pub allowed_guilds: Arc<[u64]>,
}

fn deserialize_base_url<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    use serde::de::{Error, Unexpected};

    let url = String::deserialize(deserializer)?;
    if url.ends_with('/') {
        Ok(url)
    } else {
        Err(Error::invalid_value(
            Unexpected::Str(&url),
            &"a URL with a trailing slash",
        ))
    }
}
