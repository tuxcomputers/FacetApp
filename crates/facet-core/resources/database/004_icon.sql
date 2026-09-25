-- icon
-- Reference table of activity icons available to assign to a face.

CREATE TABLE IF NOT EXISTS icon (
  icon_id     INTEGER CONSTRAINT PK_icon PRIMARY KEY
  , icon_name TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS UN1_icon ON icon(icon_name);

INSERT INTO icon (icon_id, icon_name)
SELECT 0, 'None' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 0);
INSERT INTO icon (icon_id, icon_name)
SELECT 1, 'ic_admin' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 1);
INSERT INTO icon (icon_id, icon_name)
SELECT 2, 'ic_agile' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 2);
INSERT INTO icon (icon_id, icon_name)
SELECT 3, 'ic_brainstorming' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 3);
INSERT INTO icon (icon_id, icon_name)
SELECT 4, 'ic_break' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 4);
INSERT INTO icon (icon_id, icon_name)
SELECT 5, 'ic_bugs' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 5);
INSERT INTO icon (icon_id, icon_name)
SELECT 6, 'ic_calls' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 6);
INSERT INTO icon (icon_id, icon_name)
SELECT 7, 'ic_camera' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 7);
INSERT INTO icon (icon_id, icon_name)
SELECT 8, 'ic_chat' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 8);
INSERT INTO icon (icon_id, icon_name)
SELECT 9, 'ic_client' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 9);
INSERT INTO icon (icon_id, icon_name)
SELECT 10, 'ic_code' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 10);
INSERT INTO icon (icon_id, icon_name)
SELECT 11, 'ic_consult' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 11);
INSERT INTO icon (icon_id, icon_name)
SELECT 12, 'ic_design' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 12);
INSERT INTO icon (icon_id, icon_name)
SELECT 13, 'ic_document' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 13);
INSERT INTO icon (icon_id, icon_name)
SELECT 14, 'ic_edit' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 14);
INSERT INTO icon (icon_id, icon_name)
SELECT 15, 'ic_emails' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 15);
INSERT INTO icon (icon_id, icon_name)
SELECT 16, 'ic_facebook' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 16);
INSERT INTO icon (icon_id, icon_name)
SELECT 17, 'ic_fitness' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 17);
INSERT INTO icon (icon_id, icon_name)
SELECT 18, 'ic_games' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 18);
INSERT INTO icon (icon_id, icon_name)
SELECT 19, 'ic_instagram' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 19);
INSERT INTO icon (icon_id, icon_name)
SELECT 20, 'ic_internet' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 20);
INSERT INTO icon (icon_id, icon_name)
SELECT 21, 'ic_logistics' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 21);
INSERT INTO icon (icon_id, icon_name)
SELECT 22, 'ic_marketing' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 22);
INSERT INTO icon (icon_id, icon_name)
SELECT 23, 'ic_media' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 23);
INSERT INTO icon (icon_id, icon_name)
SELECT 24, 'ic_meeting' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 24);
INSERT INTO icon (icon_id, icon_name)
SELECT 25, 'ic_money' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 25);
INSERT INTO icon (icon_id, icon_name)
SELECT 26, 'ic_music' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 26);
INSERT INTO icon (icon_id, icon_name)
SELECT 27, 'ic_office' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 27);
INSERT INTO icon (icon_id, icon_name)
SELECT 28, 'ic_presentation' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 28);
INSERT INTO icon (icon_id, icon_name)
SELECT 29, 'ic_project' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 29);
INSERT INTO icon (icon_id, icon_name)
SELECT 30, 'ic_quotation' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 30);
INSERT INTO icon (icon_id, icon_name)
SELECT 31, 'ic_reading' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 31);
INSERT INTO icon (icon_id, icon_name)
SELECT 32, 'ic_report' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 32);
INSERT INTO icon (icon_id, icon_name)
SELECT 33, 'ic_shopping' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 33);
INSERT INTO icon (icon_id, icon_name)
SELECT 34, 'ic_studying' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 34);
INSERT INTO icon (icon_id, icon_name)
SELECT 35, 'ic_support' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 35);
INSERT INTO icon (icon_id, icon_name)
SELECT 36, 'ic_test' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 36);
INSERT INTO icon (icon_id, icon_name)
SELECT 37, 'ic_tv' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 37);
INSERT INTO icon (icon_id, icon_name)
SELECT 38, 'ic_twitter' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 38);
INSERT INTO icon (icon_id, icon_name)
SELECT 39, 'ic_urgent' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 39);
INSERT INTO icon (icon_id, icon_name)
SELECT 40, 'ic_ux' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 40);
INSERT INTO icon (icon_id, icon_name)
SELECT 41, 'ic_write' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 41);
INSERT INTO icon (icon_id, icon_name)
SELECT 42, 'ic_you_tube' WHERE NOT EXISTS (SELECT 1 FROM icon WHERE icon_id = 42);
