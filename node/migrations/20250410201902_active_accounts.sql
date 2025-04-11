CREATE TABLE IF NOT EXISTS active_accounts (
    id INTEGER PRIMARY KEY NOT NULL,
    public_key TEXT NOT NULL,
    UNIQUE (public_key)
) STRICT;
