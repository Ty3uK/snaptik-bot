pub mod snap;

use anyhow::{Result, bail};
use url::Url;

use self::snap::SnapUrlResolver;

#[derive(Debug)]
pub enum Platform {
    TikTok,
    Instagram,
}

impl Platform {
    pub fn new(url: Option<&str>) -> Result<Self> {
        match url {
            Some(url) if url.ends_with("tiktok.com") => Ok(Self::TikTok),
            Some(url) if url.ends_with("instagram.com") => Ok(Self::Instagram),
            _ => bail!("This kind of link is not supported yet."),
        }
    }
}

pub trait ResolveUrl<'a> {
    async fn resolve_url(&self, url: &'a str) -> Result<Url>;
}

#[derive(Debug)]
pub enum UrlResolver<'a> {
    TikTok(SnapUrlResolver<'a>),
    Instagram(SnapUrlResolver<'a>),
}

impl<'a> ResolveUrl<'a> for UrlResolver<'a> {
    async fn resolve_url(&self, url: &'a str) -> Result<Url> {
        match &self {
            Self::TikTok(resolver) => resolver.resolve_url(url).await,
            Self::Instagram(resolver) => resolver.resolve_url(url).await,
        }
    }
}
