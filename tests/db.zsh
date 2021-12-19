#!/usr/bin/env zsh

# NOTE: Must be run from top of repo

source tests/common.zsh

cmd_exists sqlite3 || bail "sqlite3 not found in path"

DB=db.sqlite

SQL="
.open $DB
.read src/database/tables.sql

INSERT INTO directory VALUES ('mydir');

INSERT INTO file VALUES
    (0, 0, 'myfile', NULL, 'Hello world', 4, 0, NULL),
    (1, 0, 'myfile', NULL, 'Hello world, again', 4, 1, NULL),
    (2, 1, 'foo.txt', 'mydir', 'Lorem ipsum', 4, 0, NULL);

INSERT INTO boilerplate VALUES (0, 0, 'myboilerplate', 0, NULL);
INSERT INTO boilerplate_files VALUES
    (0, 0, 0, 'myfile'),
    (1, 0, 1, 'foo.txt');
"

if [[ ! -f $DB ]]; then
    info "Creating and filling test database: $DB"
    echo $SQL | sqlite3
else
    info "Test database already exists: $DB"
fi

sqlite3 $DB
