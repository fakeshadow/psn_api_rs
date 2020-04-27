// this example does not work properly despite it builds.
use std::future::Future;
use std::pin::Pin;

use reqwest::{header, Client, Method, Request, Url};

use psn_api_rs::{
    models::PSNUser,
    traits::{EncodeUrl, PSNRequest},
    types::PSNInner,
};
use serde::de::DeserializeOwned;

#[derive(Debug)]
struct MyPSN {
    inner: PSNInner,
    client: Client,
}

impl From<PSNInner> for MyPSN {
    fn from(psn: PSNInner) -> Self {
        MyPSN {
            inner: psn,
            client: Client::new(),
        }
    }
}

impl EncodeUrl for MyPSN {
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

#[derive(Debug)]
struct MyError;

/* place holder impls. The impl detail is determined by your http client. */
impl PSNRequest for MyPSN {
    type Client = ();
    type Error = MyError;

    fn gen_access_and_refresh(
        &mut self,
        _client: &Self::Client,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn gen_access_from_refresh(
        &mut self,
        _client: &Self::Client,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn get_by_url_encode<'s, 'u: 's, T: DeserializeOwned + 'static>(
        &'s self,
        _client: &Self::Client,
        url: &'u str,
    ) -> Pin<Box<dyn Future<Output = Result<T, Self::Error>> + Send + 's>> {
        Box::pin(async move {
            /*
                this method is a proper impl.
            */

            let mut req = Request::new(Method::GET, Url::parse(url).expect("invalid url"));

            let headers = req.headers_mut();

            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

            if let Some(token) = self.access_token() {
                headers.insert(
                    header::AUTHORIZATION,
                    format!("Bearer {}", token).parse().unwrap(),
                );
            }

            let res = self
                .client
                .execute(req)
                .await
                .map_err(|_| MyError)?
                .json::<T>()
                .await
                .map_err(|_| MyError)?;

            Ok(res)
        })
    }

    fn del_by_url_encode<'s, 'u: 's>(
        &'s self,
        _client: &Self::Client,
        url: &'u str,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn post_by_multipart<'s, 't: 's>(
        &'s self,
        _client: &Self::Client,
        boundary: &'t str,
        url: &'t str,
        body: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn read_path(path: &str) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, Self::Error>> + Send>> {
        Box::pin(async move { Ok(vec![]) })
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let npsso = "npsso".into();

    let mut psn = PSNInner::new();
    psn.set_region("us".to_owned())
        .set_lang("en".to_owned())
        .set_self_online_id(String::from("Your Login account PSN online_id"))
        .add_npsso(npsso);

    let mut my_psn: MyPSN = psn.into();

    my_psn = my_psn.auth().await.unwrap_or_else(|e| panic!("{:?}", e));

    println!(
        "\r\nAuthentication Success! You PSN info are:\r\n{:#?}",
        my_psn
    );

    let user = my_psn
        .get_profile::<PSNUser>(&(), "Hakoom")
        .await
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot examples user info : \r\n{:#?}", user);

    Ok(())
}
