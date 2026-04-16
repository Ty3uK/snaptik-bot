use anyhow::Result;
use reqwest::Url;
use serde::Serialize;

pub mod tiktok;
pub mod twitter;
pub mod instagram;

#[derive(Debug, Serialize)]
pub struct ResolverResult {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub referer: Option<String>,
}

pub trait Resolver {
    async fn resolve(&self, source_url: &Url) -> Result<ResolverResult>;
}
