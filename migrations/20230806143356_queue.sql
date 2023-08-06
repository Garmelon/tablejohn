CREATE TABLE queue (
    id       TEXT NOT NULL PRIMARY KEY,
    hash     TEXT NOT NULL,
    date     TEXT NOT NULL,
    priority INT  NOT NULL DEFAULT 0,
    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;
