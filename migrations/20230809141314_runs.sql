CREATE TABLE runs (
    id   TEXT NOT NULL PRIMARY KEY,
    hash TEXT NOT NULL,

    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE TABLE measurements (
    id        TEXT NOT NULL,
    name      TEXT NOT NULL,
    value     REAL NOT NULL,
    stddev    REAL,
    unit      TEXT,
    direction INT,

    PRIMARY KEY (id, name),
    FOREIGN KEY (id) REFERENCES runs (id) ON DELETE CASCADE
) STRICT;
