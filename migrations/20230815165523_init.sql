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
    stddev    REAL,
    unit      TEXT,
    direction INT,

    PRIMARY KEY (id, metric),
    FOREIGN KEY (id) REFERENCES runs (id) ON DELETE CASCADE
) STRICT;

CREATE TABLE run_output (
    id     TEXT NOT NULL,
    idx    INT  NOT NULL,
    source INT  NOT NULL,
    text   TEXT NOT NULL,

    PRIMARY KEY (id, idx),
    FOREIGN KEY (id) REFERENCES runs (id) ON DELETE CASCADE
) STRICT;

CREATE TABLE queue (
    hash     TEXT NOT NULL PRIMARY KEY,
    date     TEXT NOT NULL,
    priority INT  NOT NULL DEFAULT 0,

    FOREIGN KEY (hash) REFERENCES commits (hash) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_commit_links_parent_child
ON commit_links (parent, child);

CREATE INDEX idx_queue_priority_date_hash
ON queue (priority DESC, unixepoch(date) DESC, hash ASC);

CREATE INDEX idx_run_measurements_metric_id_value
ON run_measurements (metric, id, value);
