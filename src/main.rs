use anyhow::anyhow;
use futures::TryFutureExt;
use futures::future::join_all;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::http::StatusCode;
use axum::{Router, extract::State, routing::post};
use rand::distr::{Alphanumeric, SampleString};
use reqwest::{Client, Url};
use tokio::fs;
use tracing::info;
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::telegram::{Telegram, TelegramUpdate, parse_entities};
use crate::{
    config::Config,
    resolver::{
        Resolver, instagram::InstagramResolver, tiktok::TikTokResolver, twitter::TwitterResolver,
    },
};

mod config;
mod resolver;
mod telegram;

#[tokio::main]
async fn main() -> Result<()> {
    let log_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(log_filter)
        .with(subscriber)
        .init();
    if let Err(error) = run().await {
        tracing::error!(
            error = %error,
            error_debug = ?error,
            "application_error"
        );
    }
    return Ok(());
}

async fn run() -> Result<()> {
    let config = fs::read("config.toml").await.unwrap_or_default();
    let config: Config = toml::from_slice(&config).context("run: cannot parse config")?;

    let mut store = reqwest_cookie_store::CookieStore::new();
    store
        .insert_raw(
            &cookie_store::RawCookie::build((
                "sessionid",
                config.instagram.unwrap_or_default().session_id,
            ))
            .domain(".instagram.com")
            .build(),
            &Url::parse("https://instagram.com").context("run: cannot parse instagram URL")?,
        )
        .context("run: cannot insert `sessionid` cookie")?;
    let store = reqwest_cookie_store::CookieStoreMutex::new(store);
    let store = std::sync::Arc::new(store);
    let client = Arc::new(
        reqwest::ClientBuilder::new()
            .cookie_provider(std::sync::Arc::clone(&store))
            .build()
            .with_context(|| "Cannot build reqwest client")?,
    );

    let tiktok_resolver = TikTokResolver::new(Arc::clone(&client));
    let twitter_resolver = TwitterResolver::new(Arc::clone(&client));
    let instagram_resolver = InstagramResolver::new(Arc::clone(&client));

    let secret = Alphanumeric.sample_string(&mut rand::rng(), 32);
    let telegram = Telegram::new(Arc::clone(&client), config.telegram.token, &secret);

    telegram.delete_webhook().await?;
    telegram.set_webhook(config.telegram.webhook_url).await?;

    let server = Router::new()
        .route("/", post(webhook_handler))
        .with_state(Arc::new(ServerState {
            client,
            telegram: Arc::new(telegram),
            tiktok_resolver,
            twitter_resolver,
            instagram_resolver,
        }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, server).await?;

    return Ok(());
}

async fn webhook_handler(
    State(state): State<ServerStateType>,
    Json(body): Json<TelegramUpdate>,
) -> StatusCode {
    let tiktok_resolver = Arc::new(&state.tiktok_resolver);
    let twitter_resolver = Arc::new(&state.twitter_resolver);
    let instagram_resolver = Arc::new(&state.instagram_resolver);
    let res = body
        .message
        .entities
        .iter()
        .flat_map(|v| parse_entities(&body.message.text, v))
        .filter(|(_, entity)| entity.type_field == "url")
        .filter_map(|(url, _)| Url::parse(url).ok())
        .map(|v| {
            let tiktok_resolver = tiktok_resolver.clone();
            let twitter_resolver = twitter_resolver.clone();
            let instagram_resolver = instagram_resolver.clone();
            return async move {
                if let Some(d) = v.domain() {
                    if d.ends_with("tiktok.com") {
                        return tiktok_resolver.resolve(&v).await;
                    }
                    if d.ends_with("twitter.com") || d.ends_with("x.com") {
                        return twitter_resolver.resolve(&v).await;
                    }
                    if d.ends_with("instagram.com") {
                        return instagram_resolver.resolve(&v).await;
                    }
                    return Err(anyhow!("App:webhook_url: unknown URL"));
                }
                return Err(anyhow!("App:webhook_url: unknown URL"));
            };
        })
        .map(|f| {
            f.and_then(|v| {
                let client = state.client.clone();
                let telegram = state.telegram.clone();
                return async move {
                    let res = client
                        .get(&v.url)
                        .header("Referer", v.referer.unwrap_or_default())
                        .send()
                        .await
                        .context("App:wehbook_url: cannot make request")?;
                    return telegram
                        .send_video(
                            body.message.chat.id,
                            res.into(),
                            v.width as usize,
                            v.height as usize,
                        )
                        .await;
                };
            })
        });
    let res = join_all(res).await;
    for res in res {
        info!("{res:?}");
    }
    return StatusCode::OK;
}

type ServerStateType = Arc<ServerState>;

struct ServerState {
    client: Arc<Client>,
    telegram: Arc<Telegram>,
    tiktok_resolver: TikTokResolver,
    twitter_resolver: TwitterResolver,
    instagram_resolver: InstagramResolver,
}
