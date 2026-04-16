use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::Deserialize;

use crate::resolver::{Resolver, ResolverResult};

pub struct InstagramResolver {
    client: Arc<Client>,
}

impl InstagramResolver {
    pub fn new(client: Arc<Client>) -> Self {
        return Self { client };
    }
}

impl Resolver for InstagramResolver {
    async fn resolve(&self, source_url: &reqwest::Url) -> Result<super::ResolverResult> {
        let kind = source_url
            .path_segments()
            .context("Instagram:resolve: cannot get path segments")?
            .filter(|v| !v.is_empty())
            .next()
            .context("Instagram:resolve: cannot get kind")?;
        let id = source_url
            .path_segments()
            .context("Instagram:resolve: cannot get path segments")?
            .filter(|v| !v.is_empty())
            .last()
            .context("Instagram:resolve: cannot get id")?;
        let id = if kind != "stories" {
            id_to_pk(id).context("Instagram:resolve: cannot convert id to video_id")?
        } else {
            id.parse().context("Instagram:resolve: cannot parse id")?
        };
        let body = self.client.get(format!("https://i.instagram.com/api/v1/media/{id}/info/"))
            .header("Accept", "*/*")
            .header("Origin", "https://www.instagram.com")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0"
            )
            .header("X-IG-App-ID", "936619743392459")
            .header("X-ASBD-ID", "0")
            .header("X-IG-WWW-Claim", "0")
            .send()
            .await
            .context("Instagram:resolve: cannot make request")?
            .bytes()
            .await
            .context("Instagram:resolve: cannot read body")?;
        let json: InstagramResponse =
            serde_json::from_slice(&body).context("Instagram:resolve: cannot parse json")?;
        if let Some(items) = json.items
            && let Some(item) = items.get(0)
        {
            let video = item
                .video_versions
                .iter()
                .max_by_key(|v| v.bandwidth)
                .context("Instagram:resolve: cannot find video")?;
            return Ok(ResolverResult {
                url: video.url.clone(),
                width: video.width,
                height: video.height,
                referer: None,
            });
        }
        return Err(anyhow!("Instagram:resolve: cannot find video"));
    }
}

#[derive(Debug, Deserialize)]
struct InstagramResponse {
    pub items: Option<Vec<InstagramItem>>,
}

#[derive(Debug, Deserialize)]
struct InstagramItem {
    pub video_versions: Vec<InstagramVideoVersion>,
}

#[derive(Debug, Deserialize)]
struct InstagramVideoVersion {
    pub bandwidth: u32,
    pub width: u32,
    pub height: u32,
    pub url: String,
}

const INVALID: u8 = 0xFF;

static DECODE_TABLE: [u8; 256] = {
    let mut table = [INVALID; 256];

    let mut i = 0;

    // A-Z
    while i < 26 {
        table[b'A' as usize + i] = i as u8;
        i += 1;
    }

    // a-z
    i = 0;
    while i < 26 {
        table[b'a' as usize + i] = (26 + i) as u8;
        i += 1;
    }

    // 0-9
    i = 0;
    while i < 10 {
        table[b'0' as usize + i] = (52 + i) as u8;
        i += 1;
    }

    table[b'-' as usize] = 62;
    table[b'_' as usize] = 63;

    table
};

fn id_to_pk(id: &str) -> Result<u64> {
    let id = if id.len() > 28 {
        &id[id.len() - 28..]
    } else {
        id
    };

    let mut result: u64 = 0;

    for &b in id.as_bytes() {
        let val = DECODE_TABLE[b as usize];
        if val == INVALID {
            anyhow::bail!("Invalid character '{}'", b as char);
        }

        result = result.saturating_mul(64).saturating_add(val as u64);
    }

    Ok(result)
}
