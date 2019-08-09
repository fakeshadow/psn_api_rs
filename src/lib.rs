use std::time::{Duration, Instant};

use futures::Future;

use derive_more::Display;

#[cfg(feature = "awc")]
#[macro_use]
extern crate serde_derive;


/// A simple PSN API wrapper. It only uses a http client(actix-web-client in this case) to communicate wih the official PSN API.
//  ToDo: You can use MakeClientCall trait to impl your own http client.(Check the trait description)
///
/// Some basics:
/// The crate use a pair of uuid and two_step code to login in to PSN Network and get a pair of access_token and refresh_token in response.
///
/// access_token last about an hour before expire and it's need to call most other PSN APIs(The PSN store API doesn't need any token to access though).
///
/// refresh_token last much longer and it's used to generate a new access_token after it is expired.
///
/// * Some thing to note:
/// There is a rate limiter for the official PSN API so better not make lots of calls in short time
/// So its' best to avoid using in multi threads also as a single thread could hit the limit easily on any given machine running this crate.

/// Example:
///``` rust
///use std::io::stdin;
///
///use futures::{lazy, Future};
///
///use tokio::runtime::current_thread::Runtime;
///use tokio::prelude::*;
///use psn_api_rs::{MakeClientCall, PSN, PSNUser};
///
///fn main() {
///    println!(
///        "Pleas input your refresh_token if you alreayd have one. Press enter to skip to next\r\n"
///    );
///   let mut refresh_token = String::new();
///    let mut uuid = String::new();
///    let mut two_step = String::new();
///
///    stdin().read_line(&mut refresh_token).unwrap();
///
///    trim(&mut refresh_token);
///
///    if refresh_token.len() == 0 {
///        println!("Please input your uuid and press enter to continue.\r\n
///You can check this link below to see how to get one paired with a two_step code which will be needed later\r\n
///https://tusticles.com/psn-php/first_login.html\r\n");
///
///        stdin().read_line(&mut uuid).unwrap();
///        trim(&mut uuid);
///
///        println!("Please input your two_step code to continue.\r\n");
///
///        stdin().read_line(&mut two_step).unwrap();
///        trim(&mut two_step);
///    }
///
///    println!("Please wait for the PSN network to response. The program will panic if there is an error occur\r\n");
///
///
///    let mut runtime = Runtime::new().unwrap();
///
///    // construct and a new PSN,add credentials and call auth to generate tokens.
///    let psn: PSN = runtime.block_on(lazy(|| {
///        PSN::new()
///            .refresh_token(refresh_token)   // <- If refresh_token is provided then it's safe to ignore uuid and two_step arg and call .auth() directly.
///            .uuid(uuid) // <- uuid and two_step are used only when refresh_token is not working or not provided.
///            .two_step(two_step)
///            .auth()
///    })).unwrap_or_else(|e| panic!("{:?}", e));
///
///    println!(
///        "Authentication Success! These are your token info from PSN network: {:?} \r\n",
///        psn
///    );
///
///    let user: PSNUser = runtime.block_on(
///        psn.get_user_profile("Hakoom")  // <- use the psn struct to call for user_profile.
///    ).unwrap_or_else(|e| panic!("{:?}", e));
///
///    println!(
///        "Test finished. Got user info : {:?}",
///        user
///    );
///
///    // psn struct is dropped at this point so it's better to store your access_token and refresh_token here to make them reusable.
///}
///
///fn trim(s: &mut String) {
///    if s.ends_with("\n") {
///        s.pop();
///        if s.ends_with("\r") {
///            s.pop();
///        }
///    }
///}
///```

/// hard code urls and credentials. Can be changed if they are not working properly.
/// The region code can be changed for a better response time.(Change hk to us, jp ,etc.)
/// They are public as people who want to use another http client could use these entries directly.
pub const NP_SSO_ENTRY: &'static str =
    "https://auth.api.sonyentertainmentnetwork.com/2.0/ssocookie";

/// grant code entry is generate with this pattern
/// ```rust
/// format!("https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/authorize?duid={}&app_context=inapp_ios&client_id={}&scope={}&response_type=code", DUID, CLIENT_ID, SCOPE);
/// ```
pub const GRANT_CODE_ENTRY: &'static str =
    "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/authorize?duid=0000000d000400808F4B3AA3301B4945B2E3636E38C0DDFC&app_context=inapp_ios&client_id=b7cbf451-6bb6-4a5a-8913-71e61f462787&scope=capone:report_submission,psn:sceapp,user:account.get,user:account.settings.privacy.get,user:account.settings.privacy.update,user:account.realName.get,user:account.realName.update,kamaji:get_account_hash,kamaji:ugc:distributor,oauth:manage_device_usercodes&response_type=code";

pub const OAUTH_TOKEN_ENTRY: &'static str =
    "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/token";

const USERS_API: &'static str =
    "https://hk-prof.np.community.playstation.net/userProfile/v1/users/";

//const USER_TROPHY_API: &'static str =
//    "https://hk-tpy.np.community.playstation.net/trophy/v1/trophyTitles";
//const ACTIVITY_API: &'static str = "https://activity.api.np.km.playstation.net/activity/api/";
//const MESSAGE_THREAD_API: &'static str =
//    "https://hk-gmsg.np.community.playstation.net/groupMessaging/v1/";
//const STORE_API: &'static str = "https://store.playstation.com/valkyrie-api/";
//const REDIRECTURI: &'static str = "com.playstation.PlayStationApp://redirect";

const CLIENT_ID: &'static str = "b7cbf451-6bb6-4a5a-8913-71e61f462787";
const CLIENT_SECRET: &'static str = "zsISsjmCx85zgCJg";
const DUID: &'static str = "0000000d000400808F4B3AA3301B4945B2E3636E38C0DDFC";
const SCOPE: &'static str = "capone:report_submission,psn:sceapp,user:account.get,user:account.settings.privacy.get,user:account.settings.privacy.update,user:account.realName.get,user:account.realName.update,kamaji:get_account_hash,kamaji:ugc:distributor,oauth:manage_device_usercodes";

#[derive(Debug)]
pub struct PSN {
    access_token: Option<String>,
    uuid: Option<String>,
    two_step: Option<String>,
    refresh_token: Option<String>,
    last_refresh_at: Option<Instant>,
}

#[cfg(feature = "awc")]
#[derive(Deserialize)]
struct Npsso {
    npsso: String,
}

#[cfg(feature = "awc")]
#[derive(Deserialize)]
struct Tokens {
    access_token: Option<String>,
    refresh_token: Option<String>,
}

#[cfg(feature = "awc")]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PSNUser {
    //    pub trophyList: Vec<PSNGame>,
    pub online_id: String,
    pub np_id: String,
    pub region: String,
    pub avatar_url: String,
    pub about_me: String,
    pub languages_used: Vec<String>,
    pub plus: u8,
//    pub trophy_summary: TrophySummary
//    type: 'object',
//    properties: {
//    level: {
//    type: 'number'
//    },
//    progress: {
//    type: 'number'
//    },
//    earnedTrophies: earnedTrophiesObject
//    }
//    },
}


#[derive(Debug, Display)]
pub enum PSNError {
    #[display(fmt = "No Access Token is found. Please login in first")]
    NoAccessToken,
    #[display(fmt = "Can't connect to PSN network")]
    NetWork,
    #[display(fmt = "Can't properly parse response body")]
    PayLoad,
    #[display(fmt = "Can not extract np_sso_cookie from response body")]
    NoNPSSO,
    #[display(fmt = "Can not extract grand code from response header")]
    NoGrantCode,
    #[display(fmt = "Can not extract access and/or refresh token(s) from response body")]
    Tokens,
}

impl PSN {
    pub fn new() -> Self {
        PSN {
            access_token: None,
            uuid: None,
            two_step: None,
            refresh_token: None,
            last_refresh_at: None,
        }
    }

    pub fn refresh_token(mut self, refresh_token: String) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    pub fn uuid(mut self, uuid: String) -> Self {
        self.uuid = Some(uuid);
        self
    }

    pub fn two_step(mut self, two_step: String) -> Self {
        self.two_step = Some(two_step);
        self
    }

    pub fn get_access_token(&self) -> Result<&str, PSNError> {
        self.access_token
            .as_ref()
            .map(String::as_str)
            .ok_or(PSNError::NoAccessToken)
    }
}

/// You can override MakeClientCall trait to impl your preferred http client
///
/// The crate can provide the url, body format and some headers needed but the response handling you have to write your own.

pub trait MakeClientCall
    where
        Self: Sized + 'static,
{
    fn auth(self) -> Box<dyn Future<Item=PSN, Error=PSNError>> {
        Box::new(
            self.gen_access_from_refresh()
                .or_else(|(_, p)| p.gen_access_and_refresh())
                .map_err(|(e, _)| e),
        )
    }

    /// This method will use uuid and two_step to get a new pair of access_token and refresh_token from PSN.
    fn gen_access_and_refresh(self) -> Box<dyn Future<Item=PSN, Error=(PSNError, Self)>>;

    /// This method will use local refresh_token to get a new access_token from PSN.
    fn gen_access_from_refresh(self) -> Box<dyn Future<Item=PSN, Error=(PSNError, Self)>>;

    #[cfg(feature = "awc")]
    fn get_user_profile(&self, username: &str) -> Box<dyn Future<Item=PSNUser, Error=PSNError>>;
}

#[cfg(feature = "awc")]
impl PSN {
    fn make_client() -> awc::Client {
        awc::Client::build()
            .connector(
                awc::Connector::new()
                    .timeout(Duration::from_secs(100))
                    .finish(),
            )
            .timeout(Duration::from_secs(100))
            .finish()
    }
}

#[cfg(feature = "awc")]
impl MakeClientCall for PSN {
    fn gen_access_and_refresh(self) -> Box<dyn Future<Item=PSN, Error=(PSNError, PSN)>> {
        Box::new(
            // User uuid and two_step code to make a call.
            PSN::make_client()
                .post(NP_SSO_ENTRY)
                .send_form(&self.np_sso_url_encode())
                .then(|r| match r {
                    Ok(r) => Ok((r, self)),
                    Err(_) => Err((PSNError::NetWork, self)),
                })
                .and_then(|(mut res, psn)| {
                    // At this point the uuid and two_step code are consumed and can't be used anymore.
                    // If you failed from this point for any reason the only way to start over is to get a new pair of uuid and two_step code.

                    // Extract the npsso cookie as string from the response json body.
                    res.json().then(|r: Result<Npsso, _>| match r {
                        Ok(n) => Ok((n, psn)),
                        Err(_) => Err((PSNError::NoNPSSO, psn)),
                    })
                })
                .and_then(|(t, psn)| {
                    // Use the npsso we get as a cookie header in a get call.
                    PSN::make_client()
                        .get(GRANT_CODE_ENTRY)
                        .header("Cookie", format!("npsso={}", t.npsso))
                        .header(
                            awc::http::header::CONTENT_TYPE,
                            "application/x-www-form-urlencoded",
                        )
                        .send()
                        .then(|r| match r {
                            Ok(r) => Ok((r, psn)),
                            Err(_) => Err((PSNError::NetWork, psn)),
                        })
                })
                .and_then(|(res, psn)| {
                    // Extract the "x-np-grant-code" from the response header and parse it to string.
                    match res.headers().get("x-np-grant-code") {
                        Some(h) => Ok((h.to_str().unwrap().to_owned(), psn)),
                        None => Err((PSNError::NoGrantCode, psn)),
                    }
                })
                .and_then(|(grant, psn)| {
                    // Use the grant code to make another post call to finish the authentication process.
                    PSN::make_client()
                        .post(OAUTH_TOKEN_ENTRY)
                        .send_form(&PSN::oauth_token_encode(grant))
                        .then(|r| match r {
                            Ok(r) => Ok((r, psn)),
                            Err(_) => Err((PSNError::NetWork, psn)),
                        })
                })
                .and_then(|(mut res, mut p)| {
                    // Extract the access_token and refresh_token from the response body json.
                    res.json().then(|r: Result<Tokens, _>| match r {
                        Ok(t) => {
                            p.access_token = t.access_token;
                            p.refresh_token = t.refresh_token;
                            Ok(p)
                        }
                        Err(_) => Err((PSNError::Tokens, p)),
                    })
                }),
        )
    }

    fn gen_access_from_refresh(self) -> Box<dyn Future<Item=PSN, Error=(PSNError, PSN)>> {
        Box::new(
            PSN::make_client()
                .post(OAUTH_TOKEN_ENTRY)
                .send_form(&self.oauth_token_refresh_encode())
                .then(|r| match r {
                    Ok(r) => Ok((r, self)),
                    Err(_) => Err((PSNError::NetWork, self))
                })
                .and_then(|(mut res, mut psn)| {
                    res.json().then(|r: Result<Tokens, _>| match r {
                        Ok(t) => {
                            psn.access_token = t.access_token;
                            Ok(psn)
                        }
                        Err(_) => Err((PSNError::Tokens, psn)),
                    })
                })
            ,
        )
    }

    #[cfg(feature = "awc")]
    fn get_user_profile(&self, online_id: &str) -> Box<dyn Future<Item=PSNUser, Error=PSNError>> {
        Box::new(
            PSN::make_client()
                .get(PSN::user_profile_encode(online_id))
                .header(
                    awc::http::header::CONTENT_TYPE,
                    "application/json",
                )
                .bearer_auth(self.access_token.as_ref().unwrap())
                .send()
                .map_err(|_| PSNError::NetWork)
                .and_then(|mut res| {
                    res.json().map_err(|_| PSNError::PayLoad).map(|r: PSNUser| r)
                })
        )
    }
}

impl PSN {
    pub fn np_sso_url_encode(&self) -> [(&'static str, String); 4] {
        let uuid = self
            .uuid
            .as_ref()
            .map(Clone::clone)
            .expect("uuid is not a proper string");
        let two_step = self
            .two_step
            .as_ref()
            .map(Clone::clone)
            .expect("two_step code is not a proper string");

        /// serde_urlencoded can be used to make a `application/x-wwww-url-encoded` `String` buffer from form
        /// the same applies to all self.XXX_url_encode() and Self::XXX_url_encode methods.
        /// ```rust
        /// use psn_api_rs::PSN;
        /// use serde_urlencoded;
        /// impl PSN {
        ///     fn url_query_string(&self) -> String {
        ///         serde_urlencoded::to_string(&self.np_sso_url_encode()).unwrap()
        ///     }
        /// }
        /// ```
        [
            ("authentication_type", "two_step".to_owned()),
            ("client_id", CLIENT_ID.to_owned()),
            ("ticket_uuid", uuid),
            ("code", two_step),
        ]
    }

    pub fn oauth_token_encode(grant_code: String) -> [(&'static str, String); 6] {
        [
            ("client_id", CLIENT_ID.to_owned()),
            ("client_secret", CLIENT_SECRET.to_owned()),
            ("duid", DUID.to_owned()),
            ("scope", SCOPE.to_owned()),
            ("code", grant_code),
            ("grant_type", "authorization_code".to_owned()),
        ]
    }

    pub fn oauth_token_refresh_encode(&self) -> [(&'static str, String); 7] {
        [
            ("app_context", "inapp_ios".to_owned()),
            ("client_id", CLIENT_ID.to_owned()),
            ("client_secret", CLIENT_SECRET.to_owned()),
            ("duid", DUID.to_owned()),
            ("scope", SCOPE.to_owned()),
            (
                "refresh_token",
                self.refresh_token.as_ref().map(String::as_str).unwrap_or("lazy uncheck").to_owned(),
            ),
            ("grant_type", "refresh_token".to_owned()),
        ]
    }

    pub fn user_profile_encode(online_id: &str) -> String {
        format!("{}{}/profile?fields=%40default,relation,requestMessageFlag,presence,%40personalDetail,trophySummary", USERS_API, online_id)
    }
}
