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
    name      TEXT NOT NULL,
    value     REAL NOT NULL,
    stddev    REAL,
    unit      TEXT,
    direction INT,

    PRIMARY KEY (id, name),
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

CREATE INDEX idx_run_measurements_name_id_value
ON run_measurements (name, id, value);
