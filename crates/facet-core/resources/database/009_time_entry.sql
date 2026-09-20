-- time_entry
-- A single tracked time span, linked to the category it was logged against.

CREATE TABLE IF NOT EXISTS time_entry (
  time_entry_id               INTEGER CONSTRAINT PK_time_entry PRIMARY KEY AUTOINCREMENT
  , category_id               INTEGER NOT NULL REFERENCES category(category_id)
  , device_event_id           INTEGER NOT NULL REFERENCES device_event(device_event_id)
  , started_at                TEXT NOT NULL
  , start_timezone_id         INTEGER NOT NULL DEFAULT 0 REFERENCES timezone(timezone_id)
  , ended_at                  TEXT NOT NULL
  , end_timezone_id           INTEGER NOT NULL DEFAULT 0 REFERENCES timezone(timezone_id)
  , duration_seconds          REAL NOT NULL CHECK (duration_seconds >= 0)
  , total_cost                INTEGER NOT NULL DEFAULT 0
  , synced_to_google_calendar INTEGER NOT NULL DEFAULT 0 CHECK (synced_to_google_calendar IN (0,1))
);

CREATE UNIQUE INDEX IF NOT EXISTS UN1_time_entry ON time_entry(device_event_id);
