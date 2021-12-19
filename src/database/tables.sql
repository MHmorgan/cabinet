
PRAGMA foreign_keys = ON;

BEGIN;


--------------------------------------------------------------------------------
-- File

CREATE TABLE IF NOT EXISTS file (
    id       INTEGER PRIMARY KEY, -- entry id
    name     TEXT NOT NULL,
    parent   INTEGER REFERENCES directory ON DELETE CASCADE,
    content  BLOB,
    mode     INTEGER NOT NULL,
    modified DATETIME NOT NULL,
    UNIQUE (name, parent),
    CHECK( typeof(name)='text' AND length(name)>0 )
);

CREATE INDEX IF NOT EXISTS path_idx ON file(name, parent);

CREATE VIEW IF NOT EXISTS file_path(id, path) AS
WITH RECURSIVE
    -- Recursively build the file paths for all files from their parents
    paths(id, name, parent) AS (
        SELECT id, name, parent FROM file
        UNION
        SELECT paths.id, directory.name || '/' || paths.name, directory.parent
          FROM paths, directory
         WHERE directory.id=paths.parent
    )
SELECT id, name FROM paths WHERE parent IS NULL;


--------------------------------------------------------------------------------
-- Directory

CREATE TABLE IF NOT EXISTS directory (
    id      INTEGER PRIMARY KEY,
    name    TEXT NOT NULL,
    parent  INTEGER REFERENCES directory ON DELETE CASCADE
);


--------------------------------------------------------------------------------
-- Boilerplate

CREATE TABLE IF NOT EXISTS boilerplate (
    id       INTEGER PRIMARY KEY,
    name     TEXT NOT NULL,
    modified TEXT NOT NULL,
    script   CLOB
);

CREATE INDEX IF NOT EXISTS bp_name_idx ON boilerplate(name);

CREATE TABLE IF NOT EXISTS bp_file_map (
    id          INTEGER PRIMARY KEY,
    boilerplate INTEGER NOT NULL REFERENCES boilerplate ON DELETE CASCADE,
    file        INTEGER NOT NULL REFERENCES file ON DELETE CASCADE,
    location    TEXT NOT NULL -- Client-side file location
);

CREATE INDEX IF NOT EXISTS bpf_file_idx ON bp_file_map(file);

CREATE VIEW IF NOT EXISTS bp_files(bp_id, path, location) AS
WITH
    bp_files AS (SELECT DISTINCT file FROM bp_file_map),
    files(id, path) AS (SELECT id, path FROM file_path WHERE id IN (SELECT * FROM bp_files))
SELECT boilerplate, path, location
FROM bp_file_map JOIN files
ON bp_file_map.file = files.id;

COMMIT;
