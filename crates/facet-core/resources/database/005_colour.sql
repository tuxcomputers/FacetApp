-- colour
-- Reference table of the colours available to assign to a category.

CREATE TABLE IF NOT EXISTS colour (
  colour_id     INTEGER CONSTRAINT PK_colour PRIMARY KEY
  , colour_name TEXT NOT NULL
  , device_hex  TEXT
  , white_lines INTEGER NOT NULL DEFAULT 0 CHECK (white_lines IN (0,1))
);

CREATE UNIQUE INDEX IF NOT EXISTS UN1_colour ON colour(colour_name);

INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 0, 'None', NULL, 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 0);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 1, 'Red', '#ff0000', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 1);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 2, 'Maroon', '#800000', 1 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 2);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 3, 'Brown', '#a52a2a', 1 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 3);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 4, 'Tan', '#d2b48c', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 4);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 5, 'Orange', '#ffa500', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 5);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 6, 'Peach', '#ffdab9', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 6);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 7, 'Gold', '#ffd700', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 7);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 8, 'Yellow', '#ffff00', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 8);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 9, 'Lime', '#00ff00', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 9);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 10, 'Olive', '#808000', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 10);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 11, 'Green', '#008000', 1 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 11);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 12, 'Teal', '#008080', 1 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 12);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 13, 'Cyan', '#00ffff', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 13);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 14, 'Blue', '#0000ff', 1 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 14);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 15, 'Navy', '#000080', 1 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 15);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 16, 'Purple', '#800080', 1 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 16);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 17, 'Magenta', '#ff00ff', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 17);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 18, 'Pink', '#ffc0cb', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 18);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 19, 'Grey', '#808080', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 19);
INSERT INTO colour (colour_id, colour_name, device_hex, white_lines)
SELECT 20, 'Silver', '#c0c0c0', 0 WHERE NOT EXISTS (SELECT 1 FROM colour WHERE colour_id = 20);
