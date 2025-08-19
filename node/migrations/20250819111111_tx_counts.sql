CREATE INDEX IF NOT EXISTS finalized_events_source_topic_idx
    ON finalized_events (source, topic);