ALTER TABLE unfinalized_events
    ADD COLUMN reverted INTEGER NOT NULL DEFAULT 0 CHECK(reverted IN (0, 1));

ALTER TABLE finalized_events
    ADD COLUMN reverted INTEGER NOT NULL DEFAULT 0 CHECK(reverted IN (0, 1));
