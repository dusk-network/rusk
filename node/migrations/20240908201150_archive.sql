-- Sqlite schema for the archive table
CREATE TABLE archive (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    block_height INTEGER NOT NULL,
    block_hash TEXT NOT NULL,
    -- Json array of contract events emitted within the block
    json_contract_events TEXT NOT NULL DEFAULT '[]',
    finalized BOOLEAN
);

CREATE UNIQUE INDEX block_height_idx ON archive (block_height);
CREATE UNIQUE INDEX block_hash_idx ON archive (block_hash);
