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
- Configurable via environment variables

## Running

```sh
cargo run
```

The server starts on `http://0.0.0.0:3000` by default.

## Configuration

All settings are optional and have defaults.

| Variable            | Default                    | Description                        |
|---------------------|----------------------------|------------------------------------|
| `DATABASE_URL`      | `sqlite:rustboard.db`      | SQLite database path               |
| `UPLOAD_DIR`        | `./uploads`                | Directory for uploaded images      |
| `BIND_ADDR`         | `0.0.0.0:3000`             | Address and port to listen on      |
| `SITE_NAME`         | `Rustboard`                | Displayed site name                |
| `SITE_URL`          | *(empty)*                  | Public base URL for OG/meta tags   |
| `IP_SALT`           | `default-salt-change-me`   | Salt for hashing poster IPs        |
| `MAX_IMAGE_BYTES`   | `5242880` (5 MB)           | Max upload size in bytes           |
| `MAX_IMAGE_WIDTH`   | `10000`                    | Max image width in pixels          |
| `MAX_IMAGE_HEIGHT`  | `10000`                    | Max image height in pixels         |
| `POST_COOLDOWN_SECS`| `30`                       | Seconds between posts per IP       |
| `MAX_SUBJECT_CHARS` | `200`                      | Max subject line length            |
| `MAX_CONTENT_CHARS` | `2000`                     | Max post body length               |

Copy `example.env` to `.env` (or export vars directly) to override defaults. **Change `IP_SALT` before deploying.**
