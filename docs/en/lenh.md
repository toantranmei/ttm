---
title: Command reference
description: Every ttm command and subcommand with syntax, flags, defaults, examples, JSON output, and exit codes.
order: 10
section: reference
---

```text
ttm [OPTIONS] <COMMAND>
```

`ttm --help` and `ttm <command> --help` print the built-in help (in English). This page covers version **0.1.5**.

## Global flags

Available on every command, before or after the subcommand (`ttm whoami --json` = `ttm --json whoami`).

| Flag | Variable | Description |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | Server, for example `http://localhost:3000` |
| `--profile <NAME>` | `TTM_PROFILE` | Profile in `config.toml` |
| `--json` | | Print results as JSON on stdout |
| `-v`, `--verbose` | `TTM_LOG` | More logging to stderr (`-v` info, `-vv` debug) |
| `-h`, `--help` | | Help |
| `-V`, `--version` | | Version (`ttm 0.1.5`; top level only) |

## Exit codes

| Code | When |
| --- | --- |
| `0` | Success |
| `1` | Error (network, authentication, API, file…); `ttm doctor` has a `FAIL` item; `ttm sync --check` finds drift; `ttm sync` hits a conflict; `ttm agent run` ends with a status other than `succeeded` |
| `2` | Invalid command-line usage (unknown flag, missing argument). For `ttm hook …`: the action is blocked by a rule |

Errors always print `error: <message>` to stderr; with `--json`, `{"error": {"code", "message"}}` is also printed to stdout
(see [Configuration](cau-hinh.md) and [Error codes](ma-loi.md)).

## Summary

| Command | Description |
| --- | --- |
| [`login`](#login) | Sign in (browser, `--device`, `--token`) |
| [`logout`](#logout) | Revoke the token and remove it from this machine |
| [`whoami`](#whoami) | The signed-in user |
| [`status`](#status) | Host, profile, token, and workspace in use |
| [`doctor`](#doctor) | Check your environment |
| [`workspace`](#workspace) | `list`, `switch`, `current` |
| [`token`](#token) | `list`, `create`, `revoke` (opens the web page) |
| [`notes`](#notes) | `search`, `get`, `query`, `comments`, `create`, `edit`, `comment`, `rows`, `suggestions`, `resolve` |
| [`project`](#project) | `init`, `link`, `open`, `status` |
| [`sync`](#sync) | Sync the plan |
| [`task`](#task) | `list`, `show`, `claim`, `done`, `approve`, `split` |
| [`rules`](#rules) | `apply` |
| [`hook`](#hook) | `pre-bash`, `stop` (called by the agent) |
| [`runner`](#runner) | `up`, `down`, `status`, `config`, `install-service`, `uninstall-service` |
| [`agent`](#agent) | `run`, `logs`, `stop` |
| [`mcp`](#mcp) | `install` |
| [`self-update`](#self-update) | Update ttm |
| [`completion`](#completion) | Shell completion script |

---

## login

```text
ttm login [--device | --token <PAT>] [--scope <SCOPE>] [--no-browser]
```

Signs in and stores a token for the current (profile, host). By default it opens the browser (loopback `127.0.0.1` +
PKCE, waits 5 minutes). Writes the host to the profile when `--host`/`TTM_HOST` is given or the profile has no host yet.

| Flag | Default | Description |
| --- | --- | --- |
| `--device` | | Device flow: prints a code to enter at `<host>/activate`. Can't be combined with `--token`. |
| `--token <PAT>` | | Store an existing token; `-` reads it from stdin |
| `--scope <SCOPE>` | `workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run offline_access` | Scopes to request, separated by spaces (doesn't apply to `--token`) |
| `--no-browser` | | Don't open the browser; only print the URL |

```sh
ttm login
ttm login --device
printf %s "$PAT" | ttm login --token -
ttm --profile dev login --host http://localhost:3000 --scope "workspace:read notee:read"
```

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

JSON: `host`, `profile`, `method` (`browser` | `device` | `token`), `stored_in` (`keyring` | `file` | `env`), `user`
(or `null` when the token lacks `workspace:read`).

## logout

```text
ttm logout
```

Revokes the access token and refresh token on the server, then removes them from the keyring and `credentials.toml`. The
local copy is removed even when the server doesn't confirm. It can't unset `TTM_TOKEN` (it only warns).

```text
Signed out of https://toantranmei.com (profile `default`).
```

JSON: `host`, `profile`, `revoked` (the server confirmed), `removed` (a token was removed from this machine).

## whoami

```text
ttm whoami
```

Calls `/me` (requires `workspace:read`).

```text
An Nguyễn <an@example.com>
host: https://toantranmei.com  profile: default
workspace: Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
```

JSON: `host`, `profile`, `user` (`id`, `email`, `name`, `avatar_url`, `locale`, `plan`), `workspace` (`id`, `name`,
`kind`, `role`, `slug` or `null`).

## status

```text
ttm status
```

What this machine will use; requires no sign-in or network (always exits 0).

```text
ttm 0.1.5
host:      https://toantranmei.com
profile:   default (/Users/an/.config/ttm/config.toml)
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
workspace: not chosen (ttm workspace switch)
server:    API v1 (min ttm 0.1.0), features: notee, factory
```

JSON: `version`, `host`, `profile`, `config_file`, `signed_in`, `token` (`source`, `masked`, `expires_at`, `expired`,
`scope`), `workspace`, `server` (`api_version`, `min_cli`, `features`) or `null`.

## doctor

```text
ttm doctor
```

One check per line: `host`, `discovery` (fetched fresh, not from the cache), `version` (compared with `min_cli`), `token`
(calls `/me`), `keyring`, `git`, `claude`, `ttm-mcp`. Levels are `ok`, `warn`, `FAIL`. **Exits 1 if any check is
`FAIL`**.

```text
[ok  ] host       https://toantranmei.com (profile default)
[ok  ] discovery  api_base https://toantranmei.com/api/v1
[ok  ] version    ttm 0.1.5, server API v1
[ok  ] token      ttm_oat_Ab12… (keyring) as an@example.com
[ok  ] keyring    OS keyring available
[ok  ] git        git version 2.50.1
[warn] claude     Claude Code not found on PATH (needed by ttm runner)
[ok  ] ttm-mcp    /Users/an/.cargo/bin/ttm-mcp
```

JSON: an array of `{name, level, detail}`.

## workspace

### workspace list

```text
ttm workspace list
```

The workspaces you're a member of; `*` marks the profile's current workspace.

```text
* Cá nhân  personal · owner · 0190f3a2-7c1e-7b55-9d0a-3c2b1e4f5a60
  Team Aurora [aurora]  team · member · 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

JSON: an array of `{id, name, kind, role, slug?, current}`.

### workspace switch

```text
ttm workspace switch <WORKSPACE>
```

Sets the current workspace for the profile. `<WORKSPACE>`: an id, slug, or name (exact match, case-insensitive), or the
start of a name if it matches only one workspace. Multiple matches: `matches several workspaces (…); use the id`.

```text
Current workspace for profile `default`: Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
```

### workspace current

```text
ttm workspace current
```

Prints the name and id (the name only when the server can be reached). JSON: `profile`, `workspace_id`, `workspace`.

## token

Tokens are managed on the web; these commands open `<host>/me/ket-noi` (and print the URL). JSON: `{"url": "…"}`.

| Command | Flags | Description |
| --- | --- | --- |
| `ttm token list` | | Open the token list |
| `ttm token create` | `--name <NAME>`, `--scope <SCOPE>` (commas or spaces), `--expires <DAYS>` (`1`–`365`, for example `30d`) | Open the token creation form with the values prefilled |
| `ttm token revoke [ID]` | | Open the revoke page |

```sh
ttm token create --name CI --scope notee:read,workspace:read --expires 90d
```

## notes

Page and database arguments accept an id or a URL. See the [Notee](notee.md) guide.

### notes search

```text
ttm notes search <QUERY> [--limit <N>] [--workspace <ID>]
```

| Flag | Default | Description |
| --- | --- | --- |
| `--limit <N>` | `10` | Maximum number of results (the server returns at most 20) |
| `--workspace <ID>` | current workspace | Limit to one workspace |

```text
0190f3b0-1111-7aaa-8bbb-0123456789ab  Kế hoạch quý 4
0190f3b0-2222-7aaa-8bbb-0123456789ab  Họp tuần
    …chốt kế hoạch phát hành bản 0.2 trước ngày 15…
```

JSON: an array of `{id, workspace_id, title, icon, snippet}`.

### notes get

```text
ttm notes get <ID> [--block <BLOCK>] [-o <FILE>]
```

Prints a page as Notee-flavored Markdown, or a database's title and properties.

| Flag | Description |
| --- | --- |
| `--block <BLOCK>` | Read one part of a truncated page (an id from `unknown_block_ids`) |
| `-o`, `--output <FILE>` | Write the Markdown to a file instead of stdout |

JSON (page): `id`, `title`, `icon`, `url`, `parent`, `properties` (database row), `markdown`, `truncated`,
`unknown_block_ids`. JSON (database): `id`, `title`, `url`, `properties[]`, `views[]`.

### notes query

```text
ttm notes query <DATABASE> (--sql <SQL> | --filter <FILE>) [--page-size <N>] [--cursor <CURSOR>]
```

| Flag | Description |
| --- | --- |
| `--sql <SQL>` | Read-only SQL on the `rows` table |
| `--filter <FILE>` | JSON file `{"filter": …, "sorts": …}` or just the filter; `-` reads stdin |
| `--page-size <N>` | 1–100 rows (server default 100); outside that range `ttm` stops before calling the server: `invalid value '500' for '--page-size <N>': 500 is out of range: --page-size takes 1 to 100 rows per page` |
| `--cursor <CURSOR>` | The `next_cursor` from the previous page |

Exactly one of `--sql` or `--filter` is required. Each row prints as `<id>  <values JSON>`; if there's another page,
stderr prints `more rows: --cursor <CURSOR>`. JSON: `columns`, `rows[{id, values}]`, `next_cursor`.

### notes comments

```text
ttm notes comments <PAGE> [--all]
```

Open comment threads; `--all` includes resolved threads. JSON: an array of threads (`id`, `anchor`, `quote`,
`resolved_at`, `comments[]`).

### notes create

```text
ttm notes create (--parent <PAGE> | --database <DATABASE> | --workspace <WORKSPACE>) -f <FILE> [--title <TITLE>] [--icon <ICON>]
```

| Flag | Description |
| --- | --- |
| `--parent <PAGE>` | Create under this page |
| `--database <DATABASE>` | Create as a row in the database |
| `--workspace <WORKSPACE>` | Create at the top level of the workspace (id) |
| `-f`, `--file <FILE>` | Markdown file (required); `-` reads stdin |
| `--title <TITLE>` | Title (default: the first `# ` line, otherwise the file name) |
| `--icon <ICON>` | Emoji |

```text
Created Ghi chú họp (https://toantranmei.com/notes/0190f3c4-…)
```

JSON: `{id, title, url}`.

### notes edit

```text
ttm notes edit <PAGE> (--old <TEXT> --new <TEXT>)... [--replace-all] [--suggest] [--allow-deleting-content]
ttm notes edit <PAGE> --replace -f <FILE> [--suggest] [--allow-deleting-content]
```

| Flag | Description |
| --- | --- |
| `--old <TEXT>` | Text to find (repeatable), case-sensitive, must match exactly one place |
| `--new <TEXT>` | Replacement for the `--old` in the same position |
| `--replace-all` | Replace every match |
| `--replace` | Replace the whole content with the `-f` file (can't be combined with `--old`/`--new`) |
| `-f`, `--file <FILE>` | File for `--replace`; `-` reads stdin |
| `--suggest` | Create a suggested edit instead of writing directly |
| `--allow-deleting-content` | Allow removing child pages, child databases, and `<truncated/>` parts |

```text
Page updated.
warning: update_content[0]: old_str not found; skipped
```

Or `Suggestion <id> created.`, or `No change applied.` JSON: `applied`, `warnings`, `seq`, `snapshot_seq`,
`suggestion_id`.

### notes comment

```text
ttm notes comment <PAGE> -m <MESSAGE> [--quote <QUOTE> [--prefix <PREFIX>] [--suffix <SUFFIX>] | --thread <THREAD>]
```

| Flag | Description |
| --- | --- |
| `-m`, `--message <MESSAGE>` | Content (required) |
| `--quote <QUOTE>` | Excerpt to anchor to; omit to comment on the whole page |
| `--prefix <PREFIX>`, `--suffix <SUFFIX>` | A few characters right before / after the excerpt, when it appears more than once (requires `--quote`) |
| `--thread <THREAD>` | Reply in this thread (can't be combined with `--quote`) |

```text
Comment thread 0190f3e0-… created.
```

### notes rows upsert

```text
ttm notes rows upsert <DATABASE> -f <FILE>
```

JSON file `[{"id"?: "…", "values": {"<property>": …}}]`, 1–100 rows; `-` reads stdin.

```text
2 row(s) saved: 0190f3c7-…, 0190f3c5-…
```

JSON: `{"ids": [...]}` in input order.

### notes rows delete

```text
ttm notes rows delete <DATABASE> <IDS>...
```

Moves the rows to the trash. `2 row(s) deleted.`

### notes suggestions

```text
ttm notes suggestions <PAGE> [--all]
```

Pending suggested edits (`--all`: also `accepted`, `rejected`, `stale`), with each change's id and the Markdown before
and after.

### notes resolve

```text
ttm notes resolve <SUGGESTION> (--accept [--change <ID>]... | --reject)
```

| Flag | Description |
| --- | --- |
| `--accept` | Accept |
| `--reject` | Reject |
| `--change <ID>` | Accept only these changes (repeatable, requires `--accept`) |

```text
Suggestion 0190f3d1-…: accepted
Stale (anchor text gone): c3
```

JSON: `id`, `status`, `stale_change_ids`.

## project

### project init

```text
ttm project init --key <KEY> [--name <NAME>] [--repo-url <URL>] [--workspace <ID>]
```

Creates a project on the server for the current repo, then links it (like `link`). Requires `factory:write`.

| Flag | Default |
| --- | --- |
| `--key <KEY>` | (required) task key prefix, uppercased |
| `--name <NAME>` | the name of the repo's root directory |
| `--repo-url <URL>` | `git remote get-url origin` |
| `--workspace <ID>` | current workspace |

JSON: the contents of `project.toml`.

### project link

```text
ttm project link <PROJECT>
```

Links the repo to an existing project (key or id) and writes `.ttm/project.toml`, `.ttm/plan`, `.ttm/base`,
`.ttm/.gitignore`.

```text
Linked /Users/an/code/aurora to AUR (Aurora), plan rev 42.
Commit .ttm/project.toml and .ttm/plan; run `ttm rules apply` to install the project rules.
```

### project open

```text
ttm project open
```

Opens `<host>/factory?project=<id>`. JSON: `{"url": "…"}`.

### project status

```text
ttm project status
```

```text
project: AUR (0190f400-…)
host:    https://toantranmei.com
rev:     42
pending change request: 0190f4a1-…
```

## sync

```text
ttm sync [--check | --resolve]
```

Pulls the plan into `.ttm/plan` and sends local edits as a change request. See [Factory](factory.md).

| Flag | Description |
| --- | --- |
| `--check` | Only report whether local and server have drifted; **exits 1 on drift**. Writes nothing. |
| `--resolve` | On conflict, let the local agent (`claude`) merge in a separate worktree and send a new change request |

JSON: `local_rev`, `server_rev`, `local_changes`, `server_changes`, `change_request`, `skipped`, `nothing_to_send`,
`conflicts[{task, field, base, local, server}]`, `in_sync`.

## task

Run inside a linked repo (except `show`, `claim`, `done`, `approve`, `split`, which accept any key).

| Command | Flags | Description | Scope |
| --- | --- | --- | --- |
| `ttm task list` | `--state <STATE>` (state id or category), `--mine` | The project's tasks | `factory:read` |
| `ttm task show <KEY>` | | Title, state, role, assignee, description | `factory:read` |
| `ttm task claim <KEY>` | | Assign the task to yourself | `factory:write` |
| `ttm task done <KEY>` | | Move to `done` (guards and approval gates still apply) | `factory:write` |
| `ttm task approve <KEY>` | `--reject` | Approve / reject the pending step | `factory:write` |
| `ttm task split <KEY> -f <FILE>` | `-f`, `--file` (required; `-` = stdin) | Create subtasks from JSON (camelCase or snake_case keys, see [Factory](factory.md)) | `factory:write` |

```text
AUR-12     doing          DEV       Dev bot          Đăng nhập bằng Google
```

```text
AUR-12  Đăng nhập bằng Google
state: doing  role: DEV  assignee: Dev bot

Cho phép đăng nhập bằng tài khoản Google.
```

Other output: `AUR-12 is now yours.`, `AUR-12 is now done.`, `AUR-12 approved; state done.`, `AUR-12 rejected; state doing.` (`--reject`),
`2 subtask(s) created under AUR-12.` JSON: the task object (camelCase, `id` is the key), or `{tasks, warnings}` for
`split`.

## rules

### rules apply

```text
ttm rules apply
```

Writes the `<!-- ttm:begin -->…<!-- ttm:end -->` block into `AGENTS.md` and `CLAUDE.md`, merges hooks into
`.claude/settings.json`, and saves `.ttm/cache/rules.json`. Prints the changed files or `Rules already up to date.` JSON:
`changed`, `checks`.

## hook

Called by Claude Code through hooks (installed by `rules apply`); reads the hook JSON on stdin. **Exits 2** when it
blocks, with the reason on stderr; exits 0 when it allows or when outside a linked repo.

| Command | Hook | Checks |
| --- | --- | --- |
| `ttm hook pre-bash` | `PreToolUse` (matcher `Bash`) | `no_push_main` (protected branches from the cached rules, see [Factory](factory.md)), `no_verify`, `conventional_commits` on `tool_input.command` |
| `ttm hook stop` | `Stop` | `tests_before_done`: a test command appears in `transcript_path` (skipped when `stop_hook_active`) |

## runner

| Command | Description |
| --- | --- |
| `ttm runner up` | Run the runner in the foreground (`Ctrl-C` to stop); registers the machine if needed and cleans up old worktrees |
| `ttm runner down` | Stop the process in `runner.pid` |
| `ttm runner status` | Id, name, running, capacity, harnesses, repos |
| `ttm runner config [flags]` | Edit `runner.toml` |
| `ttm runner install-service` | Install a service that starts at login (launchd, systemd `--user`, Task Scheduler), carrying the `PATH` of the shell that runs it |
| `ttm runner uninstall-service` | Stop and remove the service (plist, unit or Task Scheduler task) |

`runner config` flags:

| Flag | Default | Description |
| --- | --- | --- |
| `--max-parallel <N>` | from CPU and RAM | Number of concurrent agents |
| `--accept-team-jobs <true\|false>` | `false` | Accept jobs for teammates' tasks |
| `--worktree-root <DIR>` | `~/.ttm/worktrees` | Where worktrees are created |
| `--keep-days <N>` | `3` | Days to keep finished worktrees |
| `--repo <KEY=PATH>` | | Local clone of a project (repeatable) |
| `--name <NAME>` | machine name | Name shown on the web |
| `--auto-merge <true\|false>` | `false` | Automatically merge the head of the merge queue and push |

JSON for `status`: `runner_id`, `name`, `running`, `pid`, `capacity`, `accept_team_jobs`, `harnesses`, `repos`,
`worktree_root`. For `config`: the contents of `runner.toml`. For `down`: `stopped`, `pid`. For `install-service`:
`installed`, `path` (the service's `PATH`; `null` on Windows). For `uninstall-service`: `removed` (`null` when none was
installed).

## agent

| Command | Flags | Description |
| --- | --- | --- |
| `ttm agent run <KEY>` | | Run the task right now on this machine and print events; exits 0 only on `succeeded` |
| `ttm agent logs <RUN>` | `-f`, `--follow` | The run's events; `-f` polls every 2 seconds |
| `ttm agent stop <RUN>` | | Kill switch |

JSON for `run`: `run_id`, `status`, `branch`, `worktree`, `pr_url`, `paused_until`. `logs --json` prints each event as one
JSON line (`id`, `at`, `kind`, `message`, `data`).

## mcp

### mcp install

```text
ttm mcp install --client <CLIENT> [--name <NAME>] [--command <PATH>] [--toolsets <TOOLSETS>] [--dry-run]
```

| Flag | Default | Description |
| --- | --- | --- |
| `--client <CLIENT>` | (required) | `claude-code`, `claude-desktop`, `cursor`, `codex` |
| `--name <NAME>` | `ttm` | Server name in the client configuration |
| `--command <PATH>` | `ttm-mcp` next to `ttm`, then on `PATH` | Command the client runs |
| `--toolsets <TOOLSETS>` | | Pass `--toolsets` to `ttm-mcp` |
| `--dry-run` | | Only print; don't write |

An explicit `--host`/`--profile` (or `TTM_HOST`) is pinned into `args`. The configuration file is merged and backed up
(`<file>.bak-<unix seconds>`). JSON: `client`, `file`, `backup`, `changed`, `dry_run`, `entry{name, command, args}`.
Details: [Connecting MCP](mcp.md).

## self-update

```text
ttm self-update [--check] [--version <VERSION>]
```

| Flag | Description |
| --- | --- |
| `--check` | Only report whether a new version exists |
| `--version <VERSION>` | Install exactly this version (for example `0.2.0`), even if older |

Downloads from GitHub Releases, verifies SHA-256 and (when `gh` is available) the attestation, and replaces `ttm` and the
`ttm-mcp` in the same directory. JSON: `current`, `latest`, `update_available`, `updated`, `attestation`.

## completion

```text
ttm completion <SHELL>
```

`<SHELL>`: `bash`, `elvish`, `fish`, `powershell`, `zsh`. Prints the script to stdout; doesn't read the configuration.
See [Installation](cai-dat.md).

## ttm-mcp

```text
ttm-mcp [--host <URL>] [--profile <NAME>] [--toolsets <LIST>] [-v...]
```

MCP server over stdio. See [Connecting MCP](mcp.md) and [MCP tools](mcp-cong-cu.md).
