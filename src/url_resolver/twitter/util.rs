use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub static ref DOWNLOAD_LINK_REGEX: Regex = Regex::new(r#"<a.+?href=\\?"(.+?)\\?""#).unwrap();
}
