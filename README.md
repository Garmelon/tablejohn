# tablejohn

Run benchmarks against commits in a git repo and present their results.

## Building from source

The following tools are required:
- `cargo` and `rustc` (best installed via [rustup](https://rustup.rs/))
- `tsc`, the [typescript](https://www.typescriptlang.org/) compiler

Once you have installed these tools, run the following command to install or
update tablejohn to `~/.cargo/bin/tablejohn`:
```
cargo install --force --git https://github.com/Garmelon/tablejohn
```

Alternatively, clone the repo and run `cargo build --release`. The compiled
binary will be located at `target/release/tablejohn`.

The binary produced by either of these steps contains everything needed to run
tablejohn. Not additional files are required.

## Developing

I recommend using VSCode and rust-analyzer in combination with the tools
mentioned in the previous section. However, some parts of the code base require
additional tools and setup.

### Changing SQL queries with sqlx

If you want to change any of the SQL queries, you will need to install `sqlx`,
the [CLI of the sqlx library][sqlx]. The sqlx library can connect to a dev
database at compile-time to verify SQL queries defined via the `query*` macro
family. This is useful during development as it gives immediate feedback on
mistakes in your queries. However, it requires a bit of setup. During normal
compilation with `cargo build`, the cached query analyses in `.sqlite/` are used
instead of the dev database. This way, the dev database and `sqlx` tool is not
required when you're just building the project.

First, run `./meta/setup`. This creates or updates the dev database at
`target/dev.db`. You will need to rerun this command whenever you change or add
a migration.

Then, if you don't use VSCode, configure your `rust-analyzer` to run with the
with the environment variable `SQLX_OFFLINE=false` using the
[`rust-analyzer.server.extraEnv` option][ra-opt]. This signals to sqlx that it
should use the dev database instead of `.sqlx/`, but only in your IDE.

**Important:** Before committing any changed SQL query, you **must** run
`./meta/update_sqlx_queries`. This will recreate your dev database (just like
`./meta/setup`) and then update the files in `.sqlx/`.

[sqlx]: https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md
[ra-opt]: https://rust-analyzer.github.io/manual.html#rust-analyzer.check.extraEnv

## Design notes

- A tablejohn instance tracks exactly one git repository.
- A tablejohn instance has exactly one sqlite db.
- Tablejohn does not clone or update repos, only inspect them.
- Tablejohn can inspect bare and non-bare repos.
- Server settings should go in a config file.
- Repo settings should go in the db and be managed via the web UI.
- Locally, tablejohn should just work™ without custom config.
- Run via `tablejohn <db> [<repo>]`

- The db contains...
    - Known commits
    - Runs and their measurements
    - Queue of tasks (not-yet-run runs)
    - Tracked branches (new commits are added to the queue automatically)
    - Github commands

- Runners...
    - Ping tablejohn instance regularly with their info?
        - WS connection complex, but quicker to update
    - Reserve tasks (for a limited amount of time 10 min?)
    - Steal tasks based on time already spent on task
    - Update server on tasks
        - Maybe this is the same as reserving a task?
        - Include last few lines of output
    - Turn tasks into runs
        - Handle errors sensibly
        - Include full output (stdout and stderr), especially if task fails
