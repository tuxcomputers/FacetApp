-- project
-- A named project. Id and name only for now.

CREATE TABLE IF NOT EXISTS project (
  project_id     INTEGER CONSTRAINT PK_project PRIMARY KEY AUTOINCREMENT
  , project_name TEXT NOT NULL
);

INSERT INTO project (project_id, project_name)
SELECT 0, 'None'
WHERE NOT EXISTS (SELECT 1 FROM project WHERE project_name = 'None');
