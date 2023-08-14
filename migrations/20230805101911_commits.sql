CREATE TABLE commits (
    hash           TEXT NOT NULL PRIMARY KEY,
    author         TEXT NOT NULL,
    author_date    TEXT NOT NULL,
    committer      TEXT NOT NULL,
    committer_date TEXT NOT NULL,
    message        TEXT NOT NULL,
    reachable      INT  NOT NULL DEFAULT 0,
    new            INT  NOT NULL DEFAULT 1
) STRICT;

CREATE TABLE commit_links (
    child  TEXT NOT NULL,
    parent TEXT NOT NULL,

    PRIMARY KEY (parent, child),
    FOREIGN KEY (parent) REFERENCES commits (hash) ON DELETE CASCADE,
    FOREIGN KEY (child)  REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE TABLE refs (
    name    TEXT NOT NULL PRIMARY KEY,
    hash    TEXT NOT NULL,
    tracked INT NOT NULL DEFAULT 0,

    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_commits_committer_date_hash
ON commits (committer_date, hash);

CREATE INDEX idx_commit_links_parent_child
ON commit_links (parent, child);
