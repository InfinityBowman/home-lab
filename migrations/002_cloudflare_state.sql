CREATE TABLE IF NOT EXISTS cloudflare_dns_records (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL UNIQUE,
    cf_record_id TEXT NOT NULL,
    hostname TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE
);
