-- 1. Полностью отключаем foreign keys для этой сессии
PRAGMA foreign_keys = OFF;

-- 2. Создаем временную копию постов, в которой НЕТ внешних ключей (REFERENCES)
-- Это гарантирует, что никакие действия с threads не затронут эти данные
CREATE TABLE posts_shadow AS SELECT * FROM posts;

-- 3. Теперь, когда данные постов в безопасности в 'posts_shadow', 
-- делаем стандартную замену таблицы threads
ALTER TABLE threads RENAME TO threads_old;

CREATE TABLE threads (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    board_id    INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    subject     TEXT    NOT NULL DEFAULT '',
    content     TEXT    NOT NULL DEFAULT '',
    image_path  TEXT, 
    ip_hash     TEXT    NOT NULL DEFAULT '',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    bump_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    post_count  INTEGER NOT NULL DEFAULT 0
);

INSERT INTO threads (id, board_id, subject, content, image_path, ip_hash, created_at, bump_at, post_count)
SELECT id, board_id, subject, content, image_path, ip_hash, created_at, bump_at, post_count 
FROM threads_old;

-- 4. Удаляем старые таблицы. Даже если сработает CASCADE на оригинальный 'posts', 
-- наши данные лежат в 'posts_shadow'.
DROP TABLE threads_old;
DROP TABLE posts;

-- 5. Воссоздаем таблицу 'posts' с правильными связями
CREATE TABLE posts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id   INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    content     TEXT    NOT NULL,
    image_path  TEXT,
    ip_hash     TEXT    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- 6. Возвращаем данные из тени
INSERT INTO posts (id, thread_id, content, image_path, ip_hash, created_at)
SELECT id, thread_id, content, image_path, ip_hash, created_at FROM posts_shadow;

-- 7. Чистим временные данные и восстанавливаем индексы
DROP TABLE posts_shadow;

CREATE INDEX IF NOT EXISTS idx_threads_board_bump ON threads(board_id, bump_at DESC);
CREATE INDEX IF NOT EXISTS idx_posts_thread ON posts(thread_id);

PRAGMA foreign_keys = ON;