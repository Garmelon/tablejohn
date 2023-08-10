# Design notes

Written down so I don't forget them, and because writing them down helps me
think them through.

## General ideas

- A tablejohn instance has exactly one sqlite db.
- A tablejohn instance optionally has a repo to update the db from.
- Tablejohn can inspect bare and non-bare repos.
- Locally, tablejohn should just work™ without custom config.
- However, some cli args might need to be specified for full functionality.
- The db contains...
    - Commits and their relationships
    - Branches and whether they're tracked
    - Runs and their measurements
    - Queue of commits
- The in-memory state also contains...
    - Connected runners and their state
    - From this follows the list of in-progress runs
- Runners...
    - Should be robust
        - Noone wants to lose a run a few hours in, for any reason
        - Explicitly design for loss of connection, server restarts
        - Also design for bench script failures
    - Can connect to more than one tablejohn instance
        - Use task runtime-based approach to fairness
        - Steal tasks based on time already spen on task
    - Use plain old http requests (with BASIC auth) to communicate with server
    - Store no data permanently
- Nice-to-have but not critical
    - Statically checked links
    - Statically checked paths for static files

## Web pages

- GET `/`
    - Tracked and untracked refs
    - Recent significant changes?
    - "What's the state of the repo?"
- GET `/graph/`
    - Interactive graph
    - Change scope interactively
    - Change metrics interactively
- GET `/queue/`
    - List of runners and their state
    - List of unfinished runs
    - "What's the state of the infrastructure?"
- GET `/commit/<hash>/`
    - Show details of a commit
    - Link to parents, chilren, runs in chronological order
    - Resolve refs and branch names to commit hashes -> redirect
- GET `/run/<rid>/`
    - Show details of a run
    - Link to commit, other runs in chronological order
    - Links to compare against previous run, closest tracked ancestors?
    - Resolve refs, branch names and commits to their latest runs -> redirect
- GET `/compare/<rid1>/`
    - Select/search run to compare against?
    - Enter commit hash or run id
    - Resolve refs, branch names and commits to their latest runs -> redirect
- GET `/compare/rid1/<rid2>/`
    - Show changes from rid2 to rid1
    - Resolve refs, branch names and commits to their latest runs -> redirect

## Runner interaction

Runner interaction happens via endpoints located at `/api/runner/`. All of these
are behind BASIC authentication. The username is `runner` and the password must
be the server's runner token. When the runner presents the correct token, the
server trusts the data the runner sends, including the name, current state, and
run ids.

On the server side, runners are identified by the runner's self-reported
identifier. This allows more human-readable and permanent links to runners than
something like session ids.

- POST `/api/runner/status`
    - Main endpoint for runner/server coordination
    - Runner periodically sends current status to server
        - Includes a secret randomly chosen by the runner
        - Subsequent requests must include exactly the same secret
        - Protects against the case where multiple runners share the same name
    - Runner may include request for new work
        - If so, server may respond with a commit hash and bench method
    - Runner may include current work
        - If so, server may respond with request to abort the work
- GET `/api/runner/repo/<hash>/tar`
    - Get tar-ed commit from the server's repo, if any exists
- GET `/api/runner/bench-repo/<hash>/tar`
    - Get tar-ed commit from the server's bench repo, if any exist

## CLI Args

tablejohn can be run in one of two modes: Server mode, and runner mode.

- server
    - Run a web server that serves the contents of a db
    - Optionally, specify repo to update the db from
    - Optionally, launch local runner (only if repo is specified)
    - When local runner is enabled, it ignores the runner section of the config
        - Instead, a runner section is generated from the server config
        - This approach should make `--local-runner` more fool-proof
- runner
    - Run only as runner (when using external machine for runners)
    - Same config file format as server, just uses different parts

## Config file and options

Regardless of the mode, the config file is always loaded the same way and has
the same format. It is split into these chunks:

- web (ignored in runner mode)
    - Everything to do with the web server
    - What address and port to bind on
    - What url the site is being served under
- repo (ignored in runner mode)
    - Everything to do with the repo the server is inspecting
    - Name (derived from repo path if not specified here)
    - How frequently to update the db from the repo
    - A remote URL to update the repo from
    - Whether to clone the repo if it doesn't yet exist
- runner (ignored in server mode)
    - Name (uses system name by default)
    - Custom bench dir path (creates temporary dir by default)
    - List of servers, each of which has...
        - Token to authenticate with
        - Base url to contact
        - Weight to prioritize with (by total run time + overhead?)
