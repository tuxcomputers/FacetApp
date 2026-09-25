-- debug_log
-- Every dev-only debug message, recorded when the `debug` setting is enabled.

CREATE TABLE IF NOT EXISTS debug_log (
  debug_log_id  INTEGER CONSTRAINT PK_debug_log PRIMARY KEY AUTOINCREMENT
  , logged_at   TEXT NOT NULL
  , timezone_id INTEGER NOT NULL DEFAULT 0 REFERENCES timezone(timezone_id)
  , tag         TEXT NOT NULL
  , message     TEXT NOT NULL
);
