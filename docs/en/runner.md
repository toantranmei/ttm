---
title: Runner
description: Run Factory agents on your machine - repo configuration, capacity, the background service, ttm agent, Claude Code requirements, tokens, and the security model.
order: 9
section: guides
---

The **runner** is `ttm runner up`: a process on your dev machine that picks up jobs assigned by the Factory server, runs
a coding agent (**Claude Code**) with your Claude account in a separate git worktree for each task, streams activity to
the Execution Map in real time, then pushes the branch and opens a pull request. The server decides which work to hand
out, which model to use, and what the prompt is; the runner only executes.

## Requirements

| Need | Why |
| --- | --- |
| `ttm` and `ttm-mcp` in the same directory (or `ttm-mcp` on `PATH`) | The agent uses the `factory-*` and `notee-*` MCP tools |
| Signed in with the `factory:run` scope (the `ttm login` default) | Register the runner, receive jobs |
| `git` | Worktrees, commits, pushes |
| [Claude Code](https://docs.anthropic.com/claude-code) (`claude` on `PATH`), signed in to a Claude plan or with `ANTHROPIC_API_KEY` set | The `claude-code` harness |
| `gh` (GitHub CLI) after `gh auth login` (optional) | Open a pull request after pushing the branch |
| A clone of the project repo with an `origin` remote | The runner only works in clones you have declared |

`ttm doctor` checks `git`, `claude`, and `ttm-mcp`.

## Get started

```sh
ttm runner config --repo AUR=~/code/aurora --max-parallel 2
ttm runner up
```

```text
Runner 0190f500-… up: 2 agent(s) at once. Ctrl-C to stop.
```

Assign a task to an agent on the web (or run `ttm agent run AUR-12`), and the runner picks up the job within a few
seconds. Follow it on the Execution Map or with `ttm agent logs <run> -f`.

## Configuration: `ttm runner config`

```sh
ttm runner config --name "macbook-an" --max-parallel 2 --keep-days 7 \
  --worktree-root ~/orca/workspaces --repo AUR=~/code/aurora --repo WEB=~/code/web
ttm runner config --accept-team-jobs true --auto-merge false
```

```text
Saved /Users/an/.config/ttm/runner.toml
```

| Flag | Default | What it does |
| --- | --- | --- |
| `--name <NAME>` | the machine name (`HOSTNAME`/`COMPUTERNAME`/`hostname`) | Name shown on the web; registering again with the same name keeps the runner id |
| `--max-parallel <N>` | computed from the machine | Number of agents running at once (≥ 1) |
| `--accept-team-jobs true\|false` | `false` | Also take jobs for teammates' tasks, not just yours |
| `--worktree-root <DIR>` | `worktree_root` from `config.toml`, then `~/.ttm/worktrees` | Where worktrees are created |
| `--keep-days <N>` | `3` | Days to keep the worktrees of finished jobs |
| `--repo KEY=PATH` | | Local clone of project `KEY` (repeatable; the key is uppercased, the path is normalized, `~/` is expanded) |
| `--auto-merge true\|false` | `false` | Automatically merge the head of the merge queue and push |

Only the flags you pass change; other values stay as they are. The `runner.toml` file sits next to `config.toml`:

```toml
name = "macbook-an"
max_parallel = 2
accept_team_jobs = false
worktree_root = "~/orca/workspaces"
keep_days = 7
runner_id = "0190f500-…"     # written by the first registration
auto_merge = false

[repos]
AUR = "/Users/an/code/aurora"
WEB = "/Users/an/code/web"

[mcp.local.github-local]                         # local MCP (0.2.0), see below
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "env:GITHUB_TOKEN" }
allow = ["dev", "reviewer"]
team_jobs = false
```

Run `ttm runner up` again (or restart the service) to apply the new configuration. The `[mcp.local.*]` tables are read
again for every job, no restart needed.

### Why you declare repos

The runner **never clones a URL the server sends**. Each job names its repo; the runner only runs it when:

1. the job's project has a clone in `[repos]` (by key, case-insensitive, or by id);
2. that clone's `origin` matches the job's repo (compared after normalizing: dropping the scheme, user, port, `.git`,
   and trailing `/`; for example `git@github.com:a/b.git` = `https://github.com/a/b`); for a `local` project, the job's
   path must match the declared path;
3. the job's branch name is valid under git's rules;
4. the worktree stays under `worktree_root`: the job's `worktree_hint` may only be a relative path; an absolute path
   (`/…`, `C:\…`), one with `..`, `\` or `:`, or one that goes through a symlink leading out of `worktree_root` is
   refused.

Otherwise the job is refused and reported to the server (`repo_not_configured`, `invalid_branch` or `invalid_worktree`),
and the terminal prints:

```text
ttm runner: refused AUR-12: no local clone is configured for project AUR; add it with `ttm runner config --repo AUR=<path to your clone>`
```

This way a compromised account or a forged job can't make your machine download and run code from somewhere unknown.

### Capacity

The number of agents running at once is `--max-parallel` if set; otherwise `clamp(CPU cores / 4, 1, 3)`, lowered so
that each agent has at least **4 GB of free RAM**. The server may set a lower limit based on your plan (Free 1, Pro 3,
Team 6 per runner). When the agent reports that your Claude usage limit is reached, the runner **stops taking jobs**
until the reset time (that run ends with status `paused` and `paused_until`), and the server requeues the work instead
of retrying continuously.

## Local MCP: `ttm runner config mcp`

A local MCP is an MCP server running **on your machine** (for example the GitHub server with your own token). By default
**no agent** may use it: you allow agents one by one (by handle).

```sh
ttm runner config mcp add github-local --command npx --arg -y --arg @modelcontextprotocol/server-github \
  --env GITHUB_TOKEN=env:GITHUB_TOKEN
ttm runner config mcp allow dev --mcp github-local     # without --mcp: every local MCP
ttm runner config mcp deny dev --mcp github-local
ttm runner config mcp team-jobs github-local on
ttm runner config mcp remove github-local
ttm runner config mcp list
```

| Key | Meaning |
| --- | --- |
| name (`[mcp.local.<name>]`) | `^[a-z0-9][a-z0-9-]{0,31}$`, never `ttm` (the runner's own MCP server) |
| `command`, `args` | How to start the server |
| `env` | `K = "env:VAR"`: read the runner's variable `VAR` **at run time** (the real value is never written to the file); `K = "value"` only for non-secret values. `--env` refuses literal values that look like tokens |
| `allow` | Agent handles that may use it |
| `team_jobs` | `true`: jobs delegated by someone else may use it too (default `false`) |

A job gets a local MCP when its name is in `job.mcp.local`, the job's agent is in `allow`, and the job is not a teammate's
job or `team_jobs = true`. A missing `env:VAR` drops that MCP (with a warning event); a name equal to `ttm` or to a remote
MCP is dropped too. As `ttm runner config mcp list` says: **only names, allowed agents and `team_jobs` are announced to
the server**; commands, arguments and environment never leave the machine. These commands edit `runner.toml` with
`toml_edit`, so your comments are kept.

### Claude token for teammates' jobs: `ttm runner config claude-token`

```sh
claude setup-token | ttm runner config claude-token     # reads the token on stdin, stores it in the keyring
ttm runner config claude-token --remove
```

See [Teammates' jobs are isolated](#teammates-jobs-are-isolated).

## How a job runs

1. **Check** the repo and branch (section above).
2. **Worktree**: `git fetch origin <default branch>`, then
   `git worktree add -B <branch> <path> origin/<default branch>`. The path is
   `<worktree_root>/<worktree_hint>` (the server suggests `<lowercase key>/<branch without ttm/>`, for example
   `aur/aur-12-dang-nhap-google`); a job without a hint gets `<worktree_root>/<KEY>/<branch>` (unusual characters become
   `-`).
3. **Rules**: runs `ttm rules apply` in the worktree (the `AGENTS.md`/`CLAUDE.md` block, Claude Code hooks).
4. **MCP**: writes `<cache>/mcp-<run>.json` (mode 0600) pointing to
   `ttm-mcp --toolsets <the job's toolsets, default factory,notee>` with `TTM_TOKEN` set to the job token, `TTM_RUN_ID`,
   `TTM_TASK`, `TTM_PROJECT`.
5. **Agent**: `claude -p <prompt> --output-format stream-json --verbose --mcp-config <file> [--model <m>]
   [--max-turns <n>] [--allowedTools <list>]` in the worktree; the model, turn count, and allowed tools come from the
   job. The time limit is set per job, **120 minutes** by default.
6. **Events**: each stream-json line becomes an event, sent up in batches (every second or every 25 events):

   | Claude Code | Event |
   | --- | --- |
   | startup | `log` (`agent started <model>`) |
   | agent text | `thought` |
   | tool call | `action` (`TodoWrite` → `plan`) |
   | final result | `response`, with input/output token counts |
   | test command (`cargo test`, `pnpm test`, `pytest`, `go test`, `vitest`, `playwright test`…) | `log` `tests passed`/`tests failed`; the result is attached to the run as `test_result` |

7. **Finish**: on success, commits whatever the agent left (`chore(<key>): changes from agent run <run>`), pushes the
   branch, opens a PR with `gh pr create` if `gh` is available (or keeps the branch's open PR, which the push
   updated), then reports the status (`succeeded`, `failed`, `cancelled`, `paused`), commit, PR, and tokens used. The
   PR title is `<KEY>: <task title>` (at most 72 characters); the agent's summary is the PR body. The temporary MCP
   file is deleted.

Since 0.2.0, between the rules and the agent the runner also installs the job's skills, prepares the run's own git hooks
and adds the remote and local MCP servers to the MCP file; Claude Code runs with `--strict-mcp-config` (the agent sees only
those servers) and `--settings <cache>/settings-<run>.json` (sections below).

Worktrees of finished jobs are kept for `keep_days` days so you can review them, then removed the next time
`ttm runner up` starts.

Each run prints one line when it starts and one when it ends (stderr):

```text
● AUR-12 running · sonnet · worktree /Users/an/orca/workspaces/aur/aur-12-dang-nhap-google
✓ AUR-12 succeeded → https://github.com/an/aurora/pull/7
✕ AUR-13 refused: repo_not_configured add it with `ttm runner config --repo AUR=<path to your clone>`
```

### Git identity and the `commit-msg` hook

- Jobs with `git` (agent v2) commit with the **agent's** name and email (`GIT_AUTHOR_*`, `GIT_COMMITTER_*`), for example
  `DEV · Code <dev+aurora@agents.toantranmei.com>`. Jobs without `git` (a person's task) keep the machine's git identity.
- **Every job**, in every `commit_identity` mode (`agent`, `delegator`, `delegator_with_trailer`), runs git with
  `core.hooksPath` pointing at the run's own hooks directory `<cache>/hooks/<run>/` (through
  `GIT_CONFIG_COUNT/KEY/VALUE`; no configuration file is changed). Its `commit-msg` hook:
  1. removes every line (case-insensitive) matching `^co-authored-by:.*(claude|anthropic)`, `generated (with|by) .*claude`,
     `🤖 generated with`, `noreply@anthropic.com`;
  2. adds the job's trailers (only `delegator_with_trailer`: `Co-authored-by: <agent name> <agent email>`, once);
  3. runs the repository's own `commit-msg` (from `core.hooksPath` or `.git/hooks`). Other original hooks are chained by
     shims.
- Claude Code always runs with `--settings` holding `{"includeCoAuthoredBy": false, "attribution": {"commit": "", "pr": ""}}`.
- Claude Code also gets `--append-system-prompt` with a runner note: the runner pushes the branch and opens the PR, so
  the agent runs no `git push` and no `gh` command that writes to a PR or issue (read-only `gh` is fine), and its summary says nothing about the PR; a review gives its verdict with the
  factory tools.
- The runner's own commit of leftovers and its push use the same environment. The hooks directory is removed after the run.

Rule: no commit made by the runner ever carries a Claude/Anthropic line.

### Skills

A job can carry **reviewed** skills (`job.skills`: slug, version, a signed download URL valid 10 minutes, SHA-256, size).
The runner:

1. reuses `<cache>/skills/<sha256>.zip` when present, else downloads it (60 seconds, stopping beyond the declared size or
   5 MB);
2. computes the SHA-256; a mismatch **refuses the job** with `skill_hash_mismatch`;
3. unpacks it safely (no `..`, no symlinks, ≤ 200 files, ≤ 5 MB) into `<worktree>/.claude/skills/ttm-<slug>/`; when git
   already tracks that directory the skill is skipped (warning);
4. adds `/.claude/skills/ttm-*/` once to the repository's `info/exclude`, so skills are never committed;
5. removes the installed directories after the run, whether it succeeded, failed, was cancelled or timed out.

### MCP through the gateway and local MCP

- **Remote MCP** servers of the workspace (`ttm mcp-server …`): the job gets `{name, url}` pointing at the server's
  gateway (`/api/v1/mcp/gw/<server>`) and calls it with the job token. MCP server secrets (OAuth tokens, secret headers)
  stay on the server and never reach your machine.
- **Local MCP**: see [Local MCP](#local-mcp-ttm-runner-config-mcp).

### Teammates' jobs are isolated

A teammate's job (`team_job`: the person who delegated it is not the runner's owner) must not see your personal Claude
setup:

- it always runs with `--setting-sources project,local` (your `~/.claude/settings.json` is left out);
- once a token is saved with `ttm runner config claude-token`, it also runs with an empty `CLAUDE_CONFIG_DIR` per run and
  `CLAUDE_CODE_OAUTH_TOKEN` set to that token, so it sees none of your hooks, plugins or personal skills
  (`~/.claude/skills`);
- the runner announces `team_isolation` only when the token is saved. Without a token it **refuses** every teammate's
  job (`unsupported_feature`, reported to the server; it never runs one with this machine's Claude setup), and
  `ttm runner status` tells you what to run.

Spike result (Claude Code 2.1.318, from `claude --help` only): `--setting-sources` exists, but the help does not say
whether plugins and personal skills are left out, so the flag alone is **not enough**; hence the empty `CLAUDE_CONFIG_DIR`.
To be confirmed with a real job during the M4 simulation.

### Features (`features`)

On registration the runner announces what it can do: `git_identity`, `skills`, `mcp_gateway`, `mcp_local`, plus
`team_isolation` when a Claude token for teammates' jobs is saved; and its local MCP list (`name`, `allow`, `team_jobs`).
The heartbeat sends the local MCP list again when `runner.toml` changes. A job whose `requires` lists a feature this
runner lacks is refused with `unsupported_feature`.

### Connection to the server

- The runner only opens **outbound** connections (works behind NAT, no open ports needed): a heartbeat every 20 seconds
  and a job long-poll (`?wait=25`, the server holds it up to 25 seconds) while it has free slots. A pending long-poll is
  not cancelled when a heartbeat is due, so a job the server already handed out is never lost.
- The heartbeat carries the list of running runs; the server answers with the runs to stop (the **kill switch** on the
  web or `ttm agent stop`) or an order to shut the runner down (“turned off from the web”).
- The runner token lives for 12 hours and is renewed automatically 30 minutes before it expires (by registering again;
  the old token is revoked). After more than 3 consecutive `401` rejections, the runner stops.

### Auto-merge (`auto_merge`)

When enabled, every minute the runner takes the head of the merge queue for the projects in `[repos]`, merges
`origin/<run branch>` into the default branch in a temporary worktree `ttm/merge-<id>` (`git merge --no-ff`, commit
`merge: <KEY> (<branch>)`), pushes `HEAD:<default branch>`, and reports `merged` or `conflict` (the server creates a
“resolve” task for conflicts). Reporting the result uses your user token (needs `factory:write`).

## Monitor and stop

```sh
ttm runner status
```

```text
● macbook-an running
     runner  0190f500-…
        pid  4242
   capacity  2
  team jobs  no
  harnesses  claude-code
  worktrees  /Users/an/orca/workspaces
   features  git_identity, skills, mcp_gateway, mcp_local

PROJECT  CLONE
───────────────────────────────
AUR      /Users/an/code/aurora
WEB      /Users/an/code/web
```

A local MCP table (name, command, allowed agents, team jobs) is added when there are any, and a warning when teammates'
jobs are not isolated yet.

```sh
ttm runner down     # sends SIGTERM (Windows: taskkill) to the process in runner.pid
```

`Ctrl-C` or `ttm runner down` stops the runner; running agents are cancelled. If the runner runs under a service, the
service restarts it; `ttm runner uninstall-service` turns it off for good.

## Background service: `ttm runner install-service`

```sh
ttm runner install-service
ttm --profile work runner install-service     # the service uses the work profile
```

| OS | What gets installed | Logs |
| --- | --- | --- |
| macOS | LaunchAgent `~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist` (`RunAtLoad`, `KeepAlive`), loaded with `launchctl load -w` | `<cache>/runner.log` (`~/.cache/ttm/runner.log`) |
| Linux | `~/.config/systemd/user/ttm-runner.service` (`Restart=on-failure`), `systemctl --user enable --now` | `journalctl --user -u ttm-runner` |
| Windows | Task Scheduler `ttm-runner`, runs at sign-in (`schtasks /Create /SC ONLOGON`) | (no log file) |

The service runs `<path to ttm> [--profile <name>] runner up`; `--profile` is only written when you pass it explicitly.

The service carries a `PATH`: launchd and systemd start services with a minimal `PATH` (`/usr/bin:/bin:…`) that lacks
`claude`, `gh` and sometimes `git`. So the plist (`EnvironmentVariables`) and the unit (`Environment="PATH=…"`) record
the `PATH` of the shell that runs `install-service` (empty or relative entries such as `.` dropped), plus whichever of
`~/.local/bin`, `~/.cargo/bin`, `/opt/homebrew/bin`, `/opt/homebrew/sbin`, `/usr/local/bin`, `/usr/bin`, `/bin`,
`/usr/sbin`, `/sbin` are missing. The command prints the recorded `PATH`:

```text
Runner service installed: /Users/an/Library/LaunchAgents/com.toantranmei.ttm-runner.plist
PATH of the service: /Users/an/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/Users/an/.cargo/bin:/opt/homebrew/sbin:/usr/sbin:/sbin
```

If you later install tools somewhere else, run `ttm runner install-service` again from a shell with the new `PATH` (it
overwrites and reloads the service). On Windows the task runs with your account's environment (the `PATH` set in
Windows), so nothing extra is needed. On Linux you also need `loginctl enable-linger $USER` if you want the runner to run
while you aren't signed in.

### Uninstall the service: `ttm runner uninstall-service`

```sh
ttm runner uninstall-service
```

Stops the service, then removes it: macOS `launchctl unload -w` and deletes the plist; Linux `systemctl --user disable
--now`, deletes the unit, `daemon-reload`; Windows `schtasks /End` then `schtasks /Delete /F /TN ttm-runner`. Output:
`Runner service stopped and removed: <file>`, or `No runner service installed.` when there was none.

## Run a task now: `ttm agent`

```sh
ttm agent run AUR-12
```

```text
Run 0190f610-… for AUR-12 on branch aur-12-dang-nhap-google
[log] worktree /Users/an/.ttm/worktrees/aur/aur-12-dang-nhap-google on aur-12-dang-nhap-google from main
[log] agent started claude-sonnet-4-5
[thought] Mình sẽ đọc context của task trước.
[action] mcp__ttm__factory-get-context
[action] Bash: cargo test -p auth
[log] tests passed: cargo test -p auth
[response] Đã thêm đăng nhập Google, test đều qua.
Run 0190f610-…: succeeded
worktree: /Users/an/.ttm/worktrees/aur/aur-12-dang-nhap-google
pull request: https://github.com/an/aurora/pull/57
```

`agent run` registers this machine if needed, asks the server for a job for the task (`factory:run`), runs it right away
in the foreground with the `runner.toml` configuration (you still have to declare `--repo`), prints the events, and
**exits 0 only if the run `succeeded`**. `Ctrl-C` cancels the run. If the task already has another run:
`409 conflict` (`rule: run-active`).

```sh
ttm agent logs 0190f610-…            # events so far
ttm agent logs 0190f610-… -f         # follow, polling every 2 seconds (Ctrl-C to quit)
ttm agent logs 0190f610-… --json     # one JSON line per event
ttm agent stop 0190f610-…            # kill switch
```

```text
2026-11-04T05:31:02Z [log] agent started claude-sonnet-4-5
2026-11-04T05:31:09Z [action] Bash: cargo test -p auth
```

`agent stop` tells the server to stop the run; the runner running it gets the order on its next heartbeat
(≤ 20 seconds), kills the agent process, and reports `cancelled`.

## Tokens the runner uses

| Token | Used for | Stored in |
| --- | --- | --- |
| User token (`ttm login`) | Registering the runner, `ttm agent run`, reporting merge results (`auto_merge`) | Keyring `<profile>@<host>` (or `TTM_TOKEN`) |
| Runner token (`ttm_job_`, 12 hours) | Heartbeats, receiving jobs, sending events, finishing runs | Keyring `<profile>#runner@<host>`, never from `TTM_TOKEN` |
| Job token (`ttm_job_`) | The agent's `ttm-mcp` within that run only, the MCP gateway | Temporary MCP file (0600), deleted when the run ends |
| Claude token for teammates' jobs (`claude setup-token`) | `CLAUDE_CODE_OAUTH_TOKEN` of teammates' jobs | Keyring `<profile>#claude-team` |

To revoke a runner: the **Runner** or **Token** tab on `/me/ket-noi`.

## Security model

- **Outbound connections only**; no ports are opened on your machine.
- **Only repos you declare**, with `origin` checked for every job; no unknown URLs are cloned; branch names are checked
  before use.
- **One worktree and one branch per task**; your own worktree isn't touched. The `pre-bash` hook blocks direct pushes to
  a protected branch, `--no-verify`, and commits that break the convention. Worktrees always stay under `worktree_root`.
- **The job token** only works within its run and expires with the job; it lives in a 0600 file and is deleted after the
  run.
- **The agent runs with your OS user's permissions**, limited by the allowed-tools list the job sends. This isn't a
  sandbox: only run the runner for projects and workspaces you trust, and review PRs before merging.
- **No Claude/Anthropic lines** in commits or PRs (the run's `commit-msg` hook, `--settings`).
- **Only reviewed skills**, checked by SHA-256, never committed, removed after the run.
- **Local MCP is off by default**, only for agents you allow; the server only learns the names. Teammates' jobs don't see
  your personal Claude setup.
- **An agent running a job never gets admin tools** (`factory:admin`): the job's MCP drops the `admin` toolset.
- The final prompt of each step passes through your machine (to run Claude Code); the server never receives your Claude
  key.
