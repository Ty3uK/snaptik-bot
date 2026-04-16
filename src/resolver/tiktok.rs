use std::sync::Arc;

use anyhow::{Ok, anyhow};

use anyhow::Context;
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::resolver::{Resolver, ResolverResult};

pub struct TikTokResolver {
    client: Arc<Client>,
}

const SCRIPT_START: &[u8; 72] =
    br#"<script id="__UNIVERSAL_DATA_FOR_REHYDRATION__" type="application/json">"#;
const SCRIPT_END: &[u8; 9] = b"</script>";

impl TikTokResolver {
    pub fn new(client: Arc<Client>) -> Self {
        return Self { client };
    }
}

impl Resolver for TikTokResolver {
    async fn resolve(&self, source_url: &Url) -> anyhow::Result<ResolverResult> {
        let body = self
            .client
            .get(source_url.clone())
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0")
            .header("Referer", "https://tiktok.com/")
            .send()
            .await
            .context("TikTok:resolve: cannot make request")?
            .bytes()
            .await
            .context("TikTok:resolve: cannot read body")?;
        let start = body
            .windows(SCRIPT_START.len())
            .position(|w| w == SCRIPT_START)
            .context("Cannot find SCRIPT_START")?
            + SCRIPT_START.len();
        let end = body[start..]
            .windows(SCRIPT_END.len())
            .position(|w| w == SCRIPT_END)
            .context("Cannot find SCRIPT_END")?
            + start;
        let raw_json = &body[start..end];
        let json: TikTokResponse =
            serde_json::from_slice(raw_json).context("TikTok:resolve: cannot parse json")?;
        if let Some(video) = json
            .default_scope
            .webapp_video_detail
            .item_info
            .item_struct
            .video
        {
            let item = video
                .bitrate_info
                .iter()
                .filter(|v| v.codec_type.starts_with("h265") && !v.play_addr.url_list.is_empty())
                .max_by_key(|v| v.bitrate)
                .context("TikTok:resolve: cannot find video")?;
            return Ok(ResolverResult {
                url: String::from(item.play_addr.url_list[0].clone()),
                width: video.width,
                height: video.height,
                referer: Some(String::from("https://www.tiktok.com/")),
            });
        }
        return Err(anyhow!("TikTok:resolve: cannot find video"));
    }
}

#[derive(Debug, Deserialize)]
struct TikTokResponse {
    #[serde(rename = "__DEFAULT_SCOPE__")]
    pub default_scope: TikTokDefaultScope,
}

#[derive(Debug, Deserialize)]
struct TikTokDefaultScope {
    #[serde(rename = "webapp.video-detail")]
    pub webapp_video_detail: TikTokWebappVideoDetail,
}

#[derive(Debug, Deserialize)]
struct TikTokWebappVideoDetail {
    #[serde(rename = "itemInfo")]
    pub item_info: TikTokItemInfo,
}

#[derive(Debug, Deserialize)]
struct TikTokItemInfo {
    #[serde(rename = "itemStruct")]
    pub item_struct: TikTokItemStruct,
}

#[derive(Debug, Deserialize)]
struct TikTokItemStruct {
    pub video: Option<TikTokVideo>,
}

#[derive(Debug, Deserialize)]
struct TikTokVideo {
    pub height: u32,
    pub width: u32,
    #[serde(rename = "bitrateInfo")]
    pub bitrate_info: Vec<TikTokBitrateInfo>,
}

#[derive(Debug, Deserialize)]
struct TikTokBitrateInfo {
    #[serde(rename = "CodecType")]
    pub codec_type: String,
    #[serde(rename = "Bitrate")]
    pub bitrate: i64,
    #[serde(rename = "PlayAddr")]
    pub play_addr: TikTokPlayAddr,
}

#[derive(Debug, Deserialize)]
struct TikTokPlayAddr {
    #[serde(rename = "UrlList")]
    pub url_list: Vec<String>,
}
