-- Single-row table tracking current supply statistics
-- This table always contains exactly one row (id=1) that gets updated
CREATE TABLE IF NOT EXISTS supply_info (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),   -- Constrained to 1 to enforce single-row table
    block_height INTEGER NOT NULL,                    -- Current block height of these stats
    total_supply REAL NOT NULL,                       -- Total coins created minus burned coins (floating point)
    circulating_supply REAL NOT NULL,                 -- Coins in public circulation (floating point)
    max_supply REAL NOT NULL,                         -- Theoretical maximum coins that can exist minus burned (floating point)
    burned REAL NOT NULL,                             -- Total coins verifiably burned (floating point)
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL  -- Last update timestamp
);

-- Insert the initial row with default values
INSERT INTO supply_info (id, block_height, total_supply, circulating_supply, max_supply, burned)
VALUES (1, 0, 500000000.0, 500000000.0, 1000000000.0, 0.0);
