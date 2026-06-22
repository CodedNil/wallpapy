CREATE TABLE IF NOT EXISTS wallpapers (
    id TEXT PRIMARY KEY,
    datetime TEXT NOT NULL,
    prompt TEXT NOT NULL,
    shortened_prompt TEXT NOT NULL,
    image_file TEXT,
    image_width INTEGER,
    image_height INTEGER,
    image_brightness REAL,
    liked_state TEXT NOT NULL,
    comment TEXT
);
