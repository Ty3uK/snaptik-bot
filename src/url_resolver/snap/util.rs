use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub static ref TOKEN_REGEX: Regex =
        Regex::new(r#"<input name="token" value="(.+?)" .+?>"#).unwrap();
}

lazy_static! {
    pub static ref DECODER_ARGS_REGEX: Regex =
        Regex::new(r#"\("(.+?)",(\d+),"(.+?)",(\d+),(\d+),(\d+)\)"#).unwrap();
}

lazy_static! {
    pub static ref RESULT_VIDEO_URL_REGEX: Regex = Regex::new(
        r#"href=\\?"(https://(.*?\.)?(snaptik\.app|snapinsta\.app|rapidcdn\.app)/.*?)\\?""#
    )
    .unwrap();
}

pub fn decode(h: &str, _u: usize, n: &str, t: u32, e: usize, _r: usize) -> String {
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
            Ok(result) => match u32::try_from(result) {
                Ok(result) => result,
                Err(err) => panic!("{}", err),
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
