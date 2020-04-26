use crate::models::MessageDetail;

#[cfg(feature = "default")]
#[derive(Deserialize, Debug)]
pub(crate) struct Tokens {
    pub(crate) access_token: Option<String>,
    pub(crate) refresh_token: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PSNResponseError {
    pub(crate) error: PSNResponseErrorInner,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PSNResponseErrorInner {
    pub(crate) code: u32,
    pub(crate) message: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendMessage {
    message_event_detail: SendMessageEventDetail,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SendMessageEventDetail {
    event_category_code: u8,
    message_detail: MessageDetail,
}

impl SendMessage {
    pub(crate) fn new(body: Option<&str>, event_category_code: u8) -> Self {
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
pub(crate) struct GenerateNewThread<'a> {
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
    pub(crate) fn new(other_id: &'a str, self_id: &'a str) -> Self {
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
