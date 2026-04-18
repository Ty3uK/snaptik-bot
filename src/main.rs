use axum::http::HeaderMap;
use futures::StreamExt;
use futures::stream;
use itertools::Itertools;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::http::StatusCode;
use axum::{Router, extract::State, routing::post};
use rand::distr::{Alphanumeric, SampleString};
use reqwest::{Client, Url};
use tokio::fs;
use tracing::error;
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::db::Db;
use crate::telegram::SendVideo;
use crate::telegram::{ReplyParameters, SendMessage, Telegram, TelegramUpdate, parse_entities};
use crate::{
    config::Config,
    resolver::{
        Resolver, instagram::InstagramResolver, tiktok::TikTokResolver, twitter::TwitterResolver,
    },
};

mod config;
mod db;
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

    let db = Db::new(config.sqlite.path)
        .await
        .context("App:run: cannot open database")?;

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
            db,
            telegram,
            tiktok_resolver,
            twitter_resolver,
            instagram_resolver,
        }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, server).await?;

    return Ok(());
}

async fn webhook_handler(
    headers: HeaderMap,
    State(state): State<ServerStateType>,
    Json(body): Json<TelegramUpdate>,
) -> StatusCode {
    let secret = headers.get("X-Telegram-Bot-Api-Secret-Token");
    if secret.is_none() || secret.unwrap().as_bytes() != state.telegram.secret.as_bytes() {
        return StatusCode::OK;
    }

    let chat_id = body.message.chat.id;
    let message_id = body.message.message_id;

    let text = match body.message.text {
        Some(v) => v,
        None => return StatusCode::OK,
    };

    let urls: Vec<Url> = body
        .message
        .entities
        .iter()
        .flat_map(|v| parse_entities(&text, v))
        .filter(|(_, entity)| entity.type_field == "url")
        .filter_map(|(url, _)| Url::parse(url).ok())
        .unique_by(|v| v.path().to_string())
        .collect();

    stream::iter(urls)
        .map(|url| {
            let state = state.clone();
            return async move {
                match state.db.get_video(url.as_str()).await {
                    Ok(Some(file_id)) => {
                        let res = state.telegram.send_video(SendVideo {
                            chat_id,
                            video: telegram::SendVideoVideo::FileId(file_id),
                            width: None,
                            height: None,
                            caption: url.to_string(),
                            reply_parameters: ReplyParameters {
                                message_id,
                                chat_id,
                                quote: url.to_string(),
                            },
                        }).await;
                        if let Err(err) = res {
                            tracing::error!(
                                message = "App:webhook_handler: cannot send telegram video by file_id",
                                error = %err
                            );
                        } else {
                            return;
                        }
                    }
                    Ok(None) => (),
                    Err(err) => tracing::error!(
                        message = "App:webhook_handler: cannot get video from db",
                        error = %err
                    ),
                };
                let result = async {
                    let result = match url.domain() {
                        Some(d) if d.ends_with("tiktok.com") => {
                            state.tiktok_resolver.resolve(&url).await?
                        }
                        Some(d) if d.ends_with("twitter.com") || d.ends_with("x.com") => {
                            state.twitter_resolver.resolve(&url).await?
                        }
                        Some(d) if d.ends_with("instagram.com") => {
                            state.instagram_resolver.resolve(&url).await?
                        }
                        _ => anyhow::bail!("unsupported domain"),
                    };
                    let video = state
                        .client
                        .get(result.url)
                        .header("Referer", result.referer)
                        .send()
                        .await?;
                    let size = video.content_length();
                    let message = state
                        .telegram
                        .send_video(SendVideo {
                            chat_id,
                            video: telegram::SendVideoVideo::Stream((Box::new(video.bytes_stream()), size)),
                            width: Some(result.width),
                            height: Some(result.height),
                            caption: url.to_string(),
                            reply_parameters: ReplyParameters {
                                message_id,
                                chat_id,
                                quote: url.to_string(),
                            },
                        })
                        .await?;
                    let file_id = message
                        .video
                        .context("App:webhook_handler: cannot get video from message")?
                        .file_id;
                    if let Err(err) = state.db.insert_video(url.as_str(), &file_id).await {
                        tracing::error!(message = "App:webhook_handler: cannot insert row to `videos` table", error = %err);
                    }
                    return Ok(());
                }
                .await;
                if let Err(err) = result {
                    error!("App:webhook_handler: {err}");
                    _ = state
                        .telegram
                        .send_message(SendMessage {
                            chat_id,
                            text: "❌ Cannot process video.".to_string(),
                            reply_parameters: ReplyParameters {
                                message_id,
                                chat_id,
                                quote: url.to_string(),
                            },
                        })
                        .await;
                }
            };
        })
        .buffer_unordered(5)
        .for_each(|_| async {})
        .await;
    return StatusCode::OK;
}

type ServerStateType = Arc<ServerState>;

struct ServerState {
    client: Arc<Client>,
    db: Db,
    telegram: Telegram,
    tiktok_resolver: TikTokResolver,
    twitter_resolver: TwitterResolver,
    instagram_resolver: InstagramResolver,
}
