/// `meta` are hard coded for PSN authentication which are used if you want to impl your own http client.
pub mod meta {
    pub const OAUTH_TOKEN_ENTRY: &str =
        "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/token";

    pub const USERS_ENTRY: &str = "-prof.np.community.playstation.net/userProfile/v1/users/";
    pub const USER_TROPHY_ENTRY: &str = "-tpy.np.community.playstation.net/trophy/v1/trophyTitles/";
    pub const MESSAGE_THREAD_ENTRY: &str =
        "-gmsg.np.community.playstation.net/groupMessaging/v1/threads";
    pub const STORE_ENTRY: &str = "https://store.playstation.com/valkyrie-api/";
    //const ACTIVITY_ENTRY: &'static str = "https://activity.api.np.km.playstation.net/activity/api/";

    pub const CLIENT_ID: &str = "7c01ce37-cb6b-4938-9c1b-9e36fd5477fa";
    pub const CLIENT_SECRET: &str = "GNumO5QMsagNcO2q";
    pub const DUID: &str = "00000007000801a8000000000000008241fdf6ab09ba863a20202020476f6f676c653a416e64726f696420534400000000000000000000000000000000";
    pub const SCOPE: &str = "kamaji:get_players_met+kamaji:get_account_hash+kamaji:activity_feed_submit_feed_story+kamaji:activity_feed_internal_feed_submit_story+kamaji:activity_feed_get_news_feed+kamaji:communities+kamaji:game_list+kamaji:ugc:distributor+oauth:manage_device_usercodes+psn:sceapp+user:account.profile.get+user:account.attributes.validate+user:account.settings.privacy.get+kamaji:activity_feed_set_feed_privacy+kamaji:satchel+kamaji:satchel_delete+user:account.profile.update+kamaji:url_preview";

    // pub const CLIENT_ID: &str = "b7cbf451-6bb6-4a5a-8913-71e61f462787";
    // pub const CLIENT_SECRET: &str = "zsISsjmCx85zgCJg";
    // pub const DUID: &str = "0000000d000400808F4B3AA3301B4945B2E3636E38C0DDFC";
    // pub const SCOPE: &str = "capone:report_submission,psn:sceapp,user:account.get,user:account.settings.privacy.get,user:account.settings.privacy.update,user:account.realName.get,user:account.realName.update,kamaji:get_account_hash,kamaji:ugc:distributor,oauth:manage_device_usercodes";
}
