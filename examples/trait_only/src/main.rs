// this example does not work properly despite it builds.

use std::borrow::Cow;
use std::pin::Pin;

use reqwest::{header, Client, Method, Request, Url};
use tokio::prelude::Future;

use psn_api_rs::{models::PSNUser, EncodeUrl, PSNRequest, PSN};
use serde::de::DeserializeOwned;

#[derive(Debug)]
struct MyPSN {
    psn: PSN,
    client: Client,
}

impl From<PSN> for MyPSN {
    fn from(psn: PSN) -> Self {
        MyPSN {
            psn,
            client: Client::new(),
        }
    }
}

impl EncodeUrl for MyPSN {
    fn uuid(&self) -> &str {
        self.psn.uuid()
    }

    fn two_step(&self) -> &str {
        self.psn.two_step()
    }

    fn access_token(&self) -> Option<&str> {
        self.psn.access_token()
    }

    fn refresh_token(&self) -> &str {
        self.psn.refresh_token()
    }

    fn region(&self) -> &str {
        self.psn.region()
    }

    fn other_online_id(&self) -> &str {
        self.psn.other_online_id()
    }

    fn self_online_id(&self) -> &str {
        self.psn.self_online_id()
    }

    fn language(&self) -> &str {
        self.psn.language()
    }

    fn np_communication_id(&self) -> &str {
        self.psn.np_communication_id()
    }
}

#[derive(Debug)]
struct MyError;

/* place holder impls. The impl detail is determined by your http client. */
impl PSNRequest for MyPSN {
    type Error = MyError;

    fn gen_access_and_refresh(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn gen_access_from_refresh(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn get_by_url_encode<'s, 'u: 's, T: DeserializeOwned + 'static>(
        &'s self,
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
        url: &'u str,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn post_by_multipart<'s, 't: 's>(
        &'s self,
        boundary: &'t str,
        url: &'t str,
        body: Cow<'static, [u8]>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn read_path(
        path: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Cow<'static, [u8]>, Self::Error>> + Send>> {
        Box::pin(async move { Ok(Cow::Owned(vec![])) })
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let uuid = "uuid".into();
    let two_step = "two_step".into();

    let mut my_psn: MyPSN = PSN::new()
        .set_region("us".to_owned())
        .set_lang("en".to_owned())
        .set_self_online_id(String::from("Your Login account PSN online_id"))
        .add_uuid(uuid)
        .add_two_step(two_step)
        .into();

    my_psn = my_psn.auth().await.unwrap_or_else(|e| panic!("{:?}", e));

    println!(
        "\r\nAuthentication Success! You PSN info are:\r\n{:#?}",
        my_psn
    );

    my_psn.psn.add_online_id("Hakoom".to_owned());

    let user: PSNUser = my_psn
        .get_profile()
        .await
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot examples user info : \r\n{:#?}", user);

    Ok(())
}
