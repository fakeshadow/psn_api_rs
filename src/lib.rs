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
//!       PSN::new()
//!            .set_region("us".to_owned()) // <- set to a psn region server suit your case. you can leave it as default which is hk
//!            .set_lang("en".to_owned()) // <- set to a language you want the response to be. default is en
//!            .set_self_online_id(String::from("Your Login account PSN online_id")) // <- this is used to generate new message thread.
//!                                                                    // safe to leave unset if you don't need to send any PSN message.
//!            .add_refresh_token(refresh_token) // <- If refresh_token is provided then it's safe to ignore uuid and two_step arg and call .auth() directly.
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

use futures::{
    future::{ok, Either},
    Future,
};

use derive_more::Display;
use rand::{distributions::Alphanumeric, Rng};
use serde::de::DeserializeOwned;

use crate::models::MessageDetail;

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
const MESSAGE_THREAD_ENTRY: &'static str =
    "-gmsg.np.community.playstation.net/groupMessaging/v1/threads";
const STORE_ENTRY: &'static str = "https://store.playstation.com/valkyrie-api/";

//const ACTIVITY_ENTRY: &'static str = "https://activity.api.np.km.playstation.net/activity/api/";

const CLIENT_ID: &'static str = "b7cbf451-6bb6-4a5a-8913-71e61f462787";
const CLIENT_SECRET: &'static str = "zsISsjmCx85zgCJg";
const DUID: &'static str = "0000000d000400808F4B3AA3301B4945B2E3636E38C0DDFC";
const SCOPE: &'static str = "capone:report_submission,psn:sceapp,user:account.get,user:account.settings.privacy.get,user:account.settings.privacy.update,user:account.realName.get,user:account.realName.update,kamaji:get_account_hash,kamaji:ugc:distributor,oauth:manage_device_usercodes";

/// `models` are used to deserialize psn response json.
/// Some response fields are ignored so if you need more/less fields you can use your own struct as long as it impl `serde::Deserialize`.
pub mod models {
    ///The response type of `get_profile()`
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

    ///The response type of `get_titles()`
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

    ///The response type of `get_trophy_set()`
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

    ///The response type of `get_message_threads()`
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

    ///The response type of `get_message_thread()`
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
        pub push_notification_flag: bool,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct NewArrivalEventDetail {
        pub new_arrival_event_flag: bool,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadEvent {
        pub message_event_detail: MessageEventDetail,
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

    ///The response type of `search_store_items()`
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreSearchResult {
        // skip this field for now
        //        pub data: StoreSearchData,
        pub included: Vec<StoreSearchData>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreSearchData {
        pub attributes: StoreSearchAttribute,
        pub id: String,
        pub relationships: StoreSearchRelationship,
        #[serde(alias = "type")]
        pub typ: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    // what a mess.
    pub struct StoreSearchAttribute {
        #[serde(alias = "badge-info")]
        pub badge_info: BadgeInfo,
        #[serde(alias = "cero-z-status")]
        pub ceroz_status: CeroZStatus,
        #[serde(alias = "content-rating")]
        pub content_rating: ContentRating,
        #[serde(alias = "content-type")]
        pub content_type: String,
        #[serde(alias = "default-sku-id")]
        pub default_sku_id: String,
        #[serde(alias = "dob-required")]
        pub dob_required: bool,
        #[serde(alias = "file-size")]
        pub file_size: FileSize,
        #[serde(alias = "game-content-type")]
        pub game_content_type: String,
        pub genres: Vec<String>,
        #[serde(alias = "is-igc-upsell")]
        pub is_igc_upsell: bool,
        #[serde(alias = "is-multiplayer-upsell")]
        pub is_multiplayer_upsell: bool,
        #[serde(alias = "kamaji-relationship")]
        pub kamaji_relationship: String,
        #[serde(alias = "legal-text")]
        pub large_text: String,
        #[serde(alias = "long-description")]
        pub long_description: String,
        #[serde(alias = "macross-brain-context")]
        pub macross_brain_context: String,
        #[serde(alias = "media-list")]
        pub media_list: MediaList,
        pub name: String,
        #[serde(alias = "nsx-confirm-message")]
        pub nsx_confirm_message: String,
        pub parent: Option<ParentGameInfo>,
        pub platforms: Vec<String>,
        #[serde(alias = "plus-reward-description")]
        pub plus_reward_description: Option<String>,
        #[serde(alias = "primary-classification")]
        pub primary_classification: String,
        #[serde(alias = "secondary-classification")]
        pub secondary_classification: String,
        #[serde(alias = "provider-name")]
        pub provider_name: String,
        #[serde(alias = "ps-camera-compatibility")]
        pub ps_camera_compatibility: String,
        #[serde(alias = "ps-move-compatibility")]
        pub ps_move_compatibility: String,
        #[serde(alias = "ps-vr-compatibility")]
        pub ps_vr_compatibility: String,
        #[serde(alias = "release-date")]
        pub release_date: String,
        pub skus: Vec<Sku>,
        #[serde(alias = "star-rating")]
        pub star_rating: StarRating,
        #[serde(alias = "subtitle-language-codes")]
        // ToDo: this field could be an option with other type
        pub subtitle_language_codes: Vec<String>,
        #[serde(alias = "tertiary-classification")]
        pub tertiary_classification: String,
        #[serde(alias = "thumbnail-url-base")]
        pub thumbnail_url_base: String,
        #[serde(alias = "top-category")]
        pub top_category: String,
        #[serde(alias = "upsell-info")]
        // ToDo: this field could be an option with other type
        pub upsell_info: Option<String>,
        #[serde(alias = "voice-language-codes")]
        // ToDo: this field could be an option with other type
        pub voice_language_codes: Vec<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct BadgeInfo {
        #[serde(alias = "non-plus-user")]
        pub non_plus_user: Option<BadgeInfoData>,
        #[serde(alias = "plus-user")]
        pub plus_user: Option<BadgeInfoData>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct BadgeInfoData {
        #[serde(alias = "discount-percentage")]
        pub discount_percentage: u8,
        #[serde(alias = "is-plus")]
        pub is_plus: bool,
        #[serde(alias = "type")]
        pub typ: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct CeroZStatus {
        #[serde(alias = "is-allowed-in-cart")]
        pub is_allowed_in_cart: bool,
        #[serde(alias = "is-on")]
        pub is_on: bool,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ContentRating {
        #[serde(alias = "content-descriptors")]
        pub content_descriptors: Vec<ContentDescriptor>,
        pub content_interactive_element: Vec<ContentInteractiveElement>,
        #[serde(alias = "rating-system")]
        pub rating_system: String,
        pub url: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ContentDescriptor {
        pub description: String,
        pub name: String,
        pub url: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ContentInteractiveElement {
        pub description: String,
        pub name: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct FileSize {
        pub unit: String,
        pub value: f32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct MediaList {
        pub preview: Vec<Link>,
        pub promo: Promo,
        pub screenshots: Vec<Link>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Promo {
        pub images: Vec<Link>,
        pub videos: Vec<Link>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Link {
        pub url: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ParentGameInfo {
        pub id: String,
        pub name: String,
        pub thumbnail: String,
        pub url: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Sku {
        pub entitlements: Vec<Entitlement>,
        pub id: String,
        #[serde(alias = "is-preorder")]
        pub is_preorder: bool,
        //ToDo: could be other type.
        pub multibuy: Option<String>,
        pub name: String,
        #[serde(alias = "playability-date")]
        pub playability_date: String,
        #[serde(alias = "plus-reward-description")]
        pub plus_reward_description: Option<String>,
        pub prices: Price,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Entitlement {
        pub duration: u32,
        #[serde(alias = "exp-after-first-use")]
        pub exp_after_first_use: u32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Price {
        #[serde(alias = "non-plus-user")]
        pub non_plus_user: PriceData,
        #[serde(alias = "plus-user")]
        pub plus_user: PriceData,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct PriceData {
        #[serde(alias = "actual-price")]
        pub actual_price: PriceDisplayValue,
        pub availability: StartEndDate,
        #[serde(alias = "discount-percentage")]
        pub discount_percentage: u8,
        #[serde(alias = "is-plus")]
        pub is_plus: bool,
        #[serde(alias = "strikethrough-price")]
        pub strikethrough_price: Option<PriceDisplayValue>,
        #[serde(alias = "upsell-price")]
        pub upsell_price: Option<PriceDisplayValue>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct PriceDisplayValue {
        pub display: String,
        pub value: u16,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct StartEndDate {
        #[serde(alias = "end-date")]
        pub end_date: Option<String>,
        #[serde(alias = "start-date")]
        pub start_date: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct StarRating {
        pub score: f32,
        pub total: u32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreSearchRelationship {
        pub children: StoreSearchRelationshipChildren,
        #[serde(alias = "legacy-skus")]
        pub legacy_skus: StoreSearchRelationshipLegacySkus,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreSearchRelationshipChildren {
        pub data: Vec<StoreSearchRelationshipData>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreSearchRelationshipLegacySkus {
        pub data: Vec<StoreSearchRelationshipData>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreSearchRelationshipData {
        pub id: String,
        #[serde(alias = "type")]
        pub typ: String,
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
    #[display(fmt = "Error from PSN response: {}", _0)]
    FromPSN(String),
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
    Self: Sized + EncodeUrl + MultiPart + 'static,
{
    type Error;

    fn auth(self) -> Box<dyn Future<Item = PSN, Error = Self::Error>> {
        Box::new(
            self.gen_access_from_refresh()
                .or_else(|(_, p)| p.gen_access_and_refresh())
                .map_err(|(e, _)| e),
        )
    }

    /// This method will use `uuid` and `two_step` to get a new pair of access_token and refresh_token from PSN.
    fn gen_access_and_refresh(self) -> Box<dyn Future<Item = PSN, Error = (Self::Error, Self)>>;

    /// This method will use local `refresh_token` to get a new `access_token` from PSN.
    fn gen_access_from_refresh(self) -> Box<dyn Future<Item = PSN, Error = (Self::Error, Self)>>;

    /// A generic http get handle function. The return type `T` need to impl `serde::deserialize`.
    fn get_by_url_encode<T: DeserializeOwned + 'static>(
        &self,
        url: &str,
    ) -> Box<dyn Future<Item = T, Error = Self::Error>>;

    /// A generic http del handle function. return status 204 as successful response.
    fn del_by_url_encode(&self, url: &str) -> Box<dyn Future<Item = (), Error = Self::Error>>;

    /// A generic multipart/form-data post handle function.
    /// take in multipart boundary to produce a proper heaader.
    fn post_by_multipart(
        &self,
        boundary: &str,
        url: &str,
        body: Vec<u8>,
    ) -> Box<dyn Future<Item = (), Error = Self::Error>>;

    /// need to `add_online_id` before call this method.
    /// ```rust
    /// PSN::new().add_online_id(String::from("123")).get_rofile()
    /// ```
    fn get_profile<T>(&self) -> Box<dyn Future<Item = T, Error = Self::Error>>
    where
        T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.profile_encode().as_str())
    }

    /// need to `add_online_id` and give a legit `offset`(offset can't be larger than the total trophy lists a user have).
    /// ```rust
    /// PSN::new().add_online_id(String::from("123")).get_titles(0)
    /// ```
    fn get_titles<T>(&self, offset: u32) -> Box<dyn Future<Item = T, Error = Self::Error>>
    where
        T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.trophy_summary_encode(offset).as_str())
    }

    /// need to `add_online_id` and `add_np_communication_id` before call this method.
    /// ```rust
    /// PSN::new().add_online_id(String::from("123")).add_np_communication_id(String::from("NPWR00233")).get_trophy_set()
    /// ```
    fn get_trophy_set<T>(&self) -> Box<dyn Future<Item = T, Error = Self::Error>>
    where
        T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.trophy_set_encode().as_str())
    }

    /// return message threads of the account you used to login PSN network.
    /// `offset` can't be large than all existing threads count.
    fn get_message_threads<T>(&self, offset: u32) -> Box<dyn Future<Item = T, Error = Self::Error>>
    where
        T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(self.message_threads_encode(offset).as_str())
    }

    /// return message thread detail of the `ThreadId`.
    fn get_message_thread<T>(
        &self,
        thread_id: &str,
    ) -> Box<dyn Future<Item = T, Error = Self::Error>>
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
    fn generate_message_thread(&self) -> Box<dyn Future<Item = (), Error = Self::Error>> {
        let boundary = Self::generate_boundary();
        let body = self.message_multipart_body(boundary.as_str(), None, None);

        self.post_by_multipart(
            boundary.as_str(),
            self.generate_thread_encode().as_str(),
            body,
        )
    }

    fn leave_message_thread(
        &self,
        thread_id: &str,
    ) -> Box<dyn Future<Item = (), Error = Self::Error>> {
        self.del_by_url_encode(self.leave_message_thread_encode(thread_id).as_str())
    }

    /// You can only send message to an existing message thread. So if you want to send to some online_id the first thing is generating a new message thread.
    /// Pass none if you don't want to send text or image file (Pass both as none will result in an error)
    fn send_message(
        &self,
        msg: Option<&str>,
        path: Option<&str>,
        thread_id: &str,
    ) -> Box<dyn Future<Item = (), Error = Self::Error>> {
        let boundary = Self::generate_boundary();
        let url = self.send_message_encode(thread_id);
        let body = self.message_multipart_body(boundary.as_str(), msg, path);

        self.post_by_multipart(boundary.as_str(), url.as_str(), body)
    }

    fn search_store_items<T>(
        &self,
        lang: &str,
        region: &str,
        age: &str,
        name: &str,
    ) -> Box<dyn Future<Item = T, Error = Self::Error>>
    where
        T: DeserializeOwned + 'static,
    {
        self.get_by_url_encode(Self::store_search_encode(lang, region, age, name).as_str())
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
    fn gen_access_and_refresh(self) -> Box<dyn Future<Item = PSN, Error = (PSNError, PSN)>> {
        Box::new(
            // User uuid and two_step code to make a post call.
            PSN::http_client()
                .post(urls::NP_SSO_ENTRY)
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
                    // Extract the "x-np-grant-code" from the response header and parse it to string.
                    match res.headers().get("x-np-grant-code") {
                        Some(h) => Ok((h.to_str().unwrap().to_owned(), psn)),
                        None => Err((PSNError::NoGrantCode, psn)),
                    }
                })
                .and_then(|(grant, psn)| {
                    // Use the grant code to make another post call to finish the authentication process.
                    PSN::http_client()
                        .post(urls::OAUTH_TOKEN_ENTRY)
                        .send_form(&PSN::oauth_token_encode(grant))
                        .then(|r| match r {
                            Ok(r) => Ok((r, psn)),
                            Err(_) => Err((PSNError::NetWork, psn)),
                        })
                })
                .and_then(|(mut res, mut psn)| {
                    // Extract the access_token and refresh_token from the response body json.
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
            // Basically the same process as the last step of gen_access_and_refresh method with a slightly different url encode.
            // We only need the new access token from response.(refresh token can't be refreshed.)
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

    fn get_by_url_encode<T>(&self, url: &str) -> Box<dyn Future<Item = T, Error = PSNError>>
    where
        T: DeserializeOwned + 'static,
    {
        Box::new(
            // The access_token is used as bearer token and content type header need to be application/json.
            PSN::http_client()
                .get(url)
                .header(awc::http::header::CONTENT_TYPE, "application/json")
                .bearer_auth(
                    self.access_token
                        .as_ref()
                        .map(String::as_str)
                        .unwrap_or("incase"),
                )
                .send()
                .map_err(|_| PSNError::NetWork)
                .and_then(|mut res| {
                    if res.status() != 200 {
                        return Either::A(res.json().map_err(|_| PSNError::PayLoad).and_then(
                            |e: PSNResponseError| Err(PSNError::FromPSN(e.error.message)),
                        ));
                    }
                    Either::B(res.json().limit(6553500).map_err(|_| PSNError::PayLoad))
                }),
        )
    }

    fn del_by_url_encode(&self, url: &str) -> Box<dyn Future<Item = (), Error = PSNError>> {
        Box::new(
            PSN::http_client()
                .delete(url)
                .bearer_auth(self.access_token.as_ref().unwrap())
                .send()
                .map_err(|_| PSNError::NetWork)
                .and_then(|mut res| {
                    if res.status() != 204 {
                        return Either::A(res.json().map_err(|_| PSNError::PayLoad).and_then(
                            |e: PSNResponseError| Err(PSNError::FromPSN(e.error.message)),
                        ));
                    }
                    Either::B(ok(()))
                }),
        )
    }

    fn post_by_multipart(
        &self,
        boundary: &str,
        url: &str,
        body: Vec<u8>,
    ) -> Box<dyn Future<Item = (), Error = PSNError>> {
        Box::new(
            // The access_token is used as bearer token and content type header need to be multipart/form-data.
            PSN::http_client()
                .post(url)
                .header(
                    awc::http::header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .bearer_auth(self.access_token.as_ref().unwrap())
                .send_body(body)
                .map_err(|_| PSNError::NetWork)
                .and_then(|mut res| {
                    if res.status() != 200 {
                        return Either::A(res.json().map_err(|_| PSNError::PayLoad).and_then(
                            |e: PSNResponseError| Err(PSNError::FromPSN(e.error.message)),
                        ));
                    }
                    Either::B(ok(()))
                }),
        )
    }
}

/// serde_urlencoded can be used to make a `application/x-wwww-url-encoded` `String` buffer from form
/// it applies to `EncodeUrl` methods return a slice type.
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

    fn leave_message_thread_encode(&self, thread_id: &str) -> String;

    fn send_message_encode(&self, thread_id: &str) -> String;

    fn store_search_encode(lang: &str, region: &str, age: &str, name: &str) -> String {
        let name = name.replace(" ", "+");

        format!(
            "{}{}/{}/{}/tumbler-search/{}?suggested_size=999&mode=game",
            STORE_ENTRY, lang, region, age, name
        )
    }

    fn store_item_encode(lang: &str, region: &str, age: &str, game_id: &str) -> String {
        format!(
            "{}{}/{}/{}/resolve/{}",
            STORE_ENTRY, lang, region, age, game_id
        )
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
        format!(
            "https://{}{}?offset={}",
            self.region.as_str(),
            MESSAGE_THREAD_ENTRY,
            offset
        )
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

    fn leave_message_thread_encode(&self, thread_id: &str) -> String {
        format!(
            "https://{}{}/{}/users/me",
            self.region.as_str(),
            MESSAGE_THREAD_ENTRY,
            thread_id
        )
    }

    fn send_message_encode(&self, thread_id: &str) -> String {
        format!(
            "https://{}{}/{}/messages",
            self.region.as_str(),
            MESSAGE_THREAD_ENTRY,
            thread_id
        )
    }
}

pub trait MultiPart {
    /// take `option<&str>` for `message` and `file path` to determine if the message is a text only or a image attached one.
    /// pass both as `None` will result in generating a new message thread body.
    fn message_multipart_body(
        &self,
        boundary: &str,
        message: Option<&str>,
        file_path: Option<&str>,
    ) -> Vec<u8>;

    fn generate_boundary() -> String {
        let mut boundary = String::with_capacity(50);
        boundary.push_str("--------------------------");

        let s: String = rand::thread_rng()
            .sample_iter(Alphanumeric)
            .take(24)
            .collect();

        boundary.push_str(s.as_str());

        boundary
    }
}

impl MultiPart for PSN {
    fn message_multipart_body(
        &self,
        boundary: &str,
        msg: Option<&str>,
        path: Option<&str>,
    ) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();

        if msg.is_none() && path.is_none() {
            let msg = serde_json::to_string(&GenerateNewThread::new(
                self.online_id.as_ref().unwrap(),
                self.self_online_id.as_ref(),
            ))
            .unwrap_or("".to_owned());

            write_string(&mut result, boundary, "threadDetail", msg.as_str());
            return result;
        };

        let event_category = if path.is_some() { 3u8 } else { 1 };
        let msg =
            serde_json::to_string(&SendMessage::new(msg, event_category)).unwrap_or("".to_owned());

        write_string(&mut result, boundary, "messageEventDetail", msg.as_str());

        if let Some(path) = path {
            use std::io::prelude::*;

            //ToDo: handle error and write file async.
            let mut f = std::fs::File::open(path).unwrap();
            let mut file_data = Vec::new();
            f.read_to_end(&mut file_data).unwrap();

            result.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"imageData\"\r\n").as_bytes(),
            );
            result.extend_from_slice("Content-Type: image/png\r\n".as_bytes());
            result.extend_from_slice(
                format!("Content-Length: {}\r\n\r\n", file_data.len()).as_bytes(),
            );
            result.append(&mut file_data);
            result.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());
        }

        result
    }
}

fn write_string(result: &mut Vec<u8>, boundary: &str, name: &str, msg: &str) {
    result.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    result.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes(),
    );
    result.extend_from_slice("Content-Type: application/json; charset=utf-8\r\n\r\n".as_bytes());
    result.extend_from_slice(format!("{}\r\n", msg).as_bytes());
    result.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PSNResponseError {
    error: PSNResponseErrorInner,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PSNResponseErrorInner {
    code: u32,
    message: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SendMessage {
    message_event_detail: SendMessageEventDetail,
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
                    body: Some(body.unwrap_or("").to_owned()),
                },
            },
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
    thread_members: Vec<NewThreadMember<'a>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct NewThreadMember<'a> {
    online_id: &'a str,
}

impl<'a> GenerateNewThread<'a> {
    fn new(other_id: &'a str, self_id: &'a str) -> Self {
        GenerateNewThread {
            thread_detail: NewThreadMembers {
                thread_members: vec![
                    NewThreadMember {
                        online_id: other_id,
                    },
                    NewThreadMember { online_id: self_id },
                ],
            },
        }
    }
}
