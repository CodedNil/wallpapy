CREATE TABLE IF NOT EXISTS wallpapers (
    id TEXT PRIMARY KEY,
    datetime TEXT NOT NULL,
    prompt TEXT NOT NULL,
    shortened_prompt TEXT NOT NULL,
    file_name TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    brightness REAL NOT NULL,
    liked_state TEXT NOT NULL DEFAULT 'Neutral',
    comment TEXT
);
