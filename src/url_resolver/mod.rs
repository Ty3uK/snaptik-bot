pub mod shorts;
pub mod snap;
pub mod twitter;

use anyhow::{bail, Result};
use url::Url;

use self::{shorts::ShortsUrlResolver, snap::SnapUrlResolver, twitter::TwitterUrlResolver};

#[derive(Debug)]
pub enum Platform {
    TikTok,
    Instagram,
    Shorts,
    Twitter,
}

impl Platform {
    pub fn new(url: Option<&str>) -> Result<Self> {
        match url {
            Some(url) if url.ends_with("tiktok.com") => Ok(Self::TikTok),
            Some(url) if url.ends_with("instagram.com") => Ok(Self::Instagram),
            Some(url) if url.ends_with("youtube.com") => Ok(Self::Shorts),
            Some(url) if url.ends_with("twitter.com") => Ok(Self::Twitter),
            Some(url) if url.ends_with("x.com") => Ok(Self::Twitter),
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
    Shorts(ShortsUrlResolver<'a>),
    Twitter(TwitterUrlResolver<'a>),
}

impl<'a> ResolveUrl<'a> for UrlResolver<'a> {
    async fn resolve_url(&self, url: &'a str) -> Result<Url> {
        match &self {
            Self::TikTok(resolver) => resolver.resolve_url(url).await,
            Self::Instagram(resolver) => resolver.resolve_url(url).await,
            Self::Shorts(resolver) => resolver.resolve_url(url).await,
            Self::Twitter(resolver) => resolver.resolve_url(url).await,
        }
    }
}
