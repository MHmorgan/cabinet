
.read src/database/tables.sql
.mode box

BEGIN;

INSERT INTO directory(name) VALUES ('mydir');

WITH
    q(id) AS (SELECT id FROM directory)
INSERT INTO directory(name, parent) VALUES
    ('foodir', (SELECT id FROM q));

WITH
    q1(id) AS (SELECT id FROM directory WHERE name IS 'mydir'),
    q2(id) AS (SELECT id FROM directory WHERE name IS 'foodir')
INSERT INTO file(name, parent, mode, modified) VALUES
    ('myfile', NULL, 101, 'Wed, 21 Oct 2015 01:11:00 GMT'),
    ('foo.txt', (SELECT id FROM q1), 202, 'Wed, 21 Oct 2015 02:22:00 GMT'),
    ('bar.txt', (SELECT id FROM q2), 303, 'Fri, 23 Oct 2015 03:33:00 GMT');

INSERT INTO boilerplate(name, modified, script) VALUES
    ('mybp', 'Fri, 23 Oct 2015 03:33:00 GMT', 'apt install python');

-- Add all files to the only boilerplate
WITH
    q(file, location) AS (SELECT id, name FROM file)
INSERT INTO bp_file_map(boilerplate, file, location)
SELECT id, file, location FROM boilerplate CROSS JOIN q;

COMMIT;
