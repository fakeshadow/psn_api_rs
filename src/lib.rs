//! # A simple PSN API wrapper.
//! It uses an async http client(actix-web-client in this case) to communicate wih the official PSN API.
//!
//! Some basics:
//! The crate use a pair of `uuid` and `two_step` code to login in to PSN Network and get a pair of `access_token` and `refresh_token` in response.
//! The `access_token` last about an hour before expire and it's needed to call most other PSN APIs(The PSN store API doesn't need any token to access though).
//! The `refresh_token` last much longer and it's used to generate a new access_token after/before it is expired.
//! * Some thing to note:
//! There is a rate limiter for the official PSN API so better not make lots of calls in short time.
//! Therefore its' best to avoid using in multi threads also as a single thread could hit the limit easily on any given machine running this crate.
//!
//! # Example:
//!``` rust
//!use futures::lazy;
//!
//!use tokio::runtime::current_thread::Runtime;
//!use psn_api_rs::{PSNRequest, PSN, models::PSNUser};
//!
//!fn main() {
//!    let refresh_token = String::from("your refresh token");
//!    let uuid = String::from("your uuid");
//!    let two_step = String::from("your two_step code");
//!
//!    let mut runtime = Runtime::new().unwrap();
//!
//!    // construct a PSN struct,add credentials and call auth to generate tokens.
//!    let mut psn: PSN = runtime.block_on(lazy(|| {
//!        PSN::new()
//!            .add_refresh_token(refresh_token)   // <- If refresh_token is provided then it's safe to ignore uuid and two_step arg and call .auth() directly.
//!            .add_uuid(uuid) // <- uuid and two_step are used only when refresh_token is not working or not provided.
//!            .add_two_step(two_step)
//!            .auth()
//!    })).unwrap_or_else(|e| panic!("{:?}", e));
//!
//!    println!(
//!        "Authentication Success! These are your token info from PSN network: \r\n{:#?} ",
//!        psn
//!    );
//!
//!    let user: PSNUser = runtime.block_on(
//!        psn.add_online_id("Hakoom".to_owned()).get_profile()  // <- use the psn struct to call for user_profile.
//!    ).unwrap_or_else(|e| panic!("{:?}", e));
//!
//!    println!(
//!        "Example finished. Got user info : \r\n{:#?}",
//!        user
//!    );
//!
//!    // psn struct is dropped at this point so it's better to store your access_token and refresh_token here to make them reusable.
//!}
//!```

use std::time::{Duration, Instant};

use futures::Future;

use derive_more::Display;
use serde::de::DeserializeOwned;
use crate::models::MessageDetail;

#[cfg(feature = "awc")]
#[macro_use]
extern crate serde_derive;

/// `urls` are hard coded for PSN authentication which are used if you want to impl your own http client.
pub mod urls {
    /// grant code entry is generate with this pattern
    /// ```rust
    /// format!("https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/authorize?duid={}&app_context=inapp_ios&client_id={}&scope={}&response_type=code", DUID, CLIENT_ID, SCOPE);
    /// ```
    pub const GRANT_CODE_ENTRY: &'static str =
        "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/authorize?duid=0000000d000400808F4B3AA3301B4945B2E3636E38C0DDFC&app_context=inapp_ios&client_id=b7cbf451-6bb6-4a5a-8913-71e61f462787&scope=capone:report_submission,psn:sceapp,user:account.get,user:account.settings.privacy.get,user:account.settings.privacy.update,user:account.realName.get,user:account.realName.update,kamaji:get_account_hash,kamaji:ugc:distributor,oauth:manage_device_usercodes&response_type=code";

    pub const NP_SSO_ENTRY: &'static str =
        "https://auth.api.sonyentertainmentnetwork.com/2.0/ssocookie";

    pub const OAUTH_TOKEN_ENTRY: &'static str =
        "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/token";
}

const USERS_ENTRY: &'static str = "-prof.np.community.playstation.net/userProfile/v1/users/";
const USER_TROPHY_ENTRY: &'static str = "-tpy.np.community.playstation.net/trophy/v1/trophyTitles/";
const MESSAGE_THREAD_ENTRY: &'static str = "-gmsg.np.community.playstation.net/groupMessaging/v1/threads";

//const ACTIVITY_API: &'static str = "https://activity.api.np.km.playstation.net/activity/api/";

//const STORE_API: &'static str = "https://store.playstation.com/valkyrie-api/";

const CLIENT_ID: &'static str = "b7cbf451-6bb6-4a5a-8913-71e61f462787";
const CLIENT_SECRET: &'static str = "zsISsjmCx85zgCJg";
const DUID: &'static str = "0000000d000400808F4B3AA3301B4945B2E3636E38C0DDFC";
const SCOPE: &'static str = "capone:report_submission,psn:sceapp,user:account.get,user:account.settings.privacy.get,user:account.settings.privacy.update,user:account.realName.get,user:account.realName.update,kamaji:get_account_hash,kamaji:ugc:distributor,oauth:manage_device_usercodes";

/// `models` are used to deserialize psn response json.
/// Some response fields are ignored so if you need more/less fields you can use your own struct as long as it impl `serde::Deserialize`.
pub mod models {
    #[derive(Serialize, Deserialize, Debug)]
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

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct PSNUserTrophySummary {
        pub level: u8,
        pub progress: u8,
        pub earned_trophies: EarnedTrophies,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TrophyTitles {
        pub total_results: u32,
        pub offset: u32,
        pub trophy_titles: Vec<TrophyTitle>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TrophyTitle {
        pub np_communication_id: String,
        pub trophy_title_name: String,
        pub trophy_title_detail: String,
        pub trophy_title_icon_url: String,
        pub trophy_title_platfrom: String,
        pub has_trophy_groups: bool,
        pub defined_trophies: EarnedTrophies,
        #[serde(alias = "comparedUser")]
        pub title_detail: TitleDetail,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TitleDetail {
        pub progress: u8,
        pub earned_trophies: EarnedTrophies,
        pub last_update_date: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct EarnedTrophies {
        pub platinum: u32,
        pub gold: u32,
        pub silver: u32,
        pub bronze: u32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TrophySet {
        pub trophies: Vec<Trophy>,
    }

    /// If one trophy is hidden and the account you use to login PSN has not obtained it,
    /// all the `Option<String>` fields will return `None`.
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Trophy {
        pub trophy_id: u8,
        pub trophy_hidden: bool,
        pub trophy_type: Option<String>,
        pub trophy_name: Option<String>,
        pub trophy_detail: Option<String>,
        pub trophy_icon_url: Option<String>,
        pub trophy_rare: u8,
        pub trophy_earned_rate: String,
        #[serde(alias = "comparedUser")]
        pub user_info: TrophyUser,
    }

    /// `earned_date` field will return `None` if this has not been earned by according `online_id`.
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TrophyUser {
        pub online_id: String,
        pub earned: bool,
        pub earned_date: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct MessageThreadsSummary {
        pub threads: Vec<MessageThreadSummary>,
        pub start: u32,
        pub size: u32,
        pub total_size: u32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct MessageThreadSummary {
        pub thread_id: String,
        pub thread_type: u8,
        pub thread_modified_date: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct MessageThread {
        pub thread_members: Vec<ThreadMember>,
        pub thread_name_detail: ThreadName,
        pub thread_thumbnail_detail: ThreadThumbnail,
        pub thread_property: ThreadProperty,
        pub new_arrival_event_detail: NewArrivalEventDetail,
        pub thread_events: Vec<ThreadEvent>,
        pub thread_id: String,
        pub thread_type: u8,
        pub thread_modified_date: String,
        pub results_count: u32,
        pub max_event_index_cursor: String,
        pub since_event_index_cursor: String,
        pub latest_event_index: String,
        pub end_of_thread_event: bool,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadMember {
        pub account_id: String,
        pub online_id: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadName {
        pub status: u8,
        pub thread_name: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadThumbnail {
        pub status: u8,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadProperty {
        pub favorite_detail: FavoriteDetail,
        pub notification_detail: NotificationDetail,
        pub kickout_flag: bool,
        pub thread_join_date: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct FavoriteDetail {
        pub favorite_flag: bool,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct NotificationDetail {
        pub push_notification_flag: bool
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct NewArrivalEventDetail {
        pub new_arrival_event_flag: bool
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadEvent {
        pub message_event_detail: MessageEventDetail
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct MessageEventDetail {
        pub event_index: String,
        pub post_date: String,
        pub event_category_code: u32,
        pub alt_event_category_code: u32,
        pub sender: ThreadMember,
        pub attached_media_path: Option<String>,
        pub message_detail: MessageDetail,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct MessageDetail {
        pub body: Option<String>,
    }
}

#[derive(Debug)]
pub struct PSN {
    region: String,
    self_online_id: String,
    access_token: Option<String>,
    uuid: Option<String>,
    two_step: Option<String>,
    refresh_token: Option<String>,
    last_refresh_at: Option<Instant>,
    online_id: Option<String>,
    np_id: Option<String>,
    np_communication_id: Option<String>,
    language: String,
}

/// You can override `PSNRequest` trait to impl your own error type.
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
    #[display(fmt = "Can not post data to PSN APIs")]
    PostData,
}

impl PSN {
    pub fn new() -> Self {
        PSN {
            region: "hk".to_owned(),
            self_online_id: "".to_owned(),
            access_token: None,
            uuid: None,
            two_step: None,
            refresh_token: None,
            last_refresh_at: None,
            online_id: None,
            np_id: None,
            np_communication_id: None,
            language: "en".to_owned(),
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

    /// `region` is the psn server region setting. default is `hk`(Hang Kong). You can change to `us`(USA),`jp`(Japan), etc
    pub fn set_region(mut self, region: String) -> Self {
        self.region = region;
        self
    }

    /// default language is English.
    pub fn set_lang(mut self, lang: String) -> Self {
        self.language = lang;
        self
    }

    /// This is your login in account's online id. default is "".
    /// This field is used to generate new message thread.
    pub fn set_self_online_id(mut self, id: String) -> Self {
        self.self_online_id = id;
        self
    }

    /// `online_id` is the displayed online_id of PSN user which used to query user's info.
    pub fn add_online_id(&mut self, online_id: String) -> &mut Self {
        self.online_id = Some(online_id);
        self
    }

    /// `np_id` is PSN user's real unique id as online_id can be changed so it's best to use this as user identifier.
    pub fn add_np_id(&mut self, np_id: String) -> &mut Self {
        self.np_id = Some(np_id);
        self
    }

    /// `np_communication_id` is PSN game's identifier which be obtained by getting user's game summary API.(Only the games the target user have played will return)
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

/// You can override `PSNRequest` trait to impl your preferred http client
///
/// The crate can provide the url, body format and some headers needed but the response handling you have to write your own.
pub trait PSNRequest
    where
        Self: Sized + EncodeUrl + 'static,
{
    type Error;

    fn auth(self) -> Box<dyn Future<Item=PSN, Error=Self::Error>> {
        Box::new(
            self.gen_access_from_refresh()
                .or_else(|(_, p)| p.gen_access_and_refresh())
                .map_err(|(e, _)| e),
        )
    }

    /// This method will use `uuid` and `two_step` to get a new pair of access_token and refresh_token from PSN.
    fn gen_access_and_refresh(self) -> Box<dyn Future<Item=PSN, Error=(Self::Error, Self)>>;

    /// This method will use local `refresh_token` to get a new `access_token` from PSN.
    fn gen_access_from_refresh(self) -> Box<dyn Future<Item=PSN, Error=(Self::Error, Self)>>;

    /// A generic http get handle function. The return type `T` need to impl `serde::deserialize`.
    fn get_by_url_encode<T: DeserializeOwned + 'static>(
        &self,
        url: &str,
    ) -> Box<dyn Future<Item=T, Error=Self::Error>>;

    /// A generic multipart/form-data post handle function.
    /// take in multipart boundary to produce a proper heaader.
    fn post_by_multipart(&self, boundary: &str, url: &str, body: Vec<u8>) -> Box<dyn Future<Item=(), Error=Self::Error>>;

    /// need to `add_online_id` before call this method.
    /// ```rust
    /// PSN::new().add_online_id(String::from("123")).get_rofile()
    /// ```
    fn get_profile<T>(&self) -> Box<dyn Future<Item=T, Error=Self::Error>>
        where
            T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.profile_encode().as_str())
    }

    /// need to `add_online_id` and give a legit `offset`(offset can't be larger than the total trophy lists a user have).
    /// ```rust
    /// PSN::new().add_online_id(String::from("123")).get_titles(0)
    /// ```
    fn get_titles<T>(&self, offset: u32) -> Box<dyn Future<Item=T, Error=Self::Error>>
        where
            T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.trophy_summary_encode(offset).as_str())
    }

    /// need to `add_online_id` and `add_np_communication_id` before call this method.
    /// ```rust
    /// PSN::new().add_online_id(String::from("123")).add_np_communication_id(String::from("NPWR00233")).get_trophy_set()
    /// ```
    fn get_trophy_set<T>(&self) -> Box<dyn Future<Item=T, Error=Self::Error>>
        where
            T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.trophy_set_encode().as_str())
    }

    /// return message threads of the account you used to login PSN network.
    /// `offset` can't be large than all existing threads count.
    fn get_message_threads<T>(&self, offset: u32) -> Box<dyn Future<Item=T, Error=Self::Error>>
        where
            T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.message_threads_encode(offset).as_str())
    }

    /// return message thread detail of the `ThreadId`.
    fn get_message_thread<T>(&self, thread_id: &str) -> Box<dyn Future<Item=T, Error=Self::Error>>
        where
            T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.message_thread_encode(thread_id).as_str())
    }

    /// need to `add_online_id` and `set_self_online_id` before call this method.
    /// Note that `set_self_online_id` take mut self while `add_online_id` take a &mut self. So `set_self_online_id` should be use as the first arg here.
    /// ```rust
    /// PSN::new().set_self_online_id(String::from("NPWR00233")).add_online_id(String::from("123")).generate_message_thread()
    /// ```
    fn generate_message_thread(&self) -> Box<dyn Future<Item=(), Error=Self::Error>> {
        let boundary = Self::generate_boundary();
        let body = self.message_multipart_body(boundary.as_str(), None, None);

        self.post_by_multipart(boundary.as_str(), self.generate_thread_encode().as_str(), body)
    }

    fn send_text_message(&self, msg: &str, thread_id: &str) -> Box<dyn Future<Item=(), Error=Self::Error>> {
        let boundary = Self::generate_boundary();
        let url = self.send_message_encode(thread_id);
        let body = self.message_multipart_body(boundary.as_str(), Some(msg), None);

        self.post_by_multipart(boundary.as_str(), url.as_str(), body)
    }
}

#[cfg(feature = "awc")]
impl PSN {
    /// default http client
    fn http_client() -> awc::Client {
        awc::Client::build()
            .connector(
                awc::Connector::new()
                    .timeout(Duration::from_secs(10))
                    .finish(),
            )
            .timeout(Duration::from_secs(10))
            .finish()
    }
}

#[cfg(feature = "awc")]
impl PSNRequest for PSN {
    type Error = PSNError;
    fn gen_access_and_refresh(self) -> Box<dyn Future<Item=PSN, Error=(PSNError, PSN)>> {
        Box::new(
            /// User uuid and two_step code to make a post call.
            PSN::http_client()
                .post(urls::NP_SSO_ENTRY)
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
                        .get(urls::GRANT_CODE_ENTRY)
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
                        .post(urls::OAUTH_TOKEN_ENTRY)
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

    fn gen_access_from_refresh(self) -> Box<dyn Future<Item=PSN, Error=(PSNError, PSN)>> {
        Box::new(
            /// Basically the same process as the last step of gen_access_and_refresh method with a slightly different url encode.
            /// We only need the new access token from response.(refresh token can't be refreshed.)
            PSN::http_client()
                .post(urls::OAUTH_TOKEN_ENTRY)
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

    fn get_by_url_encode<T>(&self, url: &str) -> Box<dyn Future<Item=T, Error=PSNError>>
        where
            T: DeserializeOwned + 'static,
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

    fn post_by_multipart(&self, boundary: &str, url: &str, body: Vec<u8>) -> Box<dyn Future<Item=(), Error=PSNError>> {
        Box::new(
            /// The access_token is used as bearer token and content type header need to be application/json.
            PSN::http_client()
                .post(url)
                .header(awc::http::header::CONTENT_TYPE, format!("multipart/form-data; boundary=------------------------{}", boundary))
                .bearer_auth(self.access_token.as_ref().unwrap())
                .send_body(body)
                .map_err(|e| PSNError::NetWork)
                .and_then(|res| {
                    if res.status() != 200 {
                        return Err(PSNError::PostData);
                    }
                    Ok(())
                })
        )
    }
}

/// serde_urlencoded can be used to make a `application/x-wwww-url-encoded` `String` buffer from form
/// it applies to all `EncodeUrl` methods.
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
pub trait EncodeUrl {
    fn np_sso_url_encode(&self) -> [(&'static str, String); 4];

    fn oauth_token_encode(grant_code: String) -> [(&'static str, String); 6];

    fn oauth_token_refresh_encode(&self) -> [(&'static str, String); 7];

    fn profile_encode(&self) -> String;

    fn trophy_summary_encode(&self, offset: u32) -> String;

    fn trophy_set_encode(&self) -> String;

    fn message_threads_encode(&self, offset: u32) -> String;

    fn message_thread_encode(&self, thread_id: &str) -> String;

    fn generate_thread_encode(&self) -> String;

    fn send_message_encode(&self, thread_id: &str) -> String;

    /// take `option<&str>` for `message` and `file path` to determine if the message is a text only or a image one.
    /// pass both as `None` will result in generating a new message thread body.
    fn message_multipart_body(&self, boundary: &str, message: Option<&str>, file_path: Option<&str>) -> Vec<u8>;

    //ToDo: using dummy boundary for now. should generate rng ones.
    fn generate_boundary() -> String {
        "ea3bbcf87c101233".to_owned()
    }
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
            "https://{}{}{}/profile?fields=%40default,relation,requestMessageFlag,presence,%40personalDetail,trophySummary",
            self.region.as_str(),
            USERS_ENTRY,
            self.online_id.as_ref().map(String::as_str).unwrap_or("lazy uncheck")
        )
    }

    fn trophy_summary_encode(&self, offset: u32) -> String {
        format!(
            "https://{}{}?fields=%40default&npLanguage={}&iconSize=m&platform=PS3,PSVITA,PS4&offset={}&limit=100&comparedUser={}",
            self.region.as_str(),
            USER_TROPHY_ENTRY,
            self.language.as_str(),
            offset,
            self.online_id.as_ref().map(String::as_str).unwrap_or("lazy uncheck")
        )
    }

    fn trophy_set_encode(&self) -> String {
        format!(
            "https://{}{}{}/trophyGroups/all/trophies?fields=%40default,trophyRare,trophyEarnedRate&npLanguage={}&comparedUser={}",
            self.region.as_str(),
            USER_TROPHY_ENTRY,
            self.np_communication_id.as_ref().map(String::as_str).unwrap_or("lazy uncheck"),
            self.language.as_str(),
            self.online_id.as_ref().map(String::as_str).unwrap_or("lazy uncheck")
        )
    }

    fn message_threads_encode(&self, offset: u32) -> String {
        format!("https://{}{}?offset={}", self.region.as_str(), MESSAGE_THREAD_ENTRY, offset)
    }

    fn message_thread_encode(&self, thread_id: &str) -> String {
        format!(
            "https://{}{}/{}?fields=threadMembers,threadNameDetail,threadThumbnailDetail,threadProperty,latestTakedownEventDetail,newArrivalEventDetail,threadEvents&count=100",
            self.region.as_str(),
            MESSAGE_THREAD_ENTRY,
            thread_id
        )
    }

    fn generate_thread_encode(&self) -> String {
        format!("https://{}{}/", self.region.as_str(), MESSAGE_THREAD_ENTRY)
    }

    fn send_message_encode(&self, thread_id: &str) -> String {
        format!("https://{}{}/{}/messages", self.region.as_str(), MESSAGE_THREAD_ENTRY, thread_id)
    }

    fn message_multipart_body(&self, boundary: &str, msg: Option<&str>, path: Option<&str>) -> Vec<u8> {

        let (name, msg) = if msg.is_none() && path.is_none() {
            let msg = serde_json::to_string(&GenerateNewThread::new(
                self.online_id.as_ref().unwrap(),
                self.self_online_id.as_ref())).unwrap_or("".to_owned());

            ("threadDetail", msg)
        } else {
            let event_category = if path.is_some() { 3u8 } else { 1 };
            let msg = serde_json::to_string(&SendMessage::new(msg, event_category)).unwrap_or("".to_owned());

            ("messageEventDetail", msg)
        };

        let mut result: Vec<u8> = Vec::new();

        result.extend_from_slice(format!("--------------------------{}\r\n", boundary).as_bytes());
        result.extend_from_slice(format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes());
        result.extend_from_slice("Content-Type: application/json; charset=utf-8\r\n\r\n".as_bytes());
        result.extend_from_slice(format!("{}\r\n", msg).as_bytes());
        result.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());

//        if let Some(path) = path {
//            use std::io::prelude::*;
//
//            let mut f = std::fs::File::open(path).unwrap();
//            let mut file_data = Vec::new();
//            f.read_to_end(&mut file_data).unwrap();
//
//            result.extend_from_slice(
//                format!("Content-Disposition: form-data; name=\"imageData\"; filename=\"233.png\"\r\n")
//                    .as_bytes(),
//            );
//            result.extend_from_slice("Content-Type: image/png\r\n\r\n".as_bytes());
//            result.extend_from_slice(format!("Content-Length: {}\r\n\r\n", file_data.len()).as_bytes());
//
//            result.append(&mut file_data);
//
//            result.extend_from_slice(format!("--{}--\r\n", "ea3bbcf87c101233").as_bytes());
//        }

        result
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

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SendMessage {
    message_event_detail: SendMessageEventDetail
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SendMessageEventDetail {
    event_category_code: u8,
    message_detail: MessageDetail,
}

impl SendMessage {
    fn new(body: Option<&str>, event_category_code: u8) -> Self {
        SendMessage {
            message_event_detail: SendMessageEventDetail {
                event_category_code,
                message_detail: MessageDetail {
                    body: body.map(|s| s.to_owned())
                },
            }
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GenerateNewThread<'a> {
    thread_detail: NewThreadMembers<'a>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct NewThreadMembers<'a> {
    thread_members: Vec<NewThreadMember<'a>>
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct NewThreadMember<'a> {
    online_id: &'a str
}

impl<'a> GenerateNewThread<'a> {
    fn new(other_id: &'a str, self_id: &'a str) -> Self {
        GenerateNewThread {
            thread_detail: NewThreadMembers {
                thread_members: vec![
                    NewThreadMember {
                        online_id: other_id
                    },
                    NewThreadMember {
                        online_id: self_id
                    }
                ]
            }
        }
    }
}