use std::time::{Duration, Instant};

use futures::Future;

use derive_more::Display;
use serde::de::DeserializeOwned;

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
///use futures::lazy;
///
///use tokio::runtime::current_thread::Runtime;
///use psn_api_rs::{PSNRequest, PSN, PSNUser};
///
///fn main() {
///    let refresh_token = String::from("your refresh token");
///    let uuid = String::from("your uuid");
///    let two_step = String::from("your two_step code");
///
///    let mut runtime = Runtime::new().unwrap();
///
///    // construct and a new PSN,add credentials and call auth to generate tokens.
///    let mut psn: PSN = runtime.block_on(lazy(|| {
///        PSN::new()
///            .add_refresh_token(refresh_token)   // <- If refresh_token is provided then it's safe to ignore uuid and two_step arg and call .auth() directly.
///            .add_uuid(uuid) // <- uuid and two_step are used only when refresh_token is not working or not provided.
///            .add_two_step(two_step)
///            .auth()
///    })).unwrap_or_else(|e| panic!("{:?}", e));
///
///    println!(
///        "Authentication Success! These are your token info from PSN network: {:?} \r\n",
///        psn
///    );
///
///    let user: PSNUser = runtime.block_on(
///        psn.add_online_id("Hakoom".to_owned()).get_profile()  // <- use the psn struct to call for user_profile.
///    ).unwrap_or_else(|e| panic!("{:?}", e));
///
///    println!(
///        "Test finished. Got user info : {:?}",
///        user
///    );
///
///    // psn struct is dropped at this point so it's better to store your access_token and refresh_token here to make them reusable.
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

const USER_TROPHY_API: &'static str =
    "https://hk-tpy.np.community.playstation.net/trophy/v1/trophyTitles/";

//const ACTIVITY_API: &'static str = "https://activity.api.np.km.playstation.net/activity/api/";
//const MESSAGE_THREAD_API: &'static str =
//    "https://hk-gmsg.np.community.playstation.net/groupMessaging/v1/";
//const STORE_API: &'static str = "https://store.playstation.com/valkyrie-api/";

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
    /// the displayed online_id of PSN user. used to query user's info.
    online_id: Option<String>,
    /// np_id is PSN user's real unique id as online_id can be changed so it's best to use this as user identifier.
    np_id: Option<String>,
    /// np_communication_id is PSN game's identifier. Can be obtained by getting user's game summary API.(Only the games the target user have played will return)
    np_communication_id: Option<String>,
    language: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PSNUser {
    pub online_id: String,
    pub np_id: String,
    pub region: String,
    pub avatar_url: String,
    pub about_me: String,
    pub languages_used: Vec<String>,
    pub plus: u8,
    pub trophy_summary: PSNUserTrophySummary,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PSNUserTrophySummary {
    level: u8,
    progress: u8,
    earned_trophies: EarnedTrophies,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TrophyTitles {
    pub total_results: u32,
    pub offset: u32,
    pub trophy_titles: Vec<TrophyTitle>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TrophyTitle {
    /// trophy title struct skip a lot of response fields. If you need a more detailed trophy title response.
    /// You could use your own struct. Typical Torphy Title fields are:
    /// ```rust
    /// use psn_api_rs::{EarnedTrophies, TitleDetail};
    /// struct ExampleTrophyTitle {
    ///    //use camelcase as this is a copy paste from response json.
    ///    npCommunicationId: String,
    ///    trophyTitleName: String,
    ///    trophyTitleDetail: String,
    ///    trophyTitleIconUrl: String,
    ///    trophyTitlePlatfrom: String,
    ///    hasTrophyGroups: bool,
    ///    definedTrophies: EarnedTrophies,
    ///    comparedUser: TitleDetail
    /// }
    /// ```
    pub np_communication_id: String,
    #[serde(alias = "comparedUser")]
    pub title_detail: TitleDetail,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TitleDetail {
    pub progress: u8,
    pub earned_trophies: EarnedTrophies,
    pub last_update_date: String,
}

#[derive(Deserialize, Debug)]
pub struct EarnedTrophies {
    pub platinum: u32,
    pub gold: u32,
    pub silver: u32,
    pub bronze: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TrophySet {
    pub trophies: Vec<Trophy>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Trophy {
    trophy_id: u8,
    trophy_hidden: bool,
    trophy_type: Option<String>,
    trophy_name: Option<String>,
    trophy_detail: Option<String>,
    trophy_icon_url: Option<String>,
    trophy_rare: u8,
    trophy_earned_rate: String,
    #[serde(alias = "comparedUser")]
    user_info: TrophyUser,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TrophyUser {
    online_id: String,
    earned: bool,
    earned_date: Option<String>,
}

#[cfg(feature = "awc")]
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
            online_id: None,
            np_id: None,
            np_communication_id: None,
            language: Some("en".to_owned()),
        }
    }

    pub fn add_refresh_token(mut self, refresh_token: String) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    pub fn add_uuid(mut self, uuid: String) -> Self {
        self.uuid = Some(uuid);
        self
    }

    pub fn add_two_step(mut self, two_step: String) -> Self {
        self.two_step = Some(two_step);
        self
    }

    /// determine the response language from PSN APIs. Will return English if not set.
    pub fn add_language(mut self, region: String) -> Self {
        self.language = Some(region);
        self
    }

    pub fn add_online_id(&mut self, online_id: String) -> &mut Self {
        self.online_id = Some(online_id);
        self
    }

    pub fn add_np_id(&mut self, np_id: String) -> &mut Self {
        self.np_id = Some(np_id);
        self
    }

    pub fn add_np_communication_id(&mut self, np_communication_id: String) -> &mut Self {
        self.np_communication_id = Some(np_communication_id);
        self
    }

    /// check if it's about time the access_token expires.
    pub fn should_refresh(&self) -> bool {
        if Instant::now().duration_since(self.last_refresh_at.unwrap_or(Instant::now()))
            > Duration::from_secs(3000)
        {
            true
        } else {
            false
        }
    }
}

/// You can override PSNRequest trait to impl your preferred http client
///
/// The crate can provide the url, body format and some headers needed but the response handling you have to write your own.

pub trait PSNRequest
where
    Self: Sized + EncodeUrl + 'static,
{
    type Error;

    fn auth(self) -> Box<dyn Future<Item = PSN, Error = Self::Error>> {
        Box::new(
            self.gen_access_from_refresh()
                .or_else(|(_, p)| p.gen_access_and_refresh())
                .map_err(|(e, _)| e),
        )
    }

    /// This method will use uuid and two_step to get a new pair of access_token and refresh_token from PSN.
    fn gen_access_and_refresh(self) -> Box<dyn Future<Item = PSN, Error = (Self::Error, Self)>>;

    /// This method will use local refresh_token to get a new access_token from PSN.
    fn gen_access_from_refresh(self) -> Box<dyn Future<Item = PSN, Error = (Self::Error, Self)>>;

    /// A general http get handle function. The return type T need to impl serde::deserialize.
    fn get_by_url_encode<T: serde::de::DeserializeOwned + 'static>(
        &self,
        url: &str,
    ) -> Box<dyn Future<Item = T, Error = Self::Error>>;

    /// need to add_online_id before call this method.
    fn get_profile<T>(&self) -> Box<dyn Future<Item = T, Error = Self::Error>>
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.profile_encode().as_str())
    }

    /// need to add_online_id and give a legit offset(offset can't be larger than the total trophy lists a user have).
    fn get_titles<T>(&self, offset: u32) -> Box<dyn Future<Item = T, Error = Self::Error>>
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.trophy_summary_encode(offset).as_str())
    }

    /// need to add_online_id and add_np_communication_id before call this method.
    fn get_trophy_set<T>(&self) -> Box<dyn Future<Item = T, Error = Self::Error>>
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.trophy_set_encode().as_str())
    }
}

#[cfg(feature = "awc")]
impl PSN {
    fn http_client() -> awc::Client {
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
impl PSNRequest for PSN {
    type Error = PSNError;
    fn gen_access_and_refresh(self) -> Box<dyn Future<Item = PSN, Error = (PSNError, PSN)>> {
        Box::new(
            /// User uuid and two_step code to make a post call.
            PSN::http_client()
                .post(NP_SSO_ENTRY)
                .send_form(&self.np_sso_url_encode())
                .then(|r| match r {
                    Ok(r) => Ok((r, self)),
                    Err(_) => Err((PSNError::NetWork, self)),
                })
                .and_then(|(mut res, psn)| {
                    /// At this point the uuid and two_step code are consumed and can't be used anymore.
                    /// If you failed from this point for any reason the only way to start over is to get a new pair of uuid and two_step code.

                    /// Extract the npsso cookie as string from the response json body.
                    res.json().then(|r: Result<Npsso, _>| match r {
                        Ok(n) => Ok((n, psn)),
                        Err(_) => Err((PSNError::NoNPSSO, psn)),
                    })
                })
                .and_then(|(t, psn)| {
                    /// Use the npsso we get as a cookie header in a get call.
                    PSN::http_client()
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
                    /// Extract the "x-np-grant-code" from the response header and parse it to string.
                    match res.headers().get("x-np-grant-code") {
                        Some(h) => Ok((h.to_str().unwrap().to_owned(), psn)),
                        None => Err((PSNError::NoGrantCode, psn)),
                    }
                })
                .and_then(|(grant, psn)| {
                    /// Use the grant code to make another post call to finish the authentication process.
                    PSN::http_client()
                        .post(OAUTH_TOKEN_ENTRY)
                        .send_form(&PSN::oauth_token_encode(grant))
                        .then(|r| match r {
                            Ok(r) => Ok((r, psn)),
                            Err(_) => Err((PSNError::NetWork, psn)),
                        })
                })
                .and_then(|(mut res, mut psn)| {
                    /// Extract the access_token and refresh_token from the response body json.
                    res.json().then(|r: Result<Tokens, _>| match r {
                        Ok(t) => {
                            psn.last_refresh_at = Some(Instant::now());
                            psn.access_token = t.access_token;
                            psn.refresh_token = t.refresh_token;
                            Ok(psn)
                        }
                        Err(_) => Err((PSNError::Tokens, psn)),
                    })
                }),
        )
    }

    fn gen_access_from_refresh(self) -> Box<dyn Future<Item = PSN, Error = (PSNError, PSN)>> {
        Box::new(
            /// Basically the same process as the last step of gen_access_and_refresh method with a slightly different url encode.
            /// We only need the new access token from response.(refresh token can't be refreshed.)
            PSN::http_client()
                .post(OAUTH_TOKEN_ENTRY)
                .send_form(&self.oauth_token_refresh_encode())
                .then(|r| match r {
                    Ok(r) => Ok((r, self)),
                    Err(_) => Err((PSNError::NetWork, self)),
                })
                .and_then(|(mut res, mut psn)| {
                    res.json().then(|r: Result<Tokens, _>| match r {
                        Ok(t) => {
                            psn.last_refresh_at = Some(Instant::now());
                            psn.access_token = t.access_token;
                            Ok(psn)
                        }
                        Err(_) => Err((PSNError::Tokens, psn)),
                    })
                }),
        )
    }

    fn get_by_url_encode<T>(&self, url: &str) -> Box<dyn Future<Item = T, Error = PSNError>>
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        Box::new(
            /// The access_token is used as bearer token and content type header need to be application/json.
            PSN::http_client()
                .get(url)
                .header(awc::http::header::CONTENT_TYPE, "application/json")
                .bearer_auth(self.access_token.as_ref().unwrap())
                .send()
                .map_err(|_| PSNError::NetWork)
                .and_then(|mut res| res.json().map_err(|_| PSNError::PayLoad)),
        )
    }
}

pub trait EncodeUrl {
    fn np_sso_url_encode(&self) -> [(&'static str, String); 4];

    fn oauth_token_encode(grant_code: String) -> [(&'static str, String); 6];

    fn oauth_token_refresh_encode(&self) -> [(&'static str, String); 7];

    fn profile_encode(&self) -> String;

    fn trophy_summary_encode(&self, offset: u32) -> String;

    fn trophy_set_encode(&self) -> String;
}

impl EncodeUrl for PSN {
    fn np_sso_url_encode(&self) -> [(&'static str, String); 4] {
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
        /// example if your http client don't support auto urlencode convert.
        /// ```rust
        /// use psn_api_rs::{PSN, EncodeUrl};
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

    fn oauth_token_encode(grant_code: String) -> [(&'static str, String); 6] {
        [
            ("client_id", CLIENT_ID.to_owned()),
            ("client_secret", CLIENT_SECRET.to_owned()),
            ("duid", DUID.to_owned()),
            ("scope", SCOPE.to_owned()),
            ("code", grant_code),
            ("grant_type", "authorization_code".to_owned()),
        ]
    }

    fn oauth_token_refresh_encode(&self) -> [(&'static str, String); 7] {
        [
            ("app_context", "inapp_ios".to_owned()),
            ("client_id", CLIENT_ID.to_owned()),
            ("client_secret", CLIENT_SECRET.to_owned()),
            ("duid", DUID.to_owned()),
            ("scope", SCOPE.to_owned()),
            (
                "refresh_token",
                self.refresh_token
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("lazy uncheck")
                    .to_owned(),
            ),
            ("grant_type", "refresh_token".to_owned()),
        ]
    }

    fn profile_encode(&self) -> String {
        format!(
            "{}{}/profile?fields=%40default,relation,requestMessageFlag,presence,%40personalDetail,trophySummary",
            USERS_API,
            self.online_id.as_ref().map(String::as_str).unwrap_or("lazy uncheck")
        )
    }

    fn trophy_summary_encode(&self, offset: u32) -> String {
        format!(
            "{}?fields=%40default&npLanguage={}&iconSize=m&platform=PS3,PSVITA,PS4&offset={}&limit=100&comparedUser={}",
            USER_TROPHY_API,
            self.language.as_ref().map(String::as_str).unwrap(),
            offset,
            self.online_id.as_ref().map(String::as_str).unwrap_or("lazy uncheck")
        )
    }

    fn trophy_set_encode(&self) -> String {
        format!(
            "{}{}/trophyGroups/all/trophies?fields=%40default,trophyRare,trophyEarnedRate&npLanguage={}&comparedUser={}",
            USER_TROPHY_API,
            self.np_communication_id.as_ref().map(String::as_str).unwrap_or("lazy uncheck"),
            self.language.as_ref().map(String::as_str).unwrap(),
            self.online_id.as_ref().map(String::as_str).unwrap_or("lazy uncheck")
        )
    }
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
