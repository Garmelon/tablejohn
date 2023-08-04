CREATE TABLE commits (
    hash TEXT NOT NULL PRIMARY KEY,
    new  INT  NOT NULL DEFAULT 1
) STRICT;

CREATE TABLE commit_links (
    parent TEXT NOT NULL,
    child  TEXT NOT NULL,
    PRIMARY KEY (parent, child),
    FOREIGN KEY (parent) REFERENCES commits (hash) ON DELETE CASCADE,
    FOREIGN KEY (child)  REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE TABLE branches (
    name TEXT NOT NULL PRIMARY KEY,
    hash TEXT NOT NULL,
    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;
