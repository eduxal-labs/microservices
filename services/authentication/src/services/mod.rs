use crate::config::Config;
use std::sync::Arc;

pub mod authentication;

#[derive(Clone)]
pub struct Authenticator {
    config: Arc<Config>,
}

impl Authenticator {
    pub async fn new() -> Self {
        let config = Arc::new(Config::new().await);
        Self { config }
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &Config {
        &self.config
    }
}
