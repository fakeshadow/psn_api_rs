use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use crate::traits::EncodeUrl;

#[derive(Debug)]
pub struct PSNInner {
    region: String,
    self_online_id: String,
    access_token: Option<String>,
    npsso: Option<String>,
    refresh_token: Option<String>,
    last_refresh_at: Option<Instant>,
    language: String,
}

impl Default for PSNInner {
    fn default() -> PSNInner {
        PSNInner {
            region: "hk".to_owned(),
            self_online_id: "".to_owned(),
            access_token: None,
            npsso: None,
            refresh_token: None,
            last_refresh_at: None,
            language: "en".to_owned(),
        }
    }
}

impl PSNInner {
    pub fn new() -> Self {
        PSNInner::default()
    }

    pub fn add_refresh_token(&mut self, refresh_token: String) {
        if !refresh_token.is_empty() {
            self.refresh_token = Some(refresh_token);
        }
    }

    pub fn get_refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    pub fn add_npsso(&mut self, npsso: String) {
        if !npsso.is_empty() {
            self.npsso = Some(npsso);
        }
    }

    /// `region` is the psn server region setting. default is `hk`(Hang Kong). You can change to `us`(USA),`jp`(Japan), etc
    pub fn set_region(&mut self, region: String) {
        self.region = region;
    }

    /// default language is English.
    pub fn set_lang(&mut self, lang: String) {
        self.language = lang;
    }

    /// This is your login in account's online id. default is "".
    /// This field is used to generate new message thread.
    pub fn set_self_online_id(&mut self, id: String) {
        self.self_online_id = id;
    }

    pub fn set_access_token(&mut self, access_token: Option<String>) {
        self.access_token = access_token;
    }

    pub fn set_refresh_token(&mut self, refresh_token: Option<String>) {
        self.refresh_token = refresh_token;
    }

    /// set refresh time to now.
    pub fn set_refresh(&mut self) {
        self.last_refresh_at = Some(Instant::now());
    }

    /// check if it's about time the access_token expires.
    pub fn should_refresh(&self) -> bool {
        if let Some(i) = self.last_refresh_at {
            let now = Instant::now();
            if now > i {
                return Instant::now().duration_since(i) > Duration::from_secs(3000);
            }
        }
        false
    }
}

impl EncodeUrl for PSNInner {
    fn npsso(&self) -> Option<&str> {
        self.npsso.as_deref()
    }

    fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    fn refresh_token(&self) -> &str {
        self.refresh_token
            .as_deref()
            .expect("refresh_token is None")
    }

    fn region(&self) -> &str {
        self.region.as_str()
    }

    fn self_online_id(&self) -> &str {
        &self.self_online_id
    }

    fn language(&self) -> &str {
        self.language.as_str()
    }
}

/// type alias to stop clippy from complaining
pub type PSNFuture<'s, T> = Pin<Box<dyn Future<Output = T> + Send + 's>>;
