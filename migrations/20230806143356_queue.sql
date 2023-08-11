CREATE TABLE queue (
    hash     TEXT NOT NULL PRIMARY KEY,
    date     TEXT NOT NULL,
    priority INT  NOT NULL DEFAULT 0,

    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;
