use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum ChatType {
    #[serde(rename(deserialize = "private"))]
    Private,
    #[serde(rename(deserialize = "group"))]
    Group,
    #[serde(rename(deserialize = "supergroup"))]
    Supergroup,
    #[serde(rename(deserialize = "channel"))]
    Channel,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SetWebhook<'a> {
    pub url: &'a str,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Chat {
    pub id: i64,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub chat_type: ChatType,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Message {
    pub message_id: Option<isize>,
    pub chat: Option<Chat>,
    pub text: Option<String>,
    pub reply_to_message: Option<Box<Message>>,
    pub video: Option<Video>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Video {
    pub file_id: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Update {
    pub update_id: isize,
    pub message: Option<Message>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SendMessage {
    pub chat_id: i64,
    pub text: String,
    pub reply_to_message_id: Option<isize>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SendVideo {
    pub chat_id: i64,
    pub video: String,
    pub reply_to_message_id: Option<isize>,
    pub caption: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum Response<T> {
    Ok {
        result: T,
    },
    Err {
        error_code: i64,
        description: String,
    },
}

pub struct Telegram<'a> {
    client: &'a Client,
    api_path: &'a str,
}

impl<'a> Telegram<'a> {
    pub fn new(client: &'a Client, api_path: &'a str) -> Self {
        Self {
            client,
            api_path,
        }
    }

    pub async fn set_webhook(&self, webhook: &SetWebhook<'_>) -> Result<String> {
        self.client.post(self.api_path.to_owned() + "/setWebhook")
            .json(&webhook)
            .send()
            .await?
            .text()
            .await
            .map_err(|err| anyhow!(err))
    }

    pub async fn send_message(&self, message: &SendMessage) -> Result<Message> {
        self.client.post(self.api_path.to_owned() + "/sendMessage")
            .json(&message)
            .send()
            .await?
            .json::<Response<Message>>()
            .await
            .map(|resp| match resp {
                Response::Ok { result } => Ok(result),
                Response::Err { description, .. } => Err(anyhow!(description)),
            })?
    }

    pub async fn send_video(&self, video: &SendVideo) -> Result<Message> {
        self.client.post(self.api_path.to_owned() + "/sendVideo")
            .json(&video)
            .send()
            .await?
            .json::<Response<Message>>()
            .await
            .map(|resp| match resp {
                Response::Ok { result } => Ok(result),
                Response::Err { description, .. } => Err(anyhow!(description)),
            })?
    }
}
