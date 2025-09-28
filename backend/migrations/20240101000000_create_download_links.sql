-- Create download_links table
CREATE TABLE IF NOT EXISTS download_links (
    id TEXT PRIMARY KEY NOT NULL,
    object_key TEXT NOT NULL,
    bucket TEXT,
    expires_at TEXT NOT NULL,  -- ISO 8601 datetime string
    max_downloads INTEGER,
    downloads_served INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,  -- ISO 8601 datetime string
    download_filename TEXT
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_download_links_expires_at ON download_links(expires_at);
CREATE INDEX IF NOT EXISTS idx_download_links_created_at ON download_links(created_at);
