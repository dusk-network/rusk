CREATE INDEX IF NOT EXISTS finalized_events_block_hash_idx
  ON finalized_events (block_hash);

CREATE INDEX IF NOT EXISTS unfinalized_events_block_hash_idx
  ON unfinalized_events (block_hash);

CREATE INDEX IF NOT EXISTS unfinalized_events_block_height_idx
  ON unfinalized_events (block_height);

CREATE INDEX IF NOT EXISTS finalized_blocks_phx1_height_idx
  ON finalized_blocks (block_height)
  WHERE phoenix_present = 1;
