/// `models` are used to deserialize psn response json.
/// Some response fields are ignored so if you need more/less fields you can use your own struct as long as it impl `serde::Deserialize`.

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

///The response type of `generate_message_thread()`
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessageThreadNew {
    pub thread_id: String,
    pub thread_modified_date: String,
    pub blocked_by_members: bool,
}

///The response type of `send_message()` and `send_message_with_buf()`
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessageThreadResponse {
    pub thread_id: String,
    pub thread_modified_date: String,
    pub event_index: String,
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
    pub badge_info: Option<BadgeInfo>,
    #[serde(alias = "cero-z-status")]
    pub ceroz_status: Option<CeroZStatus>,
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
    pub skus: Option<Vec<Sku>>,
    #[serde(alias = "star-rating")]
    pub star_rating: StarRating,
    #[serde(alias = "subtitle-language-codes")]
    // ToDo: this field could be an option with other type
    pub subtitle_language_codes: Vec<SubtitleLanguageCode>,
    #[serde(alias = "tertiary-classification")]
    pub tertiary_classification: Option<String>,
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
    pub value: Option<f32>,
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
    pub score: Option<f32>,
    pub total: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleLanguageCode {
    pub codes: Vec<String>,
    pub name: String,
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
