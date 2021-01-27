use std::env;
#[cfg(feature = "discord_bot")]
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config {
    pub http_base_url: &'static str,

    #[cfg(feature = "discord_bot")]
    pub discord_key_file: &'static Path,
}

impl Config {
    pub fn load() -> Self {
        Self {
            http_base_url: {
                let mut var = env::var("HTTP_BASE_URL").expect("no HTTP_BASE_URL set");
                if !var.ends_with('/') {
                    var.push('/')
                }
                Box::leak(var.into_boxed_str())
            },
            #[cfg(feature = "discord_bot")]
            discord_key_file: {
                let path: PathBuf = env::var("DISCORD_KEY_FILE")
                    .expect("no DISCORD_KEY_FILE set")
                    .into();

                Box::leak(path.into_boxed_path())
            },
        }
    }
}
