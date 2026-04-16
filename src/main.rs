use std::sync::Arc;

use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum::{Router, extract::State, routing::post};
use rand::distr::{Alphanumeric, SampleString};
use reqwest::Url;
use tokio::fs;
use tracing::debug;
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::telegram::Telegram;
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

    telegram.set_webhook(config.telegram.webhook_url).await?;

    let server = Router::new()
        .route("/", post(webhook_handler))
        .with_state(Arc::new(ServerState {
            tiktok_resolver,
            twitter_resolver,
            instagram_resolver,
        }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, server).await?;

    return Ok(());
}

async fn webhook_handler(State(state): State<ServerStateType>, body: String) -> StatusCode {
    // let source_url = Url::parse(&body).unwrap();
    // let res = match source_url.domain() {
    //     Some(d) if d.ends_with("tiktok.com") => {
    //         state.tiktok_resolver.resolve(&source_url).await.unwrap()
    //     }
    //     Some(d) if d.ends_with("twitter.com") || d.ends_with("x.com") => {
    //         state.twitter_resolver.resolve(&source_url).await.unwrap()
    //     }
    //     Some(d) if d.ends_with("instagram.com") => {
    //         state.instagram_resolver.resolve(&source_url).await.unwrap()
    //     }
    //     _ => return StatusCode::OK,
    // };
    // debug!("{res:?}");
    return StatusCode::OK;
}

type ServerStateType = Arc<ServerState>;

struct ServerState {
    tiktok_resolver: TikTokResolver,
    twitter_resolver: TwitterResolver,
    instagram_resolver: InstagramResolver,
}
