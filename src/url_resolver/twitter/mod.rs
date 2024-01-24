use anyhow::{anyhow, bail, Result};
use reqwest::Client;

mod util;
use serde::Deserialize;
use url::Url;
use util::DOWNLOAD_LINK_REGEX;

use super::ResolveUrl;

#[derive(Debug)]
pub struct TwitterUrlResolver<'a> {
    http_client: &'a Client,
}

#[derive(Debug, Deserialize)]
pub struct Response {
    data: Option<String>,
}

impl<'a> TwitterUrlResolver<'a> {
    pub fn new(http_client: &'a Client) -> Self {
        Self { http_client }
    }
}

impl<'a> ResolveUrl<'a> for TwitterUrlResolver<'a> {
    async fn resolve_url(&self, url: &'a str) -> Result<url::Url> {
        let json = self.http_client.post("https://savetwitter.net/api/ajaxSearch")
            .form(&[
                ("q", url),
                ("lang", "en")
            ])
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0")
            .header("Referer", "https://savetwitter.net")
            .send()
            .await?
            .json::<Response>()
            .await
            .map_err(|err| anyhow!(err))?;
        let data = json.data.ok_or(anyhow!("Cannot read `data` field"))?;
        let capts = DOWNLOAD_LINK_REGEX
            .captures(&data)
            .ok_or(anyhow!("Cannot find download link."))?;
        if capts.len() < 2 {
            bail!("Cannot find download link");
        }
        Url::parse(&capts[1]).map_err(|err| anyhow!(err))
    }
}
