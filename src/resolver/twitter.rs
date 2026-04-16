use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use reqwest::{
    Client, Response, Url,
    header::{HeaderMap, HeaderValue},
};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::resolver::{Resolver, ResolverResult};

pub struct TwitterResolver {
    client: Arc<Client>,
    guest_token: RwLock<Option<String>>,
    common_headers: HeaderMap,
}

impl TwitterResolver {
    pub fn new(client: Arc<Client>) -> Self {
        let mut common_headers = HeaderMap::new();
        common_headers.insert(
            "User-Agent",
            HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0")
        );
        common_headers
            .insert("Authorization", HeaderValue::from_static("Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA"));
        common_headers.append("x-twitter-client-language", HeaderValue::from_static("en"));
        common_headers.append("x-twitter-active-user", HeaderValue::from_static("yes"));
        common_headers.append("accept-language", HeaderValue::from_static("en"));
        return Self {
            client,
            guest_token: RwLock::new(None),
            common_headers,
        };
    }

    async fn get_guest_token(&self) -> Result<String> {
        let res = self
            .client
            .post("https://api.x.com/1.1/guest/activate.json")
            .headers(self.common_headers.clone())
            .send()
            .await
            .context("Twitter:get_guest_token: cannot make request")?
            .bytes()
            .await
            .context("Twitter:get_guest_token: cannot read body")?;
        let json: TwitterGuestTokenResponse =
            serde_json::from_slice(&res).context("Twitter:get_guest_token: cannot parse json")?;
        return Ok(json.guest_token.into());
    }

    async fn do_request(&self, url: Url) -> Result<Response> {
        let guest_token = self
            .guest_token
            .read()
            .await
            .clone()
            .context("Twitter:do_request: guest_token is empty")?;
        return self
            .client
            .get(url)
            .headers(self.common_headers.clone())
            .header("x-guest-token", guest_token)
            .send()
            .await
            .with_context(|| format!("Twitter:do_request: cannot make request"));
    }
}

impl Resolver for TwitterResolver {
    async fn resolve(&self, source_url: &Url) -> Result<ResolverResult> {
        if self.guest_token.read().await.is_none() {
            let token = self
                .get_guest_token()
                .await
                .context("twitter:resolve: cannot get guest_token")?;
            let mut guard = self.guest_token.write().await;
            *guard = Some(token);
        }
        let tweet_id = source_url
            .path_segments()
            .context("Twitter:resolve: cannot get path segments")?
            .last()
            .context("Twitter:resolve: cannot get tweet id")?;
        let url = Url::parse_with_params(
            "https://x.com/i/api/graphql/2ICDjqPd81tulZcYrtpTuQ/TweetResultByRestId",
            [(
                "variables",
                format!(
                    r#"{{"tweetId":"{tweet_id}","includePromotedContent":false,"withVoice":false,"withCommunity":false}}"#
                ),
            )],
        )?;
        let mut res = self
            .do_request(url.clone())
            .await
            .context("Twitter:resolve: cannot do request")?;
        let status_code = res.status();
        if status_code == 403 || status_code == 404 || status_code == 429 {
            let mut guard = self.guest_token.write().await;
            *guard = Some(
                self.get_guest_token()
                    .await
                    .context("Twitter:resolve: cannot get guest_token")?,
            );
            res = self
                .do_request(url)
                .await
                .context("Twitter:resolve: cannot do request")?;
        }
        if status_code != 200 {
            return Err(anyhow!(
                "Twitter:resolve: bad response status code: {status_code}"
            ));
        }
        let body = res
            .bytes()
            .await
            .context("Twitter:resolve: cannot read body")?;
        let json: TwitterResponse =
            serde_json::from_slice(&body).context("Twitter:resolve: cannot parse json")?;
        let media = json
            .data
            .tweet_result
            .result
            .legacy
            .entities
            .media
            .get(0)
            .context("Twitter:resolve: media is empty")?;
        let url = media
            .video_info
            .variants
            .iter()
            .filter(|v| v.content_type == "video/mp4")
            .max_by_key(|v| v.bitrate)
            .map(|v| v.url.clone())
            .context("Twitter:resolve: cannot find url")?;
        return Ok(ResolverResult {
            url,
            width: media.original_info.width,
            height: media.original_info.height,
            referer: Some("https://x.com".to_string()),
        });
    }
}

#[derive(Deserialize)]
struct TwitterGuestTokenResponse {
    guest_token: String,
}

#[derive(Deserialize)]
struct TwitterResponse {
    data: TwitterData,
}

#[derive(Deserialize)]
struct TwitterData {
    #[serde(rename = "tweetResult")]
    tweet_result: TwitterTweetResult,
}

#[derive(Deserialize)]
struct TwitterTweetResult {
    result: TwitterResult,
}

#[derive(Deserialize)]
struct TwitterResult {
    legacy: TwitterLegacy,
}

#[derive(Deserialize)]
struct TwitterLegacy {
    entities: TwitterEntities,
}

#[derive(Deserialize)]
struct TwitterEntities {
    media: Vec<TwitterMedia>,
}

#[derive(Deserialize)]
struct TwitterMedia {
    original_info: TwitterOriginalInfo,
    video_info: TwitterVideoInfo,
}

#[derive(Deserialize)]
struct TwitterOriginalInfo {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct TwitterVideoInfo {
    variants: Vec<TwitterVariant>,
}

#[derive(Debug, Deserialize)]
struct TwitterVariant {
    content_type: String,
    bitrate: Option<u64>,
    url: String,
}
