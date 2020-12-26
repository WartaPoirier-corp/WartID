use std::env;

#[derive(Debug)]
pub struct Config {
    pub http_base_url: &'static str,
}

impl Config {
    pub fn load() -> Self {
        Self {
            http_base_url: {
                let mut var = env::var("HTTP_BASE_URL").unwrap();
                if !var.ends_with('/') {
                    var.push('/')
                }
                Box::leak(var.into_boxed_str())
            },
        }
    }
}
