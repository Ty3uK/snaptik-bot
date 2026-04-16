use anyhow::anyhow;
use std::sync::Arc;

use anyhow::{Context, Result};
use reqwest::{Client, multipart::Form};
use serde::Deserialize;

pub struct Telegram<'a> {
    client: Arc<Client>,
    secret: &'a str,
    endpoint: String,
}

impl<'a> Telegram<'a> {
    pub fn new(client: Arc<Client>, token: &'a str, secret: &'a str) -> Self {
        return Self {
            client,
            secret: secret,
            endpoint: format!("https://api.telegram.org/bot{token}"),
        };
    }

    pub async fn set_webhook(&'a self, url: &'a str) -> Result<bool> {
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
