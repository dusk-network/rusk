-- Table for unfinalized blocks that are not yet finalized and could be removed again.
CREATE TABLE unfinalized_blocks (
    block_height INTEGER PRIMARY KEY NOT NULL,
    block_hash TEXT NOT NULL,
    UNIQUE (block_height, block_hash)
) STRICT;

CREATE UNIQUE INDEX unfinalized_block_height_idx ON unfinalized_blocks (block_height);
CREATE UNIQUE INDEX unfinalized_block_hash_idx ON unfinalized_blocks (block_hash);

-- unfinalized_blocks(1)->unfinalized_events(n)
-- Every row is a single event
CREATE TABLE unfinalized_events (
    id INTEGER PRIMARY KEY NOT NULL,
    block_height INTEGER NOT NULL,
    block_hash TEXT NOT NULL,
    origin TEXT NOT NULL, -- origin hash
    topic TEXT NOT NULL,
    source TEXT NOT NULL, -- contract address
    data BLOB NOT NULL,

    FOREIGN KEY (block_height) REFERENCES unfinalized_blocks(block_height)
    FOREIGN KEY (block_hash) REFERENCES unfinalized_blocks(block_hash)
) STRICT;

-- Every row is a single block
CREATE TABLE finalized_blocks (
    id INTEGER PRIMARY KEY NOT NULL,
    block_height INTEGER NOT NULL,
    block_hash TEXT NOT NULL,
    phoenix_present INTEGER NOT NULL,

    UNIQUE (block_height, block_hash)
    check(id = block_height)
    check(phoenix_present IN (0, 1))
) STRICT;

CREATE UNIQUE INDEX block_height_idx ON finalized_blocks (block_height);
CREATE UNIQUE INDEX block_hash_idx ON finalized_blocks (block_hash);

-- finalized_blocks(1)->finalized_events(n)
-- Every row is a single event
CREATE TABLE finalized_events (
    id INTEGER PRIMARY KEY NOT NULL,
    block_height INTEGER NOT NULL,
    block_hash TEXT NOT NULL,
    origin TEXT NOT NULL, -- origin hash
    topic TEXT NOT NULL,
    source TEXT NOT NULL, -- contract address
    data BLOB NOT NULL,

    FOREIGN KEY (block_height) REFERENCES finalized_blocks(block_height)
    FOREIGN KEY (block_hash) REFERENCES finalized_blocks(block_hash)
) STRICT;

CREATE INDEX events_block_height_idx ON finalized_events (block_height);
CREATE INDEX events_block_height_source_idx ON finalized_events (block_height, source);
CREATE INDEX origin_idx ON finalized_events (origin);
CREATE INDEX source_idx ON finalized_events (source);
