use anyhow::anyhow;
use std::sync::Arc;

use anyhow::{Context, Result};
use reqwest::{
    Body, Client,
    multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};

pub struct Telegram {
    client: Arc<Client>,
    secret: String,
    endpoint: String,
}

impl Telegram {
    pub fn new(client: Arc<Client>, token: &str, secret: &str) -> Self {
        return Self {
            client,
            secret: secret.to_string(),
            endpoint: format!("https://api.telegram.org/bot{token}"),
        };
    }

    pub async fn delete_webhook(&self) -> Result<bool> {
        let body = self
            .client
            .post(format!("{}/deleteWebhook", self.endpoint))
            .send()
            .await
            .context("Telegram:set_webhook: cannot make request")?
            .bytes()
            .await
            .context("Telegram:set_webhook: cannot read body")?;
        let res: TelegramResponse<bool> =
            serde_json::from_slice(&body).context("Telegram:delete_webhook: cannot parse json")?;
        match res {
            TelegramResponse::Ok { result, .. } => return Ok(result),
            TelegramResponse::Err { description, .. } => {
                return Err(anyhow!("Telegram:delete_webhook: {}", description));
            }
        }
    }

    pub async fn set_webhook(&self, url: &str) -> Result<bool> {
        let form = Form::new()
            .text("url", url.to_owned())
            .text("secret_token", self.secret.to_owned());
        let body = self
            .client
            .post(format!("{}/setWebhook", self.endpoint))
            .multipart(form)
            .send()
            .await
            .context("Telegram:set_webhook: cannot make request")?
            .bytes()
            .await
            .context("Telegram:set_webhook: cannot read body")?;
        let res: TelegramResponse<bool> =
            serde_json::from_slice(&body).context("Telegram:set_webhook: cannot parse json")?;
        match res {
            TelegramResponse::Ok { result, .. } => return Ok(result),
            TelegramResponse::Err { description, .. } => {
                return Err(anyhow!("Telegram:set_webhook: {}", description));
            }
        }
    }

    pub async fn send_video(
        &self,
        chat_id: i64,
        video: Body,
        width: usize,
        height: usize,
    ) -> Result<Message> {
        let form = Form::new()
            .text("chat_id", chat_id.to_string())
            .text("width", width.to_string())
            .text("height", height.to_string())
            .part("video", Part::stream(video));
        let body = self
            .client
            .post(format!("{}/sendVideo", self.endpoint))
            .multipart(form)
            .send()
            .await
            .context("Telegram:set_webhook: cannot make request")?
            .bytes()
            .await
            .context("Telegram:set_webhook: cannot read body")?;
        let res: TelegramResponse<Message> =
            serde_json::from_slice(&body).context("Telegram:set_webhook: cannot parse json")?;
        match res {
            TelegramResponse::Ok { result, .. } => return Ok(result),
            TelegramResponse::Err { description, .. } => {
                return Err(anyhow!("Telegram:send_video: {}", description));
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TelegramResponse<T> {
    Ok {
        #[serde(rename = "ok")]
        _ok: bool,
        result: T,
    },
    Err {
        #[serde(rename = "ok")]
        _ok: bool,
        description: String,
    },
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Message,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub from: From,
    pub chat: Chat,
    pub date: i64,
    pub text: String,
    pub entities: Option<Vec<Entity>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct From {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    pub language_code: String,
    pub is_premium: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub offset: usize,
    pub length: usize,
    #[serde(rename = "type")]
    pub type_field: String,
}

pub fn parse_entities<'a>(text: &'a str, entities: &'a [Entity]) -> Vec<(&'a str, &'a Entity)> {
    let mut result = Vec::with_capacity(entities.len());

    let mut entities_sorted: Vec<_> = entities.iter().collect();
    entities_sorted.sort_by_key(|e| e.offset);

    let mut utf16_pos = 0;
    let mut next = 0;
    let mut active: Vec<(usize, &'a Entity)> = Vec::new();

    for (byte_idx, ch) in text.char_indices() {
        // start entities
        while next < entities_sorted.len() && entities_sorted[next].offset == utf16_pos {
            active.push((byte_idx, entities_sorted[next]));
            next += 1;
        }

        utf16_pos += ch.len_utf16();

        // end entities
        let mut i = 0;
        while i < active.len() {
            let (start_byte, entity) = active[i];

            if entity.offset + entity.length == utf16_pos {
                let end_byte = byte_idx + ch.len_utf8();
                if let Some(slice) = text.get(start_byte..end_byte) {
                    result.push((slice, entity));
                }
                active.remove(i);
            } else {
                i += 1;
            }
        }
    }

    result
}
