CREATE TABLE commits (
    hash    TEXT NOT NULL PRIMARY KEY,
    new     INT  NOT NULL DEFAULT 1,
    tracked INT  NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE commit_links (
    parent TEXT NOT NULL,
    child  TEXT NOT NULL,
    PRIMARY KEY (parent, child),
    FOREIGN KEY (parent) REFERENCES commits (hash) ON DELETE CASCADE,
    FOREIGN KEY (child)  REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE TABLE tracked_refs (
    name TEXT NOT NULL PRIMARY KEY,
    hash TEXT NOT NULL,
    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_commit_links_child_parent
ON commit_links(child, parent);

CREATE INDEX idx_commit_links_parent_child
ON commit_links(parent, child);
