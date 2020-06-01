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
//!    let user = psn_inner
//!            .get_profile::<PSNUser>(&client, "Hakoom")
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
    use std::future::Future;
    use std::sync::{Mutex, MutexGuard};
    use std::time::Duration;

    use derive_more::Display;
    use reqwest::{Client, ClientBuilder, Error, Proxy};
    use serde::de::DeserializeOwned;
    use tang_rs::{Builder, Manager, ManagerFuture, ManagerTimeout, Pool, PoolRef};
    use tokio::time::{delay_for, Delay};

    use crate::models::MessageThreadNew;
    use crate::traits::PSNRequest;
    use crate::types::PSNInner;

    #[derive(Debug, Clone)]
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
        #[display(fmt = "No PSNInner object is available")]
        NoPSNInner,
        #[display(fmt = "Failed to login in to PSN on npsso code: {}", _0)]
        InvalidNpsso(Box<str>),
        #[display(fmt = "Failed to login in to PSN on refresh token: {}", _0)]
        InvalidRefresh(Box<str>),
        #[display(fmt = "Request to PSN pool is timeout.")]
        TimeOut,
        #[display(fmt = "Error from Reqwest http client: {}", _0)]
        FromReqwest(Error),
        #[display(fmt = "Error from PSN response: {}", _0)]
        FromPSN(Box<str>),
        #[display(fmt = "Error from IO: {}", _0)]
        FromStd(std::io::Error),
    }

    impl From<()> for PSNError {
        fn from(_: ()) -> Self {
            PSNError::TimeOut
        }
    }

    pub struct PSNInnerManager {
        inner: Mutex<Vec<PSNInner>>,
        client: Client,
    }

    impl PSNInnerManager {
        fn new() -> Self {
            PSNInnerManager {
                inner: Mutex::new(Vec::new()),
                client: ClientBuilder::new()
                    .build()
                    .expect("Failed to build http client for PSNInnerManager"),
            }
        }

        fn get_psn_inner(&self) -> MutexGuard<'_, Vec<PSNInner>> {
            self.inner.lock().unwrap()
        }

        fn add_psn_inner(&self, psn_inner: Vec<PSNInner>) {
            let mut inners = self.get_psn_inner();
            for psn in psn_inner.into_iter() {
                for (index, inner) in inners.iter().enumerate() {
                    if psn.get_email() == inner.get_email() {
                        inners.remove(index);
                        break;
                    }
                }
                inners.push(psn);
            }
        }
    }

    impl Manager for PSNInnerManager {
        type Connection = PSNInner;
        type Error = PSNError;
        type Timeout = Delay;
        type TimeoutError = ();

        fn connect(&self) -> ManagerFuture<'_, Result<Self::Connection, Self::Error>> {
            Box::pin(async move { self.get_psn_inner().pop().ok_or(PSNError::NoClient) })
        }

        fn is_valid<'a>(
            &'a self,
            conn: &'a mut Self::Connection,
        ) -> ManagerFuture<'a, Result<(), Self::Error>> {
            Box::pin(async move {
                if conn.should_refresh() {
                    conn.gen_access_from_refresh(&self.client).await
                } else {
                    Ok(())
                }
            })
        }

        fn is_closed(&self, _conn: &mut Self::Connection) -> bool {
            false
        }

        fn spawn<Fut>(&self, fut: Fut)
        where
            Fut: Future<Output = ()> + Send + 'static,
        {
            tokio::spawn(fut);
        }

        fn timeout<Fut: Future>(
            &self,
            fut: Fut,
            dur: Duration,
        ) -> ManagerTimeout<Fut, Self::Timeout> {
            ManagerTimeout::new(fut, delay_for(dur))
        }
    }

    type Proxies = Mutex<Vec<(String, Option<String>, Option<String>)>>;

    pub struct ProxyPoolManager {
        proxies: Proxies,
        marker: &'static str,
    }

    impl ProxyPoolManager {
        fn new() -> Self {
            ProxyPoolManager {
                proxies: Mutex::new(Vec::new()),
                marker: "https://www.google.com",
            }
        }

        fn add_proxy(&self, proxies: Vec<(&str, Option<&str>, Option<&str>)>) {
            let mut inner = self.proxies.lock().unwrap();

            for (address, username, password) in proxies.into_iter() {
                inner.push((
                    address.into(),
                    username.map(Into::into),
                    password.map(Into::into),
                ))
            }
        }
    }

    impl Manager for ProxyPoolManager {
        type Connection = Client;
        type Error = PSNError;
        type Timeout = Delay;
        type TimeoutError = ();

        fn connect(&self) -> ManagerFuture<'_, Result<Self::Connection, Self::Error>> {
            Box::pin(async move {
                let (address, username, password) = self
                    .proxies
                    .lock()
                    .unwrap()
                    .pop()
                    .ok_or(PSNError::NoClient)?;
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

        fn spawn<Fut>(&self, fut: Fut)
        where
            Fut: Future<Output = ()> + Send + 'static,
        {
            tokio::spawn(fut);
        }

        fn timeout<Fut: Future>(
            &self,
            fut: Fut,
            dur: Duration,
        ) -> ManagerTimeout<Fut, Self::Timeout> {
            ManagerTimeout::new(fut, delay_for(dur))
        }
    }

    impl From<Error> for PSNError {
        fn from(e: Error) -> Self {
            PSNError::FromReqwest(e)
        }
    }

    impl PSN {
        /// A shortcut for building a temporary http client
        pub fn new_client() -> Result<Client, PSNError> {
            ClientBuilder::new().build().map_err(|_| PSNError::NoClient)
        }

        /// Accept multiple PSNInner and  use them concurrently with a pool.
        pub async fn new(psn_inner: Vec<PSNInner>) -> Self {
            let mgr = PSNInnerManager::new();

            let size = psn_inner.len();

            mgr.add_psn_inner(psn_inner);

            let inner_pool = Builder::new()
                .always_check(true)
                .idle_timeout(None)
                .max_lifetime(None)
                .min_idle(0)
                .max_size(size)
                .build(mgr)
                .await
                .expect("Failed to build PSNInner pool");

            PSN {
                inner: inner_pool,
                client: Self::new_client().expect("Failed to build http client"),
                proxy_pool: None,
            }
        }

        /// Add new PSNInner to Manager. This inner will be used as backup and only when an active PSNInner is dropped from pool will it be used.
        ///
        /// It's a good idea to clear all the backup PSNInners and replace them with new ones in schedule.
        pub fn add_psn_inner(&self, inners: Vec<PSNInner>) {
            self.inner.get_manager().add_psn_inner(inners);
        }

        pub fn set_psn_inner_max(&self, max_size: usize) {
            self.inner.set_max_size(max_size);
        }

        pub fn pause_inner(&self) {
            self.inner.pause();
        }

        pub fn resume_inner(&self) {
            self.inner.resume();
        }

        pub fn clear_inner(&self) {
            self.inner.clear();
        }

        /// Add http proxy pool to combat PSN rate limiter. This is not required.
        ///# Example:
        ///```no_run
        ///use psn_api_rs::{psn::PSN, types::PSNInner, traits::PSNRequest};
        ///
        ///#[tokio::main]
        ///async fn main() -> std::io::Result<()> {
        ///    let mut psn_inner = PSNInner::new();
        ///
        ///    psn_inner.add_refresh_token("refresh_token".into())
        ///             .add_npsso("npsso".into());
        ///
        ///    psn_inner = psn_inner
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
        ///    let psn = PSN::new(vec![psn_inner]).await.init_proxy(proxies).await;
        ///
        ///    Ok(())
        ///}
        ///```
        pub async fn init_proxy(
            mut self,
            proxies: Vec<(&str, Option<&str>, Option<&str>)>,
        ) -> Self {
            let mgr = ProxyPoolManager::new();
            let size = proxies.len();
            mgr.add_proxy(proxies);

            let pool = Builder::new()
                .always_check(false)
                .idle_timeout(None)
                .max_lifetime(None)
                .min_idle(0)
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
                pool.get_manager().add_proxy(proxies);
            }
        }

        pub async fn get_profile<T: DeserializeOwned + 'static>(
            &self,
            online_id: &str,
        ) -> Result<T, PSNError> {
            let (client, psn_inner) = self.get().await?;

            psn_inner.get_profile(&client, online_id).await
        }

        pub async fn get_titles<T: DeserializeOwned + 'static>(
            &self,
            online_id: &str,
            offset: u32,
        ) -> Result<T, PSNError> {
            let (client, psn_inner) = self.get().await?;

            psn_inner.get_titles(&client, online_id, offset).await
        }

        pub async fn get_trophy_set<T: DeserializeOwned + 'static>(
            &self,
            online_id: &str,
            np_communication_id: &str,
        ) -> Result<T, PSNError> {
            let (client, psn_inner) = self.get().await?;

            psn_inner
                .get_trophy_set(&client, online_id, np_communication_id)
                .await
        }

        pub async fn get_message_threads<T: DeserializeOwned + 'static>(
            &self,
            offset: u32,
        ) -> Result<T, PSNError> {
            let (client, psn_inner) = self.get().await?;

            psn_inner.get_message_threads(&client, offset).await
        }

        pub async fn get_message_thread<T: DeserializeOwned + 'static>(
            &self,
            thread_id: &str,
        ) -> Result<T, PSNError> {
            let (client, psn_inner) = self.get().await?;

            psn_inner.get_message_thread(&client, thread_id).await
        }

        pub async fn leave_message_thread(&self, thread_id: &str) -> Result<(), PSNError> {
            let (client, psn_inner) = self.get().await?;

            psn_inner.leave_message_thread(&client, thread_id).await
        }

        pub async fn send_message(
            &self,
            online_id: &str,
            msg: Option<&str>,
            path: Option<&str>,
        ) -> Result<(), PSNError> {
            let (client, psn_inner) = self.get().await?;

            let thread: MessageThreadNew = psn_inner
                .generate_message_thread(&client, online_id)
                .await?;

            psn_inner
                .send_message(&client, online_id, msg, path, &thread.thread_id)
                .await
        }

        pub async fn send_message_with_buf(
            &self,
            online_id: &str,
            msg: Option<&str>,
            buf: Option<&[u8]>,
        ) -> Result<(), PSNError> {
            let (client, psn_inner) = self.get().await?;

            let thread: MessageThreadNew = psn_inner
                .generate_message_thread(&client, online_id)
                .await?;

            psn_inner
                .send_message_with_buf(&client, online_id, msg, buf, &thread.thread_id)
                .await
        }

        pub async fn search_store_items<T: DeserializeOwned + 'static>(
            &self,
            lang: &str,
            region: &str,
            age: &str,
            name: &str,
        ) -> Result<T, PSNError> {
            let (client, psn_inner) = self.get().await?;

            psn_inner
                .search_store_items(&client, lang, region, age, name)
                .await
        }

        pub fn get_inner(&self) -> Pool<PSNInnerManager> {
            self.inner.clone()
        }

        async fn get(&self) -> Result<(Client, PoolRef<'_, PSNInnerManager>), PSNError> {
            let proxy_ref = self.get_proxy_cli().await?;
            let inner_ref = self.inner.get().await?;

            let client = match proxy_ref.as_ref() {
                Some(proxy_ref) => (&**proxy_ref).clone(),
                None => (&self.client).clone(),
            };

            drop(proxy_ref);

            Ok((client, inner_ref))
        }

        async fn get_proxy_cli(&self) -> Result<Option<PoolRef<'_, ProxyPoolManager>>, PSNError> {
            let fut = match self.proxy_pool.as_ref() {
                Some(pool) => pool.get(),
                None => return Ok(None),
            };
            let pool_ref = fut.await?;
            Ok(Some(pool_ref))
        }
    }
}
