use super::config::Config;
use super::ratelimit::RateLimits;
use axum::extract::FromRef;
use hmac::{Hmac, Mac};
use ipnet::IpNet;
use leptos::config::LeptosOptions;
use sea_orm::DatabaseConnection;
use sha2::Sha256;
use std::net::IpAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub leptos: LeptosOptions,
    pub shared: Arc<Shared>,
}

pub struct Shared {
    pub secret: Vec<u8>,
    pub trusted_proxies: Vec<IpNet>,
    pub limits: RateLimits,
    pub secure_cookies: bool,
}

impl AppState {
    pub fn new(db: DatabaseConnection, leptos: LeptosOptions, config: &Config, secret: Vec<u8>) -> Self {
        Self {
            db,
            leptos,
            shared: Arc::new(Shared {
                secret,
                trusted_proxies: config.trusted_proxies(),
                limits: RateLimits::new(&config.limits),
                secure_cookies: config.secure_cookies,
            }),
        }
    }

    pub fn limits(&self) -> &RateLimits {
        &self.shared.limits
    }

    /// A stable pseudonym for a client address. Lets moderators see which
    /// reports came from one place without the database holding addresses.
    pub fn source_hash(&self, key: IpAddr) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.shared.secret).expect("HMAC takes any key length");
        mac.update(key.to_string().as_bytes());
        hex::encode(&mac.finalize().into_bytes()[..12])
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos.clone()
    }
}
