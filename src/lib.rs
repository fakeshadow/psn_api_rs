//! # A simple PSN API wrapper.
//! It uses an async http client(hyper::Client in this case) to communicate wih the official PSN API.
//!
//! Some basics:
//! The crate use `npsso` code to login in to PSN Network and get a pair of `access_token` and `refresh_token` in response.
//! [How to obtain uuid and two_step tokens](https://tusticles.com/psn-php/first_login.html)
//! The `access_token` last about an hour before expire and it's needed to call most other PSN APIs(The PSN store API doesn't need any token to access though).
//! The `refresh_token` last much longer and it's used to generate a new access_token after/before it is expired.
//!
//! * Note:
//! There is a rate limiter for the official PSN API so better not make lots of calls in short time.
//! The proxy example (The best practice to use this libaray) shows how to make high concurrency possible and combat the rate limiter effectivly.
//!
//! # Basic Example:
//!```no_run
//!use psn_api_rs::{psn::PSN, types::PSNInner, traits::PSNRequest, models::PSNUser};
//!
//!#[tokio::main]
//!async fn main() -> std::io::Result<()> {
//!    let refresh_token = String::from("your refresh token");
//!    let npsso = String::from("your npsso");
//!
//!    let client = PSN::new_client().expect("Failed to build http client");
//!
//!    // construct a PSNInner object,add credentials and call auth to generate tokens.
//!    let mut psn_inner = PSNInner::new();
//!    psn_inner.set_region("us".to_owned()) // <- set to a psn region server suit your case. you can leave it as default which is hk
//!            .set_lang("en".to_owned()) // <- set to a language you want the response to be. default is en
//!            .set_self_online_id(String::from("Your Login account PSN online_id")) // <- this is used to generate new message thread. safe to leave unset if you don't need to send any PSN message.
//!            .add_refresh_token(refresh_token) // <- If refresh_token is provided then it's safe to ignore add_npsso and call .auth() directly.
//!            .add_npsso(npsso); // <- npsso is used only when refresh_token is not working or not provided.
//!
//!    psn_inner = psn_inner
//!            .auth()
//!            .await
//!            .unwrap_or_else(|e| panic!("{:?}", e));
//!
//!    println!(
//!        "Authentication Success! These are your info from PSN network: \r\n{:#?} ",
//!        psn_inner
//!    );
//!
//!    let user: PSNUser = psn_inner
//!            .get_profile(&client, "Hakoom")
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
    use reqwest::{Client, ClientBuilder, Error, Proxy};
    use serde::de::DeserializeOwned;
    use tang_rs::{Builder, Manager, ManagerFuture, Pool, PoolRef};

    use crate::traits::PSNRequest;
    use crate::types::PSNInner;

    #[derive(Debug)]
    pub struct PSN {
        inner: Pool<PSNInnerManager>,
        client: Client,
        proxy_pool: Option<Pool<ProxyPoolManager>>,
    }

    /// You can override `PSNRequest` trait to impl your own error type.
    #[derive(Debug, Display)]
    pub enum PSNError {
        #[display(fmt = "No http client is available and/or new client can't be made.")]
        NoClient,
        #[display(fmt = "No psn object is available")]
        NoPSNInner,
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

    pub struct PSNInnerManager {
        inner: SegQueue<PSNInner>,
    }

    impl PSNInnerManager {
        fn new() -> Self {
            PSNInnerManager {
                inner: SegQueue::new(),
            }
        }

        fn add_psn_inner(&self, psn: PSNInner) {
            self.inner.push(psn);
        }
    }

    impl Manager for PSNInnerManager {
        type Connection = PSNInner;
        type Error = PSNError;

        fn connect(&self) -> ManagerFuture<'_, Result<Self::Connection, Self::Error>> {
            Box::pin(async move { self.inner.pop().map_err(|_| PSNError::NoClient) })
        }

        fn is_valid<'a>(
            &'a self,
            _conn: &'a mut Self::Connection,
        ) -> ManagerFuture<'a, Result<(), Self::Error>> {
            // ToDo: check should_refresh here.
            unimplemented!()
        }

        fn is_closed(&self, _conn: &mut Self::Connection) -> bool {
            false
        }
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

    impl PSN {
        /// A shortcut for building a temporary http client
        pub fn new_client() -> Result<Client, PSNError> {
            ClientBuilder::new().build().map_err(|_| PSNError::NoClient)
        }

        /// Accept multiple PSNInner and use a connection pool to use them concurrently.
        pub async fn new(psn_inner: Vec<PSNInner>) -> Self {
            let mgr = PSNInnerManager::new();

            let size = psn_inner.len() as u8;

            for inner in psn_inner.into_iter() {
                mgr.add_psn_inner(inner);
            }

            let inner_pool = Builder::new()
                .always_check(false)
                .idle_timeout(None)
                .max_lifetime(None)
                .min_idle(size)
                .max_size(size)
                .build(mgr)
                .await
                .expect("Failed to build proxy pool");

            PSN {
                inner: inner_pool,
                client: Self::new_client().expect("Failed to build http client"),
                proxy_pool: None,
            }
        }

        /// Add http proxy pool to combat PSN rate limiter. This is not required.
        ///# Example:
        ///```no_run
        ///use psn_api_rs::{psn::PSN, types::PSNInner, traits::PSNRequest};
        ///
        ///#[tokio::main]
        ///async fn main() -> std::io::Result<()> {
        ///    let psn_inner = PSNInner::new()
        ///            .add_refresh_token("refresh_token".into())
        ///            .add_npsso("npsso".into())
        ///            .auth()
        ///            .await
        ///            .expect("Authentication failed");
        ///
        ///    let proxies = vec![
        ///        // ("address", Some(username), Some(password)),
        ///        ("http://abc.com", None, None),
        ///        ("https://test:1234", None, None),
        ///        ("http://abc.com", Some("user"), Some("pass")),
        ///    ];
        ///
        ///    let psn = PSN::new(vec![psn_inner]).await.init_proxy(proxies);
        ///
        ///    Ok(())
        ///}
        ///```
        pub async fn init_proxy(
            mut self,
            proxies: Vec<(&str, Option<&str>, Option<&str>)>,
        ) -> Self {
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
            self
        }

        /// Add new proxy into `ProxyPoolManager` on the fly.
        /// The max proxy pool size is determined by the first proxies vector's length passed to 'PSN::init_proxy'(upper limit pool size is u8).
        /// Once you hit the max pool size all additional proxies become backup and can only be activated when an active proxy is dropped(connection broken for example)
        pub fn add_proxy(&self, proxies: Vec<(&str, Option<&str>, Option<&str>)>) {
            if let Some(pool) = &self.proxy_pool {
                for (address, username, password) in proxies.into_iter() {
                    pool.get_manager().add_proxy(address, username, password);
                }
            }
        }

        pub async fn get_profile<T: DeserializeOwned + 'static>(
            &self,
            online_id: &str,
        ) -> Result<T, PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner.get_profile(client, online_id).await
        }

        pub async fn get_titles<T: DeserializeOwned + 'static>(
            &self,
            online_id: &str,
            offset: u32,
        ) -> Result<T, PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner.get_titles(client, online_id, offset).await
        }

        pub async fn get_trophy_set<T: DeserializeOwned + 'static>(
            &self,
            online_id: &str,
            np_communication_id: &str,
        ) -> Result<T, PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner
                .get_trophy_set(client, online_id, np_communication_id)
                .await
        }

        pub async fn get_message_threads<T: DeserializeOwned + 'static>(
            &self,
            offset: u32,
        ) -> Result<T, PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner.get_message_threads(client, offset).await
        }

        pub async fn get_message_thread<T: DeserializeOwned + 'static>(
            &self,
            thread_id: &str,
        ) -> Result<T, PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner.get_message_thread(client, thread_id).await
        }

        pub async fn generate_message_thread(&self, online_id: &str) -> Result<(), PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner.generate_message_thread(client, online_id).await
        }

        pub async fn leave_message_thread(&self, thread_id: &str) -> Result<(), PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner.leave_message_thread(client, thread_id).await
        }

        pub async fn send_message(
            &self,
            online_id: &str,
            msg: Option<&str>,
            path: Option<&str>,
            thread_id: &str,
        ) -> Result<(), PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner
                .send_message(client, online_id, msg, path, thread_id)
                .await
        }

        pub async fn search_store_items<T: DeserializeOwned + 'static>(
            &self,
            lang: &str,
            region: &str,
            age: &str,
            name: &str,
        ) -> Result<T, PSNError> {
            let inner_ref = self.inner.get().await?;
            let proxy_ref = self.get_proxy_cli().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => &**proxy_ref,
                None => &self.client,
            };

            let psn_inner = &*inner_ref;

            psn_inner
                .search_store_items(client, lang, region, age, name)
                .await
        }

        async fn get_proxy_cli(&self) -> Result<Option<PoolRef<'_, ProxyPoolManager>>, PSNError> {
            let fut = match self.proxy_pool.as_ref() {
                Some(pool) => pool.get(),
                None => return Ok(None),
            };
            let pool_ref = fut.await?;
            Ok(Some(pool_ref))
        }

        pub fn clients_state(&self) -> u8 {
            self.proxy_pool
                .as_ref()
                .map(|pool| pool.state().idle_connections)
                .unwrap_or(0)
        }
    }
}
