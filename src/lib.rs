use db::Db;
use serde::{Deserialize, Serialize};
use url_resolver::{
    shorts::ShortsUrlResolver, snap::SnapUrlResolver, Platform, ResolveUrl, UrlResolver,
};
use worker::*;

mod db;
mod telegram;
mod url_resolver;

use telegram::{DeleteMessage, EditMessageText, Telegram};

#[derive(Deserialize, Serialize)]
struct RouterData {
    api_path: String,
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let api_path = env
        .secret("BOT_TOKEN")
        .map(|token| format!("https://api.telegram.org/bot{}", token.to_string()))
        .unwrap();

    Router::with_data(RouterData { api_path })
        .get_async("/api/webhook", setup_webhook)
        .post_async("/api/update", process_update)
        .run(req, env)
        .await
}

async fn setup_webhook(_req: Request, ctx: RouteContext<RouterData>) -> Result<Response> {
    let http_client = reqwest::Client::new();
    let tg_client = Telegram::new(&http_client, &ctx.data.api_path);
    let webhook = telegram::SetWebhook {
        url: "https://snaptik-bot.ty3uk.workers.dev/api/update",
    };

    match tg_client.set_webhook(&webhook).await {
        Ok(_) => (),
        Err(err) => console_error!("{}", err),
    }

    Response::ok("Success")
}

async fn process_update(mut req: Request, ctx: RouteContext<RouterData>) -> Result<Response> {
    let db = Db::new(&ctx.env);
    let http_client = reqwest::Client::new();
    let tg_client = Telegram::new(&http_client, &ctx.data.api_path);

    let update = match req.json::<telegram::Update>().await {
        Ok(data) => match data.message {
            Some(data) => data,
            None => return Response::ok(""),
        },
        Err(err) => {
            console_error!("{}", err);
            return Response::ok("");
        }
    };

    let chat = match &update.chat {
        Some(chat) => chat,
        None => return Response::ok(""),
    };

    let mut message_text = match update.text {
        Some(text) => text,
        None => return Response::ok(""),
    };

    if message_text.is_empty() {
        return Response::ok("");
    }

    if chat.chat_type != telegram::ChatType::Private {
        if message_text != "@SnapTikRsBot" {
            return Response::ok("");
        }

        if update.reply_to_message.is_none() {
            return Response::ok("");
        }

        let update = *update.reply_to_message.unwrap();
        message_text = update.text.unwrap();
    }

    if !message_text.ends_with('/') {
        message_text.push('/');
    }

    if message_text == "/start" {
        let message = telegram::SendMessage {
            chat_id: chat.id,
            text: String::from("Привет! Я бот, который позволяет скачивать видео из TikTok.\n\nПришли мне ссылку, а в ответ я пришлю видео."),
            reply_to_message_id: None,
        };

        if let Err(err) = tg_client.send_message(&message).await {
            console_error!("{err}");
        }

        return Response::ok("");
    }

    let message_to_edit = match tg_client
        .send_message(&telegram::SendMessage {
            chat_id: chat.id,
            text: "⏱️  Processing...".to_string(),
            reply_to_message_id: update.message_id,
        })
        .await
    {
        Ok(message) => message,
        Err(err) => {
            console_error!("{err}");
            return Response::ok("");
        }
    };

    let send_bad_url_message = || async {
        if let Err(err) = tg_client
            .edit_message_text(&EditMessageText {
                chat_id: chat.id,
                message_id: message_to_edit.message_id,
                text: "❌ Only TikTok, Instagram or Shorts links are accepted.".to_string(),
            })
            .await
        {
            console_error!("{err}");
        }
    };

    let url = match Url::parse(&message_text) {
        Ok(url) => url,
        Err(_) => {
            send_bad_url_message().await;
            return Response::ok("");
        }
    };

    if let Some(db) = &db {
        match db.get_video_file_id(&message_text).await {
            Ok(Some(file_id)) => {
                if let Err(err) = tg_client
                    .send_video(&telegram::SendVideo {
                        chat_id: chat.id,
                        video: file_id,
                        reply_to_message_id: update.message_id,
                        caption: Some(message_text.clone()),
                    })
                    .await
                {
                    console_error!("{err}");
                }

                if let Err(err) = tg_client
                    .delete_message(&DeleteMessage {
                        chat_id: chat.id,
                        message_id: message_to_edit.message_id.unwrap(),
                    })
                    .await
                {
                    console_error!("{err}");
                }

                return Response::ok("");
            }
            Err(err) => console_error!("`db.get_video_file_id` error: {err}"),
            _ => (),
        }
    }

    let platform = match Platform::new(url.host_str()) {
        Ok(url) => url,
        Err(err) => {
            console_error!("{err}: {url}");
            send_bad_url_message().await;
            return Response::ok("");
        }
    };

    let resolver: UrlResolver = match platform {
        Platform::TikTok => UrlResolver::TikTok(SnapUrlResolver::new(&http_client, &platform)),
        Platform::Instagram => {
            UrlResolver::Instagram(SnapUrlResolver::new(&http_client, &platform))
        }
        Platform::Shorts => UrlResolver::Shorts(ShortsUrlResolver::new(&http_client)),
    };

    let url = match resolver.resolve_url(url.as_str()).await {
        Ok(mut url) => {
            let query_pairs: Vec<_> = url
                .query_pairs()
                .filter(|(key, _)| key != "dl")
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect();
            url.query_pairs_mut().clear().extend_pairs(query_pairs);
            url
        }
        Err(err) => {
            console_error!("{err}");

            if let Err(err) = tg_client
                .edit_message_text(&EditMessageText {
                    chat_id: chat.id,
                    message_id: message_to_edit.message_id,
                    text: "❌ Cannot process video.".to_string(),
                })
                .await
            {
                console_error!("{err}");
            }

            return Response::ok("");
        }
    };

    if let Err(err) = tg_client
        .delete_message(&DeleteMessage {
            chat_id: chat.id,
            message_id: message_to_edit.message_id.unwrap(),
        })
        .await
    {
        console_error!("{err}");
    }

    let video = tg_client
        .send_video(&telegram::SendVideo {
            chat_id: chat.id,
            video: url.to_string(),
            reply_to_message_id: update.message_id,
            caption: Some(message_text.clone()),
        })
        .await
        .map_or_else(
            |err| {
                console_error!("{err}");
                None
            },
            |it| it.video,
        );

    if let (Some(db), Some(video)) = (&db, &video) {
        if let Err(err) = db.insert_video_file_id(&message_text, &video.file_id).await {
            console_error!("`db.insert_video_file_id` error: {err}")
        }
    };

    Response::ok("")
}
