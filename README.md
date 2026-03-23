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

**Before deploying, change `site.ip_salt`** — it is used to hash poster IP addresses. Using the default value means anyone could reverse-engineer which posts share an IP. Generate a secret with:

```sh
openssl rand -hex 32
```

### `[server]`

| Key         | Default         | Description                              |
|-------------|-----------------|------------------------------------------|
| `bind_addr` | `0.0.0.0:3000`  | Address and port to listen on            |
| `log_level` | `info`          | Log verbosity: `error`, `warn`, `info`, `debug`, `trace` |

### `[database]`

| Key          | Default               | Description                        |
|--------------|-----------------------|------------------------------------|
| `url`        | `sqlite:rustboard.db` | SQLite database file path          |
| `upload_dir` | `./uploads`           | Directory for uploaded images      |

### `[site]`

| Key       | Default      | Description                              |
|-----------|--------------|------------------------------------------|
| `name`    | `Rustboard`  | Displayed site name                      |
| `url`     | *(empty)*    | Public base URL for OG/meta tags         |
| `ip_salt` | *(placeholder)* | Salt for hashing poster IPs — **change this** |

### `[limits]`

| Key                 | Default      | Description                               |
|---------------------|--------------|-------------------------------------------|
| `max_image_bytes`   | `5242880`    | Max upload size in bytes (5 MB)           |
| `max_image_width`   | `10000`      | Max image width in pixels                 |
| `max_image_height`  | `10000`      | Max image height in pixels                |
| `post_cooldown_secs`| `30`         | Seconds between posts per IP              |
| `max_subject_chars` | `200`        | Max subject line length                   |
| `max_content_chars` | `2000`       | Max post body length                      |
| `threads_per_board` | `100`        | Max threads shown per board               |
