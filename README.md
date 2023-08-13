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

### uPlot

For displaying graphs, the [uPlot library][uplot] is used. Because this is the
only dependency on the JS side, I decided not to use a package manager and
instead just add the library files directly.

To update the uPlot files, run `./meta/update_uplot`. This will download the
required files from uPlot's master. The `uPlot.d.ts` file's export statement is
patched to resemble the one in `uPlot.js`. This way I don't have to enable
`esModuleInterop` in my `tsconfig.json`.

[uplot]: https://github.com/leeoniya/uPlot/
