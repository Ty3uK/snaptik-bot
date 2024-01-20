use anyhow::{anyhow, bail, Result};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::header::HeaderMap;
use serde::Deserialize;

lazy_static! {
    pub static ref CSRF_REGEX: Regex =
        Regex::new(r#"<input.+?name="csrf_token".+?value="(.+)".+?>"#).unwrap();
    pub static ref JSON_REGEX: Regex =
        Regex::new(r#"(?s)set_listener\(.+?(\[.+\]).+?"a""#).unwrap();
    pub static ref SESSION_REGEX: Regex = Regex::new(r#"(session=.+?;)"#).unwrap();
}

#[derive(Debug, Deserialize)]
struct Media {
    format_note: String,
    url: Option<String>,
}

type MediaList = (Vec<Media>, Vec<Media>);

pub fn get_media_url(html: &str) -> Result<String> {
    let capts = JSON_REGEX
        .captures(html)
        .ok_or(anyhow!("Cannot capture `json`"))?;
    if capts.len() == 0 {
        bail!("Cannot capture `json`");
    }
    let mut json = capts[1].to_string();
    json.insert(0, '[');
    json.push(']');
    let list: MediaList = serde_json::from_str(&json)?;
    let mut list = list.0;
    list.sort_by(|a, b| {
        if a.format_note == "1080p" || a.format_note == "720p" {
            return std::cmp::Ordering::Less;
        }
        if a.format_note == "1080p" && b.format_note == "720p" {
            return std::cmp::Ordering::Less;
        }
        std::cmp::Ordering::Equal
    });
    list[0].url.clone().ok_or(anyhow!("Cannot get media url"))
}

pub fn get_cookie(headers: &HeaderMap) -> Result<String> {
    let set_cookie = headers
        .get("set-cookie")
        .ok_or(anyhow!("Cannot get `cookie` header"))?
        .to_str()
        .map_err(|err| anyhow!(err))?;
    let capts = SESSION_REGEX
        .captures(set_cookie)
        .ok_or(anyhow!("Cannot capture `session`"))?;
    if capts.len() == 0 {
        bail!("Cannot capture `session`");
    }
    Ok(capts[1].to_string())
}
