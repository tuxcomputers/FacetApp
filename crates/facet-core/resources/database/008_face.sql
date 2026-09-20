-- face
-- The 12 physical faces of the TimeFlip device, then the app's own faces from 13 up, each linked to its
-- assigned category. Nothing above 12 is ever reported by a device; manual mode times on 13 and 14 and
-- alternates between them, so the face a finished segment names keeps its category while the next segment
-- runs (see ManualFace and the device_face note in 003_device_event.sql).

CREATE TABLE IF NOT EXISTS face (
  face_id       INTEGER CONSTRAINT PK_face PRIMARY KEY
  , category_id INTEGER NOT NULL REFERENCES category(category_id)
  , locked      INTEGER NOT NULL DEFAULT 0 CHECK (locked IN (0,1))
);

INSERT INTO face (face_id, category_id)
SELECT 1, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 1);
INSERT INTO face (face_id, category_id, locked)
SELECT 2, (SELECT category_id FROM category WHERE category_name = 'Meeting'), 1 WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 2);
INSERT INTO face (face_id, category_id)
SELECT 3, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 3);
INSERT INTO face (face_id, category_id)
SELECT 4, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 4);
INSERT INTO face (face_id, category_id)
SELECT 5, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 5);
INSERT INTO face (face_id, category_id)
SELECT 6, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 6);
INSERT INTO face (face_id, category_id)
SELECT 7, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 7);
INSERT INTO face (face_id, category_id, locked)
SELECT 8, (SELECT category_id FROM category WHERE category_name = 'Break'), 1 WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 8);
INSERT INTO face (face_id, category_id)
SELECT 9, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 9);
INSERT INTO face (face_id, category_id)
SELECT 10, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 10);
INSERT INTO face (face_id, category_id)
SELECT 11, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 11);
INSERT INTO face (face_id, category_id)
SELECT 12, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 12);
INSERT INTO face (face_id, category_id)
SELECT 13, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 13);
INSERT INTO face (face_id, category_id)
SELECT 14, (SELECT category_id FROM category WHERE category_name = 'Unassigned') WHERE NOT EXISTS (SELECT 1 FROM face WHERE face_id = 14);
