use std::future::Future;
use std::pin::Pin;

use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::de::DeserializeOwned;

use crate::metas::meta::*;
use crate::private_model::{GenerateNewThread, SendMessage};
use crate::types::PSNFuture;

/// You can override `PSNRequest` trait to impl your preferred http client
/// The crate can provide the url, body format and some headers needed but the response handling you have to write your own.
/// methods with multiple lifetimes always use args with shorter lifetime than self and the returned future.
pub trait PSNRequest: Sized + Send + Sync + EncodeUrl + 'static {
    type Client: Send + Sync + Clone;
    type Error;

    fn auth(
        mut self,
        client: Self::Client,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + Send>> {
        Box::pin(async move {
            if self.gen_access_and_refresh(&client).await.is_err() {
                self.gen_access_from_refresh(&client).await?;
            }
            Ok(self)
        })
    }

    /// This method will use `uuid` and `two_step` to get a new pair of access_token and refresh_token from PSN.
    fn gen_access_and_refresh<'se>(
        &'se mut self,
        client: &'se Self::Client,
    ) -> PSNFuture<'se, Result<(), Self::Error>>;

    /// This method will use local `refresh_token` to get a new `access_token` from PSN.
    fn gen_access_from_refresh<'se>(
        &'se mut self,
        client: &'se Self::Client,
    ) -> PSNFuture<'se, Result<(), Self::Error>>;

    /// A generic http get handle function. The return type `T` need to impl `serde::deserialize`.
    fn get_by_url_encode<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        url: &'st str,
    ) -> PSNFuture<'se, Result<T, Self::Error>>;

    /// A generic http del handle function. return status 204 as successful response.
    fn del_by_url_encode<'se, 'st: 'se>(
        &'se self,
        client: &'se Self::Client,
        url: &'st str,
    ) -> PSNFuture<'se, Result<(), Self::Error>>;

    /// A generic multipart/form-data post handle function.
    /// take in multipart boundary to produce a proper heaader.
    fn post_by_multipart<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        boundary: &'st str,
        url: &'st str,
        body: Vec<u8>,
    ) -> PSNFuture<'se, Result<T, Self::Error>>;

    fn get_profile<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        online_id: &'st str,
    ) -> PSNFuture<'se, Result<T, Self::Error>> {
        Box::pin(async move {
            let url = self.profile_encode(online_id);
            self.get_by_url_encode(client, url.as_str()).await
        })
    }

    /// need a legit `offset`(offset can't be larger than the total trophy lists a user have).
    fn get_titles<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        online_id: &'st str,
        offset: u32,
    ) -> PSNFuture<'se, Result<T, Self::Error>> {
        Box::pin(async move {
            let url = self.trophy_summary_encode(online_id, offset);
            self.get_by_url_encode(client, url.as_str()).await
        })
    }

    fn get_trophy_set<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        online_id: &'st str,
        np_communication_id: &'st str,
    ) -> PSNFuture<'se, Result<T, Self::Error>> {
        Box::pin(async move {
            let url = self.trophy_set_encode(online_id, np_communication_id);
            self.get_by_url_encode(client, url.as_str()).await
        })
    }

    /// return message threads of the account you used to login PSN network.
    /// `offset` can't be large than all existing threads count.
    fn get_message_threads<'a, T: DeserializeOwned + 'static>(
        &'a self,
        client: &'a Self::Client,
        offset: u32,
    ) -> PSNFuture<Result<T, Self::Error>> {
        Box::pin(async move {
            let url = self.message_threads_encode(offset);
            self.get_by_url_encode(client, url.as_str()).await
        })
    }

    /// return message thread detail of the `ThreadId`.
    fn get_message_thread<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        thread_id: &'st str,
    ) -> PSNFuture<'se, Result<T, Self::Error>> {
        Box::pin(async move {
            let url = self.message_thread_encode(thread_id);
            self.get_by_url_encode(client, url.as_str()).await
        })
    }

    fn generate_message_thread<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        online_id: &'st str,
    ) -> PSNFuture<'se, Result<T, Self::Error>> {
        Box::pin(async move {
            let boundary = Self::generate_boundary();
            let body = self
                .multipart_body(boundary.as_str(), online_id, None, None)
                .await?;
            let url = self.generate_thread_encode();

            self.post_by_multipart(client, boundary.as_str(), url.as_str(), body)
                .await
        })
    }

    fn leave_message_thread<'se, 'st: 'se>(
        &'se self,
        client: &'se Self::Client,
        thread_id: &'st str,
    ) -> PSNFuture<'se, Result<(), Self::Error>> {
        Box::pin(async move {
            let url = self.leave_message_thread_encode(thread_id);
            self.del_by_url_encode(client, url.as_str()).await
        })
    }

    /// You can only send message to an existing message thread. So if you want to send to some online_id the first thing is generating a new message thread.
    /// Pass none if you don't want to send text or image file (Pass both as none will result in an error)
    fn send_message<'se, 'st: 'se>(
        &'se self,
        client: &'se Self::Client,
        online_id: &'st str,
        msg: Option<&'st str>,
        path: Option<&'st str>,
        thread_id: &'st str,
    ) -> PSNFuture<'se, Result<(), Self::Error>> {
        Box::pin(async move {
            let boundary = Self::generate_boundary();
            let url = self.send_message_encode(thread_id);
            let body = self.multipart_body(&boundary, online_id, msg, path).await?;

            self.post_by_multipart(client, boundary.as_str(), url.as_str(), body)
                .await
        })
    }

    fn search_store_items<'se, 'st: 'se, T: DeserializeOwned + 'static>(
        &'se self,
        client: &'se Self::Client,
        lang: &'st str,
        region: &'st str,
        age: &'st str,
        name: &'st str,
    ) -> PSNFuture<'se, Result<T, Self::Error>> {
        Box::pin(async move {
            let url = Self::store_search_encode(lang, region, age, name);
            self.get_by_url_encode(client, url.as_str()).await
        })
    }

    /// take `option<&str>` for `message` and `file path` to determine if the message is a text only or a image attached one.
    /// pass both as `None` will result in generating a new message thread body.
    fn multipart_body<'se, 'st: 'se>(
        &'se self,
        boundary: &'st str,
        online_id: &'st str,
        msg: Option<&'st str>,
        path: Option<&'st str>,
    ) -> PSNFuture<'se, Result<Vec<u8>, Self::Error>> {
        Box::pin(async move {
            let mut result: Vec<u8> = Vec::new();

            if msg.is_none() && path.is_none() {
                let msg = serde_json::to_string(&GenerateNewThread::new(
                    online_id,
                    self.self_online_id(),
                ))
                .unwrap_or_else(|_| "".to_owned());

                write_string(&mut result, boundary, "threadDetail", msg.as_str());
                return Ok(result);
            };

            let event_category = if path.is_some() { 3u8 } else { 1 };
            let msg = serde_json::to_string(&SendMessage::new(msg, event_category))
                .unwrap_or_else(|_| "".to_owned());

            write_string(&mut result, boundary, "messageEventDetail", msg.as_str());

            if let Some(path) = path {
                let file_data = Self::read_path(path).await?;

                result.extend_from_slice(b"Content-Disposition: form-data; name=\"imageData\"\r\n");
                result.extend_from_slice(b"Content-Type: image/png\r\n");

                result.extend_from_slice(
                    format!("Content-Length: {}\r\n\r\n", file_data.len()).as_bytes(),
                );
                // ToDo: in case extend failed
                result.extend_from_slice(&file_data);
                result.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());
            }

            Ok(result)
        })
    }

    /// read local file from path.
    fn read_path(path: &str) -> PSNFuture<Result<Vec<u8>, Self::Error>>;
}

fn write_string(result: &mut Vec<u8>, boundary: &str, name: &str, msg: &str) {
    result.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    result.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes(),
    );
    result.extend_from_slice(b"Content-Type: application/json; charset=utf-8\r\n\r\n");
    result.extend_from_slice(format!("{}\r\n", msg).as_bytes());
    result.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
}

/// serde_urlencoded can be used to make a `application/x-wwww-url-encoded` `String` buffer from form
/// it applies to `EncodeUrl` methods return a slice type.
/// examples if your http client don't support auto urlencode convert.
/// ```ignore
/// use psn_api_rs::{PSN, EncodeUrl};
/// use serde_urlencoded;
/// impl PSN {
///     fn url_query_string(&self) -> String {
///         serde_urlencoded::to_string(&self.np_sso_url_encode()).unwrap()
///     }
/// }
/// ```
pub trait EncodeUrl {
    fn npsso(&self) -> Option<&str>;
    fn access_token(&self) -> Option<&str>;
    fn refresh_token(&self) -> &str;
    fn region(&self) -> &str;
    fn self_online_id(&self) -> &str;
    fn language(&self) -> &str;

    fn oauth_token_encode() -> [(&'static str, &'static str); 4] {
        [
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("scope", SCOPE),
            ("grant_type", "sso_cookie"),
        ]
    }

    fn oauth_token_refresh_encode(&self) -> [(&'static str, &str); 7] {
        [
            ("app_context", "inapp_ios"),
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("duid", DUID),
            ("scope", SCOPE),
            ("refresh_token", self.refresh_token()),
            ("grant_type", "refresh_token"),
        ]
    }

    fn profile_encode(&self, online_id: &str) -> String {
        format!(
            "https://{}{}{}/profile?fields=%40default,relation,requestMessageFlag,presence,%40personalDetail,trophySummary",
            self.region(),
            USERS_ENTRY,
            online_id
        )
    }

    fn trophy_summary_encode(&self, online_id: &str, offset: u32) -> String {
        format!(
            "https://{}{}?fields=%40default&npLanguage={}&iconSize=m&platform=PS3,PSVITA,PS4&offset={}&limit=100&comparedUser={}",
            self.region(),
            USER_TROPHY_ENTRY,
            self.language(),
            offset,
            online_id
        )
    }

    fn trophy_set_encode(&self, online_id: &str, np_communication_id: &str) -> String {
        format!(
            "https://{}{}{}/trophyGroups/all/trophies?fields=%40default,trophyRare,trophyEarnedRate&npLanguage={}&comparedUser={}",
            self.region(),
            USER_TROPHY_ENTRY,
            np_communication_id,
            self.language(),
            online_id
        )
    }

    fn message_threads_encode(&self, offset: u32) -> String {
        format!(
            "https://{}{}?offset={}",
            self.region(),
            MESSAGE_THREAD_ENTRY,
            offset
        )
    }

    fn message_thread_encode(&self, thread_id: &str) -> String {
        format!(
            "https://{}{}/{}?fields=threadMembers,threadNameDetail,threadThumbnailDetail,threadProperty,latestTakedownEventDetail,newArrivalEventDetail,threadEvents&count=100",
            self.region(),
            MESSAGE_THREAD_ENTRY,
            thread_id
        )
    }

    fn generate_thread_encode(&self) -> String {
        format!("https://{}{}/", self.region(), MESSAGE_THREAD_ENTRY)
    }

    fn leave_message_thread_encode(&self, thread_id: &str) -> String {
        format!(
            "https://{}{}/{}/users/me",
            self.region(),
            MESSAGE_THREAD_ENTRY,
            thread_id
        )
    }

    fn send_message_encode(&self, thread_id: &str) -> String {
        format!(
            "https://{}{}/{}/messages",
            self.region(),
            MESSAGE_THREAD_ENTRY,
            thread_id
        )
    }

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

    /// boundary is used to when making multipart request to PSN.
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
