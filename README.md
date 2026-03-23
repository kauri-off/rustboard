# rustboard

A JavaScript free imageboard engine written in Rust.

## Features

- Multiple boards with slugs (e.g. `/b/`, `/g/`)
- Threads with image uploads (JPEG, PNG, GIF, WebP)
- IP-based rate limiting with hashed IP storage
- Password-protected admin panel
- Configurable via `config.toml`

## Configuration

All settings are optional and have defaults. Edit `config.toml` to override them.

**Before deploying, change `site.ip_salt` and `admin.password`**

Generate a secret with:

```sh
openssl rand -hex 32
```

## Admin Panel

The admin panel is available at `/admin` and protected by password authentication. The server will refuse to start if `admin.password` is left as the default `"changeme"`.

### What the admin can do

- **Manage boards** — create and delete boards. Changes apply immediately without restarting the server.
- **Manage posts** — delete threads and individual posts. Deleting a thread cascades to all its replies and cleans up uploaded image files from disk.
- **Edit site settings** — change the site name, threads per board, post cooldown, and image/content size limits. All changes are persisted to SQLite and take effect instantly.

### Security

- Session-based authentication with UUID tokens (24-hour expiry, `HttpOnly` + `SameSite=Strict` cookies).
- Login rate limiting: 5 failed attempts lock the IP out for 15 minutes.

## Running

```sh
cargo build --release
```

The server reads `config.toml` from the working directory. Pass `--config` to use a different file:

```sh
./target/release/rustboard --config /etc/rustboard/config.toml
```

The server starts on `http://0.0.0.0:3000` by default.

# License

[GPLv3](LICENSE)
