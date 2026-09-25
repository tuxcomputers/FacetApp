-- event_type
-- Reference table of the different event types the TimeFlip device can trigger.

CREATE TABLE IF NOT EXISTS event_type (
  event_type_id INTEGER CONSTRAINT PK_event_type PRIMARY KEY
  , event_name  TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS UN1_event_type ON event_type(event_name);

INSERT INTO event_type (event_type_id, event_name)
SELECT 1, 'face_flip' WHERE NOT EXISTS (SELECT 1 FROM event_type WHERE event_type_id = 1);
INSERT INTO event_type (event_type_id, event_name)
SELECT 2, 'pause' WHERE NOT EXISTS (SELECT 1 FROM event_type WHERE event_type_id = 2);

INSERT INTO event_type (event_type_id, event_name)
SELECT 3, 'double_tap' WHERE NOT EXISTS (SELECT 1 FROM event_type WHERE event_type_id = 3);
INSERT INTO event_type (event_type_id, event_name)
SELECT 4, 'auto_pause_minutes' WHERE NOT EXISTS (SELECT 1 FROM event_type WHERE event_type_id = 4);
INSERT INTO event_type (event_type_id, event_name)
SELECT 5, 'battery_level' WHERE NOT EXISTS (SELECT 1 FROM event_type WHERE event_type_id = 5);
INSERT INTO event_type (event_type_id, event_name)
SELECT 6, 'system_state' WHERE NOT EXISTS (SELECT 1 FROM event_type WHERE event_type_id = 6);
INSERT INTO event_type (event_type_id, event_name)
SELECT 7, 'device_info' WHERE NOT EXISTS (SELECT 1 FROM event_type WHERE event_type_id = 7);
INSERT INTO event_type (event_type_id, event_name)
SELECT 8, 'event_log' WHERE NOT EXISTS (SELECT 1 FROM event_type WHERE event_type_id = 8);
