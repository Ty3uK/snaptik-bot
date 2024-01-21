# snaptik-bot

A bot that can download videos from:

- TikTok
- Instagram
- Youtube Shorts.

## Deployment

You need to set several secrets in Cloudflare Worker settings:

- `BOT_TOKEN` - token from (@BotFather)[https://t.me/BotFather]
- `LIBSQL_CLIENT_TOKEN` - token from any libSQL provider
- `LIBSQL_CLIENT_URL` - connection URL from any libSQL provider (HTTP-operated)

```bash
npx wrangler deploy
```
## Built With

  - [rust](https://www.rust-lang.org) - A language empowering everyone to build reliable and efficient software
  - [workers-rs](https://github.com/cloudflare/workers-rs) - A Rust SDK for writing Cloudflare Workers
  - [serde](https://crates.io/crates/serde) - A generic serialization/deserialization framework
  - [serde_json](https://crates.io/crates/serde_json) - A JSON serialization file format
  - [reqwest](https://crates.io/crates/reqwest) - Higher level HTTP client library
  - [url](https://crates.io/crates/url) - URL library for Rust, based on the WHATWG URL Standard
  - [regex](https://crates.io/crates/regex) - An implementation of regular expressions for Rust
  - [lazy_static](https://crates.io/crates/lazy_static) - A macro for declaring lazily evaluated statics in Rust
  - [anyhow](https://crates.io/crates/anyhow) - Flexible concrete Error type built on std::error::Error
  - [libsql_client](https://crates.io/crates/libsql-client) - HTTP-based client for libSQL and sqld

## License

This project is licensed under the [MIT](LICENSE.md) - see the [LICENSE.md](LICENSE.md) file for details

