//! # A simple PSN API wrapper.
//! It uses an async http client(hyper::Client in this case) to communicate wih the official PSN API.
//!
//! Some basics:
//! The crate use `npsso` code to login in to PSN Network and get a pair of `access_token` and `refresh_token` in response.
//! [How to obtain uuid and two_step tokens](https://tusticles.com/psn-php/first_login.html)
//! The `access_token` last about an hour before expire and it's needed to call most other PSN APIs(The PSN store API doesn't need any token to access though).
//! The `refresh_token` last much longer and it's used to generate a new access_token after/before it is expired.
//! * Some thing to note:
//! There is a rate limiter for the official PSN API so better not make lots of calls in short time.
//!
//! # Example:
//!```no_run
//!use psn_api_rs::{psn::PSN, traits::PSNRequest, models::PSNUser};
//!
//!#[tokio::main]
//!async fn main() -> std::io::Result<()> {
//! let refresh_token = String::from("your refresh token");
//!    let npsso = String::from("your npsso");
//!
//!    // construct a PSN object,add credentials and call auth to generate tokens.
//!    let mut psn: PSN = PSN::new()
//!            .set_region("us".to_owned()) // <- set to a psn region server suit your case. you can leave it as default which is hk
//!            .set_lang("en".to_owned()) // <- set to a language you want the response to be. default is en
//!            .set_self_online_id(String::from("Your Login account PSN online_id")) // <- this is used to generate new message thread.
//!                                                                    // safe to leave unset if you don't need to send any PSN message.
//!            .add_refresh_token(refresh_token) // <- If refresh_token is provided then it's safe to ignore add_npsso and call .auth() directly.
//!            .add_npsso(npsso) // <- npsso is used only when refresh_token is not working or not provided.
//!            .auth()
//!            .await
//!            .unwrap_or_else(|e| panic!("{:?}", e));
//!
//!    println!(
//!        "Authentication Success! These are your token info from PSN network: \r\n{:#?} ",
//!        psn
//!    );
//!
//!    let user: PSNUser = psn
//!            .add_online_id("Hakoom".to_owned())
//!            .get_profile()
//!            .await
//!            .unwrap_or_else(|e| panic!("{:?}", e));
//!
//!    println!(
//!        "Example finished. Got user info : \r\n{:#?}",
//!        user
//!    );
//!
//!    Ok(())
//!    // psn struct is dropped at this point so it's better to store your access_token and refresh_token here to make them reusable.
//!}
//!```

#[macro_use]
extern crate serde_derive;

pub mod metas;
pub mod models;
pub mod traits;
pub mod types;

mod private_model;

#[cfg(feature = "default")]
pub mod psn {
    use crossbeam_queue::SegQueue;
    use derive_more::Display;
    use reqwest::{header, Client, ClientBuilder, Error, Proxy};
    use serde::de::DeserializeOwned;
    use tang_rs::{Builder, Manager, ManagerFuture, Pool, PoolRef};

    use crate::metas::meta::OAUTH_TOKEN_ENTRY;
    use crate::private_model::{PSNResponseError, Tokens};
    use crate::traits::{EncodeUrl, PSNRequest};
    use crate::types::{PSNFuture, PSNInner};

    #[derive(Debug)]
    pub struct PSN {
        inner: PSNInner,
        client: Client,
        proxy_pool: Option<Pool<ProxyPoolManager>>,
    }

    /// You can override `PSNRequest` trait to impl your own error type.
    #[derive(Debug, Display)]
    pub enum PSNError {
        #[display(fmt = "No http client is available and/or new client can't be made.")]
        NoClient,
        #[display(fmt = "Failed to login in to PSN")]
        AuthenticationFail,
        #[display(fmt = "Request is timeout")]
        TimeOut,
        #[display(fmt = "Error from Reqwest: {}", _0)]
        FromReqwest(Error),
        #[display(fmt = "Error from PSN response: {}", _0)]
        FromPSN(String),
        #[display(fmt = "Error from Local: {}", _0)]
        FromStd(std::io::Error),
    }

    pub struct ProxyPoolManager {
        proxies: SegQueue<(String, Option<String>, Option<String>)>,
        marker: &'static str,
    }

    impl ProxyPoolManager {
        fn new() -> Self {
            ProxyPoolManager {
                proxies: SegQueue::new(),
                marker: "www.google.com",
            }
        }

        fn add_proxy(&self, address: &str, username: Option<&str>, password: Option<&str>) {
            self.proxies.push((
                address.into(),
                username.map(Into::into),
                password.map(Into::into),
            ));
        }
    }

    impl Manager for ProxyPoolManager {
        type Connection = Client;
        type Error = PSNError;

        fn connect(&self) -> ManagerFuture<'_, Result<Self::Connection, Self::Error>> {
            Box::pin(async move {
                let (address, username, password) =
                    self.proxies.pop().map_err(|_| PSNError::NoClient)?;
                let proxy = match username {
                    Some(username) => Proxy::all(&address)
                        .map(|p| p.basic_auth(&username, password.as_deref().unwrap_or(""))),
                    None => Proxy::all(&address),
                };

                Client::builder()
                    .proxy(proxy.map_err(|_| PSNError::NoClient)?)
                    .build()
                    .map_err(|_| PSNError::NoClient)
            })
        }

        fn is_valid<'a>(
            &'a self,
            conn: &'a mut Self::Connection,
        ) -> ManagerFuture<'a, Result<(), Self::Error>> {
            Box::pin(async move {
                let _ = conn.get(self.marker).send().await?;
                Ok(())
            })
        }

        fn is_closed(&self, _conn: &mut Self::Connection) -> bool {
            false
        }
    }

    impl From<Error> for PSNError {
        fn from(e: Error) -> Self {
            PSNError::FromReqwest(e)
        }
    }

    impl From<tokio::time::Elapsed> for PSNError {
        fn from(_: tokio::time::Elapsed) -> Self {
            PSNError::TimeOut
        }
    }

    impl Default for PSN {
        fn default() -> Self {
            PSN {
                inner: Default::default(),
                client: ClientBuilder::new()
                    .build()
                    .expect("Failed to build http client"),
                proxy_pool: None,
            }
        }
    }

    impl PSN {
        pub fn new() -> Self {
            PSN::default()
        }

        pub fn add_refresh_token(mut self, refresh_token: String) -> Self {
            self.inner.add_refresh_token(refresh_token);
            self
        }

        pub fn get_refresh_token(&self) -> Option<&str> {
            self.inner.get_refresh_token()
        }

        pub fn add_npsso(mut self, npsso: String) -> Self {
            self.inner.add_npsso(npsso);
            self
        }

        /// `region` is the psn server region setting. default is `hk`(Hang Kong). You can change to `us`(USA),`jp`(Japan), etc
        pub fn set_region(mut self, region: String) -> Self {
            self.inner.set_region(region);
            self
        }

        /// default language is English.
        pub fn set_lang(mut self, lang: String) -> Self {
            self.inner.set_lang(lang);
            self
        }

        /// This is your login in account's online id. default is "".
        /// This field is used to generate new message thread.
        pub fn set_self_online_id(mut self, id: String) -> Self {
            self.inner.set_self_online_id(id);
            self
        }

        pub fn set_access_token(&mut self, access_token: Option<String>) -> &mut Self {
            self.inner.set_access_token(access_token);
            self
        }

        pub fn set_refresh_token(&mut self, refresh_token: Option<String>) -> &mut Self {
            self.inner.set_refresh_token(refresh_token);
            self
        }

        /// set refresh time to now.
        pub fn set_refresh(&mut self) {
            self.inner.set_refresh();
        }

        /// check if it's about time the access_token expires.
        pub fn should_refresh(&self) -> bool {
            self.inner.should_refresh()
        }

        ///Add http proxy pool to combat PSN rate limiter.
        ///# Example:
        ///```no_run
        ///use psn_api_rs::{psn::PSN, traits::PSNRequest};
        ///
        ///#[tokio::main]
        ///async fn main() -> std::io::Result<()> {
        ///    let refresh_token = String::from("your refresh token");
        ///
        ///    let mut psn: PSN = PSN::new()
        ///            .set_region("us".to_owned())
        ///            .set_lang("en".to_owned())
        ///            .set_self_online_id(String::from("Your Login account PSN online_id"))
        ///            .add_refresh_token(refresh_token);
        ///
        ///    // the max proxy pool size is determined by the first proxies vector's length passed to PSN object(upper limit pool size is u8).
        ///    // You can pass more proxies on the fly to PSN but once you hit the max pool size
        ///    // all additional proxies become backup and can only be activated when an active proxy is dropped(connection broken for example)
        ///    let proxies = vec![
        ///        // ("address", Some(username), Some(password)),
        ///        ("http://abc.com", None, None),
        ///        ("https://test:1234", None, None),
        ///        ("http://abc.com", Some("user"), Some("pass")),
        ///    ];
        ///
        ///    psn.add_proxy(proxies).await;
        ///    psn = psn.auth().await.unwrap_or_else(|e| panic!("{:?}", e));
        ///
        ///    Ok(())
        ///}
        ///```
        pub async fn add_proxy(&mut self, proxies: Vec<(&str, Option<&str>, Option<&str>)>) {
            if let Some(pool) = &self.proxy_pool {
                for (address, username, password) in proxies.into_iter() {
                    pool.get_manager().add_proxy(address, username, password);
                }
                return;
            }

            let mgr = ProxyPoolManager::new();
            let size = proxies.len() as u8;
            for (address, username, password) in proxies.into_iter() {
                mgr.add_proxy(address, username, password);
            }

            let pool = Builder::new()
                .always_check(false)
                .idle_timeout(None)
                .max_lifetime(None)
                .min_idle(size)
                .max_size(size)
                .build(mgr)
                .await
                .expect("Failed to build proxy pool");

            self.proxy_pool = Some(pool);
        }

        pub async fn get_proxy_cli(
            &self,
        ) -> Result<Option<PoolRef<'_, ProxyPoolManager>>, PSNError> {
            let fut = match self.proxy_pool.as_ref() {
                Some(pool) => pool.get(),
                None => return Ok(None),
            };
            let conn = fut.await?;
            println!("got connection");
            Ok(Some(conn))
        }

        pub fn clients_state(&self) -> u8 {
            self.proxy_pool
                .as_ref()
                .map(|pool| pool.state().idle_connections)
                .unwrap_or(0)
        }
    }

    impl EncodeUrl for PSN {
        fn npsso(&self) -> Option<&str> {
            self.inner.npsso()
        }

        fn access_token(&self) -> Option<&str> {
            self.inner.access_token()
        }

        fn refresh_token(&self) -> &str {
            self.inner.refresh_token()
        }

        fn region(&self) -> &str {
            self.inner.region()
        }

        fn self_online_id(&self) -> &str {
            self.inner.self_online_id()
        }

        fn language(&self) -> &str {
            self.inner.language()
        }
    }

    impl PSNRequest for PSN {
        type Error = PSNError;

        fn gen_access_and_refresh(&mut self) -> PSNFuture<Result<(), Self::Error>> {
            Box::pin(async move {
                let npsso = self.inner.npsso().ok_or(PSNError::AuthenticationFail)?;

                let string_body = serde_urlencoded::to_string(&PSNInner::oauth_token_encode())
                    .expect("Failed to parse string body for first authentication");

                let tokens = self
                    .client
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

        fn gen_access_from_refresh(&mut self) -> PSNFuture<Result<(), Self::Error>> {
            Box::pin(async move {
                let string_body = serde_urlencoded::to_string(&self.oauth_token_refresh_encode())
                    .expect("Failed to parse string body for second time authentication");

                let tokens = self
                    .client
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

        fn get_by_url_encode<'s, 'u: 's, T: DeserializeOwned + 'static>(
            &'s self,
            url: &'u str,
        ) -> PSNFuture<'s, Result<T, Self::Error>> {
            Box::pin(
                // The access_token is used as bearer token and content type header need to be application/json.
                async move {
                    let pool_ref = self.get_proxy_cli().await?;

                    let client = match &pool_ref {
                        Some(cli) => &**cli,
                        None => &self.client,
                    };

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
                        // ToDo: conditional drop bad Client because proxy failure
                        let e = res.json::<PSNResponseError>().await?;
                        Err(PSNError::FromPSN(e.error.message))
                    } else {
                        let res = res.json().await?;
                        Ok(res)
                    }
                },
            )
        }

        fn del_by_url_encode<'s, 'u: 's>(
            &'s self,
            url: &'u str,
        ) -> PSNFuture<'s, Result<(), Self::Error>> {
            Box::pin(async move {
                let pool_ref = self.get_proxy_cli().await?;

                let client = match pool_ref.as_ref() {
                    Some(cli) => &**cli,
                    None => &self.client,
                };

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
            boundary: &'st str,
            url: &'st str,
            body: Vec<u8>,
        ) -> PSNFuture<'se, Result<(), Self::Error>> {
            Box::pin(
                // The access_token is used as bearer token and content type header need to be multipart/form-data.
                async move {
                    let pool_ref = self.get_proxy_cli().await?;

                    let client = match pool_ref.as_ref() {
                        Some(cli) => &**cli,
                        None => &self.client,
                    };

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
}
