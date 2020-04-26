use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use reqwest::header;
use serde::de::DeserializeOwned;

use crate::metas::meta::OAUTH_TOKEN_ENTRY;
use crate::private_model::{PSNResponseError, Tokens};
use crate::psn::PSNError;
use crate::traits::{EncodeUrl, PSNRequest};

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

    pub fn add_refresh_token(&mut self, refresh_token: String) -> &mut Self {
        if !refresh_token.is_empty() {
            self.refresh_token = Some(refresh_token);
        }
        self
    }

    pub fn get_refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    pub fn add_npsso(&mut self, npsso: String) -> &mut Self {
        if !npsso.is_empty() {
            self.npsso = Some(npsso);
        }
        self
    }

    /// `region` is the psn server region setting. default is `hk`(Hang Kong). You can change to `us`(USA),`jp`(Japan), etc
    pub fn set_region(&mut self, region: String) -> &mut Self {
        self.region = region;
        self
    }

    /// default language is English.
    pub fn set_lang(&mut self, lang: String) -> &mut Self {
        self.language = lang;
        self
    }

    /// This is your login in account's online id. default is "".
    /// This field is used to generate new message thread.
    pub fn set_self_online_id(&mut self, id: String) -> &mut Self {
        self.self_online_id = id;
        self
    }

    pub fn set_access_token(&mut self, access_token: Option<String>) -> &mut Self {
        self.access_token = access_token;
        self
    }

    pub fn set_refresh_token(&mut self, refresh_token: Option<String>) -> &mut Self {
        self.refresh_token = refresh_token;
        self
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
            .expect("refresh_token can't be None when npsso code is not working")
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

impl PSNRequest for PSNInner {
    type Client = reqwest::Client;
    type Error = PSNError;

    fn gen_access_and_refresh(
        &mut self,
        client: Self::Client,
    ) -> PSNFuture<Result<(), Self::Error>> {
        Box::pin(async move {
            let npsso = self.npsso().ok_or(PSNError::AuthenticationFail)?;

            let string_body = serde_urlencoded::to_string(&Self::oauth_token_encode())
                .expect("Failed to parse string body for first authentication");

            let tokens = client
                .post(OAUTH_TOKEN_ENTRY)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("Cookie", format!("npsso={}", npsso))
                .body(string_body)
                .send()
                .await?
                .json::<Tokens>()
                .await?;

            if tokens.access_token.is_none() || tokens.refresh_token.is_none() {
                return Err(PSNError::AuthenticationFail);
            }

            self.set_access_token(tokens.access_token)
                .set_refresh_token(tokens.refresh_token)
                .set_refresh();

            Ok(())
        })
    }

    fn gen_access_from_refresh(
        &mut self,
        client: Self::Client,
    ) -> PSNFuture<Result<(), Self::Error>> {
        Box::pin(async move {
            let string_body = serde_urlencoded::to_string(&self.oauth_token_refresh_encode())
                .expect("Failed to parse string body for second time authentication");

            let tokens = client
                .post(OAUTH_TOKEN_ENTRY)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(string_body)
                .send()
                .await?
                .json::<Tokens>()
                .await?;

            if tokens.access_token.is_none() {
                return Err(PSNError::AuthenticationFail);
            }

            self.set_access_token(tokens.access_token).set_refresh();

            Ok(())
        })
    }

    fn get_by_url_encode<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        url: &'st str,
    ) -> PSNFuture<'se, Result<T, Self::Error>> {
        Box::pin(
            // The access_token is used as bearer token and content type header need to be application/json.
            async move {
                let req = match self.access_token() {
                    Some(token) => client
                        .get(url)
                        // The access_token is used as bearer token and content type header need to be application/json.
                        .header(header::AUTHORIZATION, format!("Bearer {}", token))
                        .header(header::CONTENT_TYPE, "application/json"),
                    // there are api endpoints that don't need access_token to access so we only add bearer token when we have it.
                    None => client
                        .get(url)
                        .header(header::CONTENT_TYPE, "application/json"),
                };

                let res = req.send().await?;

                if res.status() != 200 {
                    let e = res.json::<PSNResponseError>().await?;
                    Err(PSNError::FromPSN(e.error.message))
                } else {
                    let res = res.json().await?;
                    Ok(res)
                }
            },
        )
    }

    fn del_by_url_encode<'se, 'st: 'se>(
        &'se self,
        client: &'se Self::Client,
        url: &'st str,
    ) -> PSNFuture<'se, Result<(), Self::Error>> {
        Box::pin(async move {
            let res = client
                .delete(url)
                .header(
                    header::AUTHORIZATION,
                    format!(
                        "Bearer {}",
                        self.access_token().expect("access_token is None")
                    ),
                )
                .send()
                .await?;

            if res.status() != 204 {
                let e = res.json::<PSNResponseError>().await?;
                Err(PSNError::FromPSN(e.error.message))
            } else {
                Ok(())
            }
        })
    }

    fn post_by_multipart<'se, 'st: 'se>(
        &'se self,
        client: &'se Self::Client,
        boundary: &'st str,
        url: &'st str,
        body: Vec<u8>,
    ) -> PSNFuture<'se, Result<(), Self::Error>> {
        Box::pin(
            // The access_token is used as bearer token and content type header need to be multipart/form-data.
            async move {
                let res = client
                    .post(url)
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .header(
                        header::AUTHORIZATION,
                        format!(
                            "Bearer {}",
                            self.access_token().expect("access_token is None")
                        ),
                    )
                    .body(body)
                    .send()
                    .await?;

                if res.status() != 200 {
                    let e = res.json::<PSNResponseError>().await?;
                    Err(PSNError::FromPSN(e.error.message))
                } else {
                    Ok(())
                }
            },
        )
    }

    fn read_path(path: &str) -> PSNFuture<Result<Vec<u8>, Self::Error>> {
        Box::pin(async move { tokio::fs::read(path).await.map_err(PSNError::FromStd) })
    }
}

/// type alias to stop clippy from complaining
pub type PSNFuture<'s, T> = Pin<Box<dyn Future<Output = T> + Send + 's>>;
