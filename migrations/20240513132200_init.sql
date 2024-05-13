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

CREATE INDEX idx_commits_hash_reachable
ON commits (hash, reachable);

CREATE TABLE commit_edges (
    child  TEXT NOT NULL,
    parent TEXT NOT NULL,

    PRIMARY KEY (parent, child),
    FOREIGN KEY (parent) REFERENCES commits (hash) ON DELETE CASCADE,
    FOREIGN KEY (child) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_commit_edges_parent_child
ON commit_edges (parent, child);

CREATE INDEX idx_commit_edges_child_parent
ON commit_edges (child, parent);

CREATE TABLE refs (
    name    TEXT NOT NULL PRIMARY KEY,
    hash    TEXT NOT NULL,
    tracked INT  NOT NULL DEFAULT 0,

    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE TABLE metrics (
    name      TEXT NOT NULL PRIMARY KEY,
    unit      TEXT,
    direction INT  NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE runs (
    id           TEXT NOT NULL PRIMARY KEY,
    hash         TEXT NOT NULL,
    bench_method TEXT NOT NULL,
    worker_name  TEXT NOT NULL,
    worker_info  TEXT,
    start        TEXT NOT NULL,
    end          TEXT NOT NULL,
    exit_code    INT  NOT NULL,

    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE TABLE run_measurements (
    id        TEXT NOT NULL,
    metric    TEXT NOT NULL,
    value     REAL NOT NULL,
    unit      TEXT,

    PRIMARY KEY (id, metric),
    FOREIGN KEY (id) REFERENCES runs (id) ON DELETE CASCADE,
    FOREIGN KEY (metric) REFERENCES metrics (name) ON UPDATE CASCADE ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_run_measurements_metric_id_value
ON run_measurements (metric, id, value);

CREATE TABLE run_output (
    id     TEXT NOT NULL,
    line   INT  NOT NULL,
    source INT  NOT NULL,
    text   TEXT NOT NULL,

    PRIMARY KEY (id, line),
    FOREIGN KEY (id) REFERENCES runs (id) ON DELETE CASCADE
) STRICT;

CREATE TABLE queue (
    hash     TEXT NOT NULL PRIMARY KEY,
    date     TEXT NOT NULL,
    priority INT  NOT NULL DEFAULT 0,

    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_queue_priority_date_hash
ON queue (priority DESC, unixepoch(date) DESC, hash ASC);
