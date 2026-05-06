-- Docker-compose services (n8n, plausible, etc.)
CREATE TABLE IF NOT EXISTS services (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    compose_path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Encrypted secrets for docker-compose services
CREATE TABLE IF NOT EXISTS service_secrets (
    id TEXT PRIMARY KEY,
    service_id TEXT NOT NULL,
    key TEXT NOT NULL,
    encrypted_value BLOB NOT NULL,
    nonce BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE CASCADE,
    UNIQUE(service_id, key)
);

CREATE INDEX IF NOT EXISTS idx_service_secrets_service_id ON service_secrets(service_id);

-- Add encryption columns to existing env_vars table
ALTER TABLE env_vars ADD COLUMN encrypted_value BLOB;
ALTER TABLE env_vars ADD COLUMN nonce BLOB;
