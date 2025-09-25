CREATE INDEX IF NOT EXISTS finalized_events_source_id_idx
  ON finalized_events (source, id);
