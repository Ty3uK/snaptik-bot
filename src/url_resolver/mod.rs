pub mod shorts;
pub mod snap;
pub mod twitter;

use anyhow::{anyhow, bail, Result};
use url::Url;

#[derive(Debug)]
pub enum Platform {
    TikTok,
    Instagram,
    Shorts,
    Twitter,
}

impl Platform {
    pub fn new(url: &Url) -> Result<Self> {
        let host = url.host_str().ok_or(anyhow!("Cannot get URL host"))?;
        if host.ends_with("tiktok.com") {
            Ok(Self::TikTok)
        } else if host.ends_with("instagram.com") {
            Ok(Self::Instagram)
        } else if host.ends_with("youtube.com") {
            Ok(Self::Shorts)
        } else if host.ends_with("twitter.com") || host == "x.com" {
            Ok(Self::Twitter)
        } else {
            bail!("This kind of link is not supported yet.")
        }
    }
}

pub trait ResolveUrl<'a> {
    async fn resolve_url(&self, url: &'a str) -> Result<Url>;
}
