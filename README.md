# rustboard

A lightweight imageboard server written in Rust.

## Stack

- **[Axum](https://github.com/tokio-rs/axum)** — async HTTP framework
- **[SQLite](https://www.sqlite.org/) via sqlx** — persistent storage with auto-migrations
- **[Askama](https://github.com/djc/askama)** — compile-time HTML templating
- **Tokio** — async runtime

## Features

- Multiple boards with slugs (e.g. `/b/`, `/g/`)
- Threads with image uploads (JPEG, PNG, GIF, WebP)
- IP-based rate limiting with hashed IP storage
- Configurable via `config.toml`

## Running

```sh
cargo run
```

The server reads `config.toml` from the working directory. Pass `--config` to use a different file:

```sh
./rustboard --config /etc/rustboard/config.toml
```

The server starts on `http://0.0.0.0:3000` by default.

## Configuration

All settings are optional and have defaults. Edit `config.toml` to override them.

**Before deploying, change `site.ip_salt` and `admin.password`**

Generate a secret with:

```sh
openssl rand -hex 32
```

# License

[GPLv3](LICENSE)
