use reqwest::Client;
use worker::*;
use serde::{Deserialize, Serialize};

mod telegram;
mod snaptik;

use telegram::Telegram;
use snaptik::Snaptik;

#[derive(Deserialize, Serialize)]
struct RouterData {
    api_path: String,
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let api_path = match env.secret("BOT_TOKEN") {
        Ok(token) => format!("https://api.telegram.org/bot{}", token.to_string()),
        Err(err) => panic!("{}", err),
    };

    let router_data = RouterData {
        api_path,
    };

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::with_data(router_data);

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get_async("/api/webhook", |_, ctx| async move {
            let client = Client::new();
            let tg_client = Telegram::new(&client, &ctx.data.api_path);
            let webhook = telegram::SetWebhook {
                url: String::from("https://snaptik-bot.ty3uk.workers.dev/api/update"),
            };

            match tg_client.set_webhook(&webhook).await {
                Ok(_) => (),
                Err(err) => console_error!("{}", err),
            }

            Response::ok("Success")
        })
        .post_async("/api/update", |mut req, ctx| async move {
            let client = Client::new();
            let tg_client = Telegram::new(&client, &ctx.data.api_path);
            let mut snaptik_client = Snaptik::new(&client);

            let update = match req.json::<telegram::Update>().await {
                Ok(data) => data.message,
                Err(err) => {
                    console_error!("{}", err);
                    return Response::ok("");
                },
            };
            let chat_id = update.chat.unwrap().id;
            let message_text = &update.text.unwrap();

            if message_text == "/start" {
                let message = telegram::SendMessage {
                    chat_id,
                    text: String::from("Привет! Я бот, который позволяет скачивать видео из TikTok.\n\nПришли мне ссылку, а в ответ я пришлю видео."),
                    reply_to_message_id: None,
                };

                match tg_client.send_message(&message).await {
                    Ok(_) => (),
                    Err(err) => console_error!("{}", err),
                }

                return Response::ok("");
            }

            let send_bad_url_message = || async {
                let message = telegram::SendMessage {
                    chat_id,
                    text: String::from("Необходимо прислать ссылку на видео из TikTok"),
                    reply_to_message_id: update.message_id,
                };

                match tg_client.send_message(&message).await {
                    Ok(_) => (),
                    Err(err) => console_error!("{}", err),
                };
            };

            let url = match Url::parse(message_text) {
                Ok(url) => url,
                Err(_) => {
                    send_bad_url_message().await;
                    return Response::ok("");
                },
            };

            let is_correct_host = match url.host_str() {
                Some(host) => host.ends_with(".tiktok.com"),
                None => false,
            };

            if !is_correct_host {
                send_bad_url_message().await;
                return Response::ok("");
            }

            let url_str = url.to_string();

            let send_service_error_message = || async {
                let message = telegram::SendMessage {
                    chat_id,
                    text: String::from("Сервис временно недоступен"),
                    reply_to_message_id: update.message_id,
                };

                match tg_client.send_message(&message).await {
                    Ok(_) => (),
                    Err(err) => console_error!("{}", err),
                }
            };

            let has_token = match snaptik_client.get_token().await {
                Ok(has_token) => has_token,
                Err(err) => {
                    console_error!("{}", err);
                    send_service_error_message().await;
                    return Response::ok("");
                },
            };

            if !has_token {
                send_service_error_message().await;
                return Response::ok("");
            }

            let tiktok_url = match snaptik_client.get_tiktok_url(&url_str).await {
                Ok(url) => url,
                Err(err) => {
                    console_error!("{}", err);
                    send_service_error_message().await;
                    return Response::ok("");
                },
            };

            if tiktok_url.is_empty() {
                send_service_error_message().await;
                return Response::ok("");
            }

            let video = telegram::SendVideo {
                chat_id,
                video: tiktok_url,
                reply_to_message_id: update.message_id,
            };

            if let Some(err) = tg_client.send_video(&video).await.err() {
                console_error!("{}", err);
            }

            Response::ok("")
        })
        .run(req, env)
    .await
}
