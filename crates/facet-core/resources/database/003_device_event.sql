-- device_event
-- One row per timing segment (a face flip or pause), as reported by the device or by the app itself.

CREATE TABLE IF NOT EXISTS device_event (
  device_event_id    INTEGER CONSTRAINT PK_device_event PRIMARY KEY AUTOINCREMENT
  , event_number     INTEGER NOT NULL
  , event_type_id    INTEGER NOT NULL REFERENCES event_type(event_type_id)
  -- 1-12 are the cube's own faces and nothing above 12 ever comes from a device. 13 upwards belong to
  -- the app: manual mode times on them, and it uses more than one deliberately (see ManualFace). A
  -- segment names a face, and the face's category is what says whose time it was, so a face reassigned
  -- while a finished segment still awaits its time_entry would change the answer under it. Consecutive
  -- manual segments therefore land on different faces, which is the same reason a cube never has this
  -- problem: flipping from one face to another leaves the first one's mapping alone.
  , device_face      INTEGER NOT NULL CHECK (device_face BETWEEN 1 AND 14)
  , start_time       TEXT NOT NULL
  , timezone_id      INTEGER NOT NULL DEFAULT 0 REFERENCES timezone(timezone_id)
  , start_epoch      INTEGER NOT NULL
  , duration_seconds REAL NOT NULL CHECK (duration_seconds >= 0)
  , paused           INTEGER NOT NULL CHECK (paused IN (0,1))
  , finalised        INTEGER NOT NULL DEFAULT 0 CHECK (finalised IN (0,1))
  , processed        INTEGER NOT NULL DEFAULT 0 CHECK (processed IN (0,1))
);

CREATE UNIQUE INDEX IF NOT EXISTS UN1_device_event ON device_event(event_number, start_epoch);
CREATE INDEX IF NOT EXISTS IN1_device_event ON device_event(start_epoch);
