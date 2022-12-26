use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct SetWebhook {
    pub url: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Chat {
    pub id: isize,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Message {
    pub message_id: Option<isize>,
    pub chat: Option<Chat>,
    pub text: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Update {
    pub update_id: isize,
    pub message: Message,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SendMessage {
    pub chat_id: isize,
    pub text: String,
    pub reply_to_message_id: Option<isize>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SendVideo {
    pub chat_id: isize,
    pub video: String,
    pub reply_to_message_id: Option<isize>,
}

pub struct Telegram<'a> {
    client: &'a Client,
    api_path: &'a String,
}

impl<'a> Telegram<'a> {
    pub fn new(client: &'a Client, api_path: &'a String) -> Self {
        Self {
            client,
            api_path,
        }
    }

    pub async fn set_webhook(&self, webhook: &SetWebhook) -> Result<String, Error> {
        self.client.post(self.api_path.to_owned() + "/setWebhook")
            .json(&webhook)
            .send()
            .await?
            .text()
            .await
    }

    pub async fn send_message(&self, message: &SendMessage) -> Result<Message, Error> {
        self.client.post(self.api_path.to_owned() + "/sendMessage")
            .json(&message)
            .send()
            .await?
            .json::<Message>()
            .await
    }

    pub async fn send_video(&self, video: &SendVideo) -> Result<(), Error> {
        self.client.post(self.api_path.to_owned() + "/sendVideo")
            .json(&video)
            .send()
            .await?;

        Ok(())
    }
}
