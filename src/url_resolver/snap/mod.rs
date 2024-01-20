use anyhow::{anyhow, bail, Result};
use reqwest::Client;
use url::Url;

mod util;
use util::{decode, DECODER_ARGS_REGEX, RESULT_VIDEO_URL_REGEX, TOKEN_REGEX};

use super::{ResolveUrl, Platform};

static BOUNDARY: &str = "----WebKitFormBoundary214sQgEtL6ZBo4uE";

#[derive(Debug)]
pub struct SnapUrlResolver<'a> {
    client: &'a Client,
    platform: &'a Platform,
}

impl<'a> SnapUrlResolver<'a> {
    pub fn new(client: &'a Client, platform: &'a Platform) -> Self {
        Self { client, platform }
    }

    async fn get_token(&self) -> Result<String> {
        if matches!(self.platform, Platform::Instagram) {
            return Ok("".to_string());
        }

        let html = self
            .client
            .get("https://snaptik.app/en")
            .send()
            .await?
            .text()
            .await?;

        let capts = TOKEN_REGEX.captures(&html).unwrap();
        if capts.len() > 0 {
            return Ok(capts[1].to_string());
        }

        bail!("Unable to get token")
    }

    async fn get_multipart_content(&self, url: &str) -> Result<String> {
        Ok(format!(
            include_str!("../../../assets/multipart.txt"),
            url = url,
            token = self.get_token().await?,
            boundary = BOUNDARY,
        ))
    }

    fn get_endpoint(&self) -> Result<&str> {
        match self.platform {
            Platform::TikTok => Ok("https://snaptik.app/abc2.php"),
            Platform::Instagram => Ok("https://snapinsta.app/action2.php"),
            _ => bail!("Unsupported platform: {:?}", self.platform),
        }
    }

    fn get_referer(&self) -> Result<&str> {
        match self.platform {
            Platform::TikTok => Ok("https://snaptik.app/"),
            Platform::Instagram => Ok("https://snapinsta.app/"),
            _ => bail!("Unsupported platform: {:?}", self.platform),
        }
    }
}

impl<'a> ResolveUrl<'a> for SnapUrlResolver<'a> {
    async fn resolve_url(&self, url: &'a str) -> Result<Url> {
        let multipart_content = self.get_multipart_content(url).await?;
        let endpoint = self.get_endpoint()?;
        let referer = self.get_referer()?;

        let encoded_str = self.client.post(endpoint)
            .body(multipart_content.to_owned())
            .header("referer", referer)
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            )
            .header("content-length", multipart_content.len().to_string())
            .send()
            .await?
            .text()
            .await?;

        let capts = DECODER_ARGS_REGEX
            .captures(&encoded_str)
            .ok_or(anyhow!("Cannot find result URL:\n\n{encoded_str}\n\n"))?;
        if capts.len() < 7 {
            bail!("Cannot find result URL:\n\n{encoded_str}\n\n");
        }

        let decoded_str = decode(
            &capts[1],
            capts[2].parse().unwrap(),
            &capts[3],
            capts[4].parse().unwrap(),
            capts[5].parse().unwrap(),
            capts[6].parse().unwrap(),
        );

        let capts = RESULT_VIDEO_URL_REGEX
            .captures(&decoded_str)
            .ok_or(anyhow!("Cannot find result URL:\n\n{decoded_str}\n\n"))?;
        if capts.len() == 0 {
            bail!("Cannot find result URL:\n\n{decoded_str}\n\n");
        }

        Url::parse(&capts[1]).map_err(|err| anyhow!(err))
    }
}
