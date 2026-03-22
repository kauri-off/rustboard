CREATE TABLE IF NOT EXISTS boards (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    slug        TEXT    NOT NULL UNIQUE,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS threads (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    board_id    INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    subject     TEXT    NOT NULL DEFAULT '',
    content     TEXT    NOT NULL DEFAULT '',
    image_path  TEXT    NOT NULL,
    ip_hash     TEXT    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    bump_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    post_count  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_threads_bump ON threads(board_id, bump_at DESC);

CREATE TABLE IF NOT EXISTS posts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id   INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    content     TEXT    NOT NULL,
    image_path  TEXT,
    ip_hash     TEXT    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_posts_thread ON posts(thread_id);

INSERT OR IGNORE INTO boards (slug, name, description) VALUES
    ('b', 'Random',     'Anything goes.'),
    ('g', 'Technology', 'Computers and technology.'),
    ('a', 'Anime',      'Anime and manga.');
