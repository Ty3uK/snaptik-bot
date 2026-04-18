use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config<'a> {
    #[serde(borrow)]
    pub sqlite: SQLite<'a>,
    #[serde(borrow)]
    pub telegram: TelegramConfig<'a>,
    #[serde(borrow)]
    pub instagram: Option<InstagramConfig<'a>>,
}

#[derive(Debug, Deserialize)]
pub struct SQLite<'a> {
    pub path: &'a str,
}

#[derive(Debug, Deserialize, Default)]
pub struct TelegramConfig<'a> {
    #[serde(borrow)]
    pub token: &'a str,
    #[serde(borrow)]
    pub webhook_url: &'a str,
}

#[derive(Debug, Deserialize, Default)]
pub struct InstagramConfig<'a> {
    #[serde(borrow)]
    pub session_id: &'a str,
}
