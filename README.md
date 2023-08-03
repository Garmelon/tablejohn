# tablejohn

A tool to run benchmarks for a git repo and display their results.

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
