use anyhow::{anyhow, bail};
use reqwest::Client;
use url::Url;

use self::util::DOWNLOAD_LINK_REGEX;

use super::ResolveUrl;

mod util;

#[derive(Debug)]
pub struct TwitterUrlResolver<'a> {
    http_client: &'a Client,
}

impl<'a> TwitterUrlResolver<'a> {
    pub fn new(http_client: &'a Client) -> Self {
        Self { http_client }
    }
}

impl<'a> ResolveUrl<'a> for TwitterUrlResolver<'a> {
    async fn resolve_url(&self, url: &'a str) -> anyhow::Result<url::Url> {
        let html = self.http_client.post("https://savetwitter.net/api/ajaxSearch")
            .form(&[
                ("q", url),
                ("lang", "en")
            ])
            .header("Referer", "https://savetwitter.net/")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0")
            .send()
            .await?
            .text()
            .await
            .map_err(|err| anyhow!(err))?;
        let capts = DOWNLOAD_LINK_REGEX
            .captures(&html)
            .ok_or(anyhow!("Cannot find URL"))?;
        if capts.len() < 2 {
            bail!("Cannot find URL");
        }
        Url::parse(&capts[1]).map_err(|err| anyhow!(err))
    }
}
