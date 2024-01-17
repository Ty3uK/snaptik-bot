use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{Client, Error};
use worker::console_warn;

pub struct Snaptik<'a> {
    boundary: String,
    client: &'a Client,
    token: String,
}

impl<'a> Snaptik<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            boundary: String::from("----WebKitFormBoundary68bnlPZvgaiO3elG"),
            token: String::from(""),
        }
    }
    
    fn get_multipart_content(&self, url: &String) -> String {
        format!(include_str!("../assets/multipart.txt"), boundary = self.boundary, url = url, token = self.token)
    }

    pub async fn get_token(&mut self) -> Result<bool, Error> {
        let html = self.client.get("https://snaptik.app/en")
            .send()
            .await?
            .text()
            .await?;

        lazy_static! {
            static ref TOKEN_REGEX: Regex = Regex::new(r#"<input name="token" value="(.+?)" .+?>"#).unwrap();
        };

        let capts = TOKEN_REGEX.captures(&html).unwrap();
        if capts.len() > 0 {
            self.token = capts[1].to_string();
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn get_tiktok_url(&self, url: &String) -> Result<String, Error> {
        let content = self.get_multipart_content(url);
        let js = self.client.post("https://snaptik.app/abc2.php")
            .body(content.clone())
            .header("host", "snaptik.app")
            .header("content-type", format!("multipart/form-data; boundary={}", self.boundary))
            .header("content-length", content.len().to_string())
            .send()
            .await?
            .text()
            .await?;

        lazy_static! {
            static ref DECODER_ARGS_REGEX: Regex = Regex::new(r#"\("(.+?)",(\d+),"(.+?)",(\d+),(\d+),(\d+)\)"#).unwrap();
        };

        let capts = DECODER_ARGS_REGEX.captures(&js).unwrap();
        let decoded_str = Snaptik::<'_>::decode(
            capts[1].to_string(),
            capts[2].parse().unwrap(),
            capts[3].to_string(),
            capts[4].parse().unwrap(),
            capts[5].parse().unwrap(),
            capts[6].parse().unwrap(),
        );

        lazy_static! {
            static ref TIKTOK_URL_REGEX: Regex = Regex::new(r#""((https://cdn.snaptik.app/|https://d.rapidcdn.app/).+?)""#).unwrap();
        }

        if let Some(capts) = TIKTOK_URL_REGEX.captures(&decoded_str) {
            if capts.len() > 0 {
                return Ok(capts[1].to_string());
            }
        } else {
            console_warn!("{decoded_str}");
        }

        Ok(String::from(""))
    }

    fn decode(h: String, _u: usize, n: String, t: u32, e: usize, _r: usize) -> String {
        let mut result = String::from("");
        let h_chars: Vec<char> = h.chars().collect();
        let n_chars: Vec<char> = n.chars().collect();

        let mut i = 0;
        while i < h.len() {
            let mut s = String::from("");

            loop {
                let h_ch = match h_chars.get(i) {
                    Some(ch) => ch,
                    None => panic!("Cannot get char by index: {}", i),
                };
                let n_ch = match n_chars.get(e) {
                    Some(ch) => ch,
                    None => panic!("Cannot get char by index: {}", e),
                };

                if h_ch == n_ch {
                    break;
                }

                s.push(*h_ch);
                i += 1;
            }

            let mut j = 0;
            while j < n.len() {
                let ch = match n_chars.get(j) {
                    Some(ch) => ch,
                    None => panic!("Cannot get char by index: {}", j),
                };
                s = s.replace(*ch, &j.to_string());
                j += 1;
            }

            i += 1;

            let e_u32: u32 = match e.try_into() {
                Ok(e) => e,
                Err(err) => panic!("{}", err),
            };
            let char_code = match usize::from_str_radix(&s, e_u32) {
                Ok(result) => {
                    match u32::try_from(result) {
                        Ok(result) => result,
                        Err(err) => panic!("{}", err),
                    }
                },
                Err(err) => panic!("{}", err),
            } - t;

            let char = match char::from_u32(char_code) {
                Some(result) => result,
                None => panic!("Cannot create char by code: {}", char_code),
            };

            result.push(char);
        }

        result
    }
}
