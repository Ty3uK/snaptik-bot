use anyhow::{anyhow, bail, Result};
use reqwest::Client;
use url::Url;

use crate::url_resolver::shorts::util::get_media_url;

use self::util::get_cookie;

use super::ResolveUrl;

mod util;
use util::CSRF_REGEX;

#[derive(Debug)]
pub struct ShortsUrlResolver<'a> {
    http_client: &'a Client,
}

#[derive(Debug)]
struct AuthData {
    csrf: String,
    cookie: String,
}

impl<'a> ShortsUrlResolver<'a> {
    pub fn new(http_client: &'a Client) -> Self {
        Self { http_client }
    }

    fn get_csrf(&self, html: &str) -> Result<String> {
        let capts = CSRF_REGEX
            .captures(html)
            .ok_or(anyhow!("Cannot get `csrf_token`"))?;
        if capts.len() == 0 {
            bail!("Cannot get `csrf_token`");
        }
        Ok(capts[1].to_string())
    }

    async fn get_auth_data(&self) -> Result<AuthData> {
        let res = self
            .http_client
            .get("https://shortsmate.com/en/")
            .send()
            .await
            .map_err(|err| anyhow!(err))?;
        let cookie = get_cookie(res.headers())?;
        let html = res.text().await.map_err(|err| anyhow!(err))?;
        let csrf = self.get_csrf(&html)?;
        Ok(AuthData { csrf, cookie })
    }
}

impl<'a> ResolveUrl<'a> for ShortsUrlResolver<'a> {
    async fn resolve_url(&self, url: &'a str) -> anyhow::Result<Url> {
        let AuthData { csrf, cookie } = self.get_auth_data().await?;
        let html = self.http_client.post("https://shortsmate.com/en/download")
            .form(&[
                ("csrf_token", csrf),
                ("url", url.to_string()),
            ])
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
            .header("Referer", "https://shortsmate.com/en/download")
            .header("Cookie", cookie)
            .send()
            .await?
            .text()
            .await
            .map_err(|err| anyhow!(err))?;
        let url = get_media_url(&html)?;
        Ok(Url::parse(&url)?)
    }
}
