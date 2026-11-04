---
title: Command reference
description: Every ttm command and subcommand with syntax, flags, defaults, examples, JSON output, and exit codes.
order: 10
section: reference
---

```text
ttm [OPTIONS] <COMMAND>
```

`ttm --help` and `ttm <command> --help` print the built-in help (in English). This page covers version **0.2.0**. Output conventions (result line, `Next:`, glyphs, badges, colours): [Command line interface](giao-dien.md).

## Global flags

Available on every command, before or after the subcommand (`ttm whoami --json` = `ttm --json whoami`).

| Flag | Variable | Description |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | Server, for example `http://localhost:3000` |
| `--profile <NAME>` | `TTM_PROFILE` | Profile in `config.toml` |
| `--json` | | Print results as JSON on stdout (0.1.6 shapes; new keys are only added) |
| `--plain` | `TTM_PLAIN=1` | Plain text for logs: no colours, frames, glyphs or emoji |
| `-y`, `--yes` | | Answer yes to confirmations and take defaults instead of asking (scripts, CI) |
| `-v`, `--verbose` | `TTM_LOG` | More logging to stderr (`-v` info, `-vv` debug) |
| `-h`, `--help` | | Help |
| `-V`, `--version` | | Version (`ttm 0.2.0`; top level only) |

## Exit codes

| Code | When |
| --- | --- |
| `0` | Success |
| `1` | Error (network, authentication, API, file…); `ttm doctor` has a `FAIL` item; `ttm sync --check` finds drift; `ttm sync` hits a conflict; `ttm agent run` ends with a status other than `succeeded` |
| `2` | Invalid command-line usage (unknown flag, missing argument); a flag is missing and nobody can be asked (`missing_flag`); a confirmation is needed without a terminal or `--yes` (`confirmation_required`); the server lacks the feature (`unsupported_feature`); a flag value is invalid (`invalid_flag`). For `ttm hook …`: the action is blocked by a rule |

Errors go to stderr in one pattern:

```text
✕ <summary>
  rule: <rule>  field: <field>
  Fix: <how to fix it>
  Help: https://toantranmei.com/help/ttm/ma-loi#<rule>
```

With `--plain` the first line is `ERROR: <summary>`. With `--json`, `{"error": {"code", "message", "status?", "rule?",
"field?", "params?"}}` is also printed to stdout (see [Configuration](cau-hinh.md) and [Error codes](ma-loi.md)).

## Server features

New commands call the API only when the server's `discovery.features` has the matching string; otherwise they print
`✕ This server does not support <x> yet` (rule `unsupported_feature`, exit 2) and call nothing.

| Feature | Commands |
| --- | --- |
| `workflow_v2` | `project init` (templates, preset agents), `workflow`, `automation` |
| `agent_v2` | `agent list/show/create/edit/pause/resume/archive/skills/mcp`, `skill`, `mcp-server` |
| `ascode_v2` | `workflow validate --file`, `cr` (configuration requests), `sync` (configuration folders in `.ttm/`) |
| `templates` | `template` |
| `membership` | Mei-er line of `whoami`/`login`, badges |
| `confirmations` | two-step confirmations through MCP (see [MCP tools](mcp-cong-cu.md)) |

## Summary

| Command | Description |
| --- | --- |
| [`login`](#login) | Sign in (browser, `--device`, `--token`) |
| [`logout`](#logout) | Revoke the token and remove it from this machine |
| [`whoami`](#whoami) | The signed-in user |
| [`status`](#status) | Host, profile, token, and workspace in use |
| [`doctor`](#doctor) | Check your environment |
| [`workspace`](#workspace) | `list`, `switch`, `current`, `members` |
| [`token`](#token) | `list`, `create`, `revoke` (opens the web page) |
| [`notes`](#notes) | `search`, `get`, `query`, `comments`, `create`, `edit`, `comment`, `rows`, `suggestions`, `resolve` |
| [`project`](#project) | `init`, `link`, `open`, `status` |
| [`sync`](#sync) | Sync the plan |
| [`task`](#task) | `list`, `show`, `claim`, `done`, `approve`, `split` |
| [`rules`](#rules) | `apply` |
| [`hook`](#hook) | `pre-bash`, `stop` (called by the agent) |
| [`workflow`](#workflow) | `show`, `validate`, `simulate`, `diff`, `publish`, `upgrade`, `copy` |
| [`automation`](#automation) | `list`, `show`, `test`, `enable`, `disable`, `runs` |
| [`runner`](#runner) | `up`, `down`, `status`, `config` (+ `config mcp`, `config claude-token`), `install-service`, `uninstall-service` |
| [`agent`](#agent) | `run`, `logs`, `stop`, `list`, `show`, `create`, `edit`, `pause`, `resume`, `archive`, `skills`, `mcp` |
| [`skill`](#skill) | `list`, `push`, `pull`, `show`, `review`, `import` |
| [`mcp-server`](#mcp-server) | `list`, `add`, `connect`, `remove` (remote MCP servers of the workspace) |
| [`cr`](#cr) | `list`, `show`, `approve`, `reject` |
| [`inbox`](#inbox) | Notifications, `--watch`, `read` |
| [`template`](#template) | `list`, `publish`, `install` |
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
| `--scope <SCOPE>` | `workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run factory:admin offline_access` | Scopes to request, separated by spaces (doesn't apply to `--token`) |
| `--no-browser` | | Don't open the browser; only print the URL |

```sh
ttm login
ttm login --device
printf %s "$PAT" | ttm login --token -
ttm --profile dev login --host http://localhost:3000 --scope "workspace:read notee:read"
```

```text
✓ Signed in as An Nguyễn <an@example.com>
  host https://toantranmei.com · profile default · token in keyring
✦ Mei-er since 2025-11  Thank you for being a Mei-er. You keep this going.
Next: ttm project init
```

Mei-er members get one more gold line (tier, the month they joined, thanks by tier; on the anniversary, a greeting).
`Next:` is `ttm project init` when the directory is not linked to a project yet, else `ttm whoami`. Older tokens without
`factory:admin` keep working; admin commands fail with `insufficient_scope` and suggest `ttm login`.

JSON: `host`, `profile`, `method` (`browser` | `device` | `token`), `stored_in` (`keyring` | `file` | `env`), `user`
(or `null` when the token lacks `workspace:read`), `meta.membership` (`{tier, status, since, until, badge}` or `null`).

## logout

```text
ttm logout
```

Revokes the access token and refresh token on the server, then removes them from the keyring and `credentials.toml`. The
local copy is removed even when the server doesn't confirm. It can't unset `TTM_TOKEN` (it only warns).

```text
✓ Signed out of https://toantranmei.com
  profile default · revoked on the server: yes
Next: ttm login
```

JSON: `host`, `profile`, `revoked` (the server confirmed), `removed` (a token was removed from this machine).

## whoami

```text
ttm whoami
```

Calls `/me` (requires `workspace:read`).

```text
An Nguyễn <an@example.com>
     user  An Nguyễn
    email  an@example.com
     host  https://toantranmei.com
  profile  default
workspace  Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
✦ Mei-er since 2025-11  Thank you for being a Mei-er. You keep this going.
```

Non-members see one faint line `Support toantranmei → https://toantranmei.com/mei-er` (only in `whoami`; no other
command mentions it). With `--plain`: `user: An Nguyễn`… and the badge `[Mei-er]`.

JSON: `host`, `profile`, `user` (`id`, `email`, `name`, `avatar_url`, `locale`, `plan`, `membership?`), `workspace` (`id`,
`name`, `kind`, `role`, `slug` or `null`), `meta.membership`.

## status

```text
ttm status
```

What this machine will use; requires no sign-in or network (always exits 0).

```text
✓ Signed in to https://toantranmei.com
  version  ttm 0.2.0
     host  https://toantranmei.com
  profile  default (/Users/an/.config/ttm/config.toml)
    token  ✓ ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
  keyring  ✓ OS keyring
workspace  not chosen
   server  ✓ API v1 (min ttm 0.1.0), features: notee, factory, workflow_v2, agent_v2
Next: ttm workspace switch <name>
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
! 1 warning(s), no failure
CHECK      STATUS  DETAIL
────────────────────────────────────────────────────────────────────────
host       ✓       https://toantranmei.com (profile default)
discovery  ✓       api_base https://toantranmei.com/api/v1
version    ✓       ttm 0.2.0, server API v1
token      ✓       ttm_oat_Ab12… (keyring) as an@example.com
keyring    ✓       OS keyring available
git        ✓       git version 2.50.1
claude     !       Claude Code not found on PATH (needed by ttm runner)
ttm-mcp    ✓       /Users/an/.cargo/bin/ttm-mcp
Next: install Claude Code, then ttm doctor
```

`--plain`: `ok`, `warn`, `FAIL` instead of `✓`, `!`, `✕`, no rule under the header.

JSON: an array of `{name, level, detail}`.

## workspace

### workspace list

```text
ttm workspace list
```

The workspaces you're a member of; `●` (`--plain`: `*`) marks the profile's current workspace.

```text
2 workspace(s), current: Team Aurora
   NAME         SLUG    ROLE    KIND      ID
────────────────────────────────────────────────────────────────────────
   Cá nhân      -       owner   personal  0190f3a2-7c1e-7b55-9d0a-3c2b1e4f5a60
●  Team Aurora  aurora  member  team      0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

JSON: an array of `{id, name, kind, role, slug?, current}`.

### workspace switch

```text
ttm workspace switch <WORKSPACE>
```

Sets the current workspace for the profile. `<WORKSPACE>`: an id, slug, or name (exact match, case-insensitive), or the
start of a name if it matches only one workspace. Multiple matches: `matches several workspaces (…); use the id`.

```text
✓ Current workspace: Team Aurora
  id 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71 · profile default
Next: ttm project init
```

### workspace current

```text
ttm workspace current
```

Prints the name and id (the name only when the server can be reached). JSON: `profile`, `workspace_id`, `workspace`.

### workspace members

```text
ttm workspace members [--workspace <ID>]
```

Members of a workspace (default: the current one): name with Mei-er badge, email, role. Requires `workspace:read`.

```text
NAME                    EMAIL              ROLE
An Nguyễn [Mei-er]      an@example.com     owner
Lan                     lan@example.com    member
```

JSON: array of `{user{id, name, email, avatar_url, membership?}, role, membership?}`.

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
✓ Created Ghi chú họp
  id 0190f3c4-…
  https://toantranmei.com/notes/0190f3c4-…
Next: ttm notes get 0190f3c4-…
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
ttm project init [--name <NAME>] [--key <KEY>] [--workspace <ID>] [--repo <URL>] [--workflow <W>]
                 [--agents create|reuse|none] [--runner this|none] [--yes]
```

Creates a project on the server for the current repo, then links it (like `link`). Requires `factory:write`. In a
terminal it is a **wizard**: name → key (suggested from the name, `^[A-Z][A-Z0-9]{1,9}$`) → workspace → repo → workflow →
preset agents (previewed from `GET /factory/agent-presets`; when an agent with the same role exists you choose to reuse
it or create a new one) → runner → confirmation. Every question has a flag; `--yes` takes the defaults. After the wizard,
stderr shows `Equivalent: ttm project init … --yes`. 0.1.6 compatible: not a terminal, `--key` given and none of
`--workflow`/`--agents`/`--runner` → the project is created right away as before, nothing asked. Workflows and preset
agents need the `workflow_v2` feature.

| Flag | Default | Description |
| --- | --- | --- |
| `--name <NAME>` | the name of the repo's root directory | Project name |
| `--key <KEY>` | suggested from the name | Task key prefix (2–10 capitals or digits, starting with a letter) |
| `--workspace <ID>` | current workspace | |
| `--repo <URL>` (old: `--repo-url`) | `git remote get-url origin` | |
| `--workflow <W>` | `balanced` | `professional` (9 statuses, 6 agents), `balanced` (5 statuses, 3 agents, about 40% fewer tokens), `kanban`, `scrum`, `autopilot` or a template slug |
| `--agents <A>` | `create` | `create` the template's agents, `reuse` agents with the same role, `none` |
| `--runner <R>` | `this` | `this`: writes `[repos] KEY = <this directory>` to `runner.toml`; `none` |

```text
OK: Created project WEB (Web todo)
  linked /home/an/web-todo · plan rev 1 · workflow balanced
AGENT              CODE   STATUS
[agent] DEV · Code  dev    created
[agent] QA · Test   qa     created
[agent] Lead        lead   reused
Next: ttm runner up
```

JSON: the contents of `project.toml` (+ `agents[{code, agent_id, created}]` when there are any).

### project link

```text
ttm project link <PROJECT>
```

Links the repo to an existing project (key or id) and writes `.ttm/project.toml`, `.ttm/plan`, `.ttm/base`,
`.ttm/.gitignore`.

```text
✓ Linked /Users/an/code/aurora to AUR (Aurora)
  plan rev 42
  Commit .ttm/project.toml and .ttm/plan.
Next: ttm rules apply
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
| `ttm task approve <KEY>` | `--reject`, `-y/--yes` | Shows the task, then asks for a confirmation before approving / rejecting the pending step; needs `--yes` without a terminal | `factory:write` |
| `ttm task split <KEY> -f <FILE>` | `-f`, `--file` (required; `-` = stdin) | Create subtasks from JSON (camelCase or snake_case keys, see [Factory](factory.md)) | `factory:write` |

```text
AUR · 2 task(s)
   KEY     TITLE                          ROLE  ASSIGNEE            EST
────────────────────────────────────────────────────────────────────────
●  AUR-12  Đăng nhập bằng Google          DEV   🤖 Dev bot ✦ Mei-er   3
◐  AUR-13  Trang hồ sơ                    QA    👤 Lan                2
Next: ttm task show AUR-12
```

```text
╭─ AUR-12 · Đăng nhập bằng Google ─╮
│ ● doing                          │
╰──────────────────────────────────╯
   status  doing (in-progress)
     role  DEV
 assignee  🤖 Dev bot
delegator  👤 An ✦ Mei-er

Acceptance criteria
  ✓ Google button shows
  ○ Email is verified

Cho phép đăng nhập bằng tài khoản Google.

Next: ttm agent run AUR-12
```

Other output: `✓ AUR-12 is now yours`, `✓ AUR-12 is now done`, `✓ AUR-12 approved`, `✓ AUR-12 rejected` (`--reject`),
`✓ 2 subtask(s) created under AUR-12` JSON: the task object (camelCase, `id` is the key), or `{tasks, warnings}` for
`split`.

## rules

### rules apply

```text
ttm rules apply
```

Writes the `<!-- ttm:begin -->…<!-- ttm:end -->` block into `AGENTS.md` and `CLAUDE.md`, merges hooks into
`.claude/settings.json`, and saves `.ttm/cache/rules.json`. Prints the changed files or `✓ Rules already up to date` JSON:
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

### runner config mcp

Local MCP servers of this machine (`[mcp.local.<name>]` tables in `runner.toml`, see [Runner](runner.md)). The file is
edited with its comments kept. The server is told only names, `allow` and `team_jobs`, never commands or environment.

| Command | Flags | Description |
| --- | --- | --- |
| `ttm runner config mcp add <NAME> --command <CMD>` | `--arg <ARG>` (repeatable), `--env K=env:VAR` or `K=<non-secret value>` (repeatable) | Add or replace a local MCP server. Names `^[a-z0-9][a-z0-9-]{0,31}$`, not `ttm`. Values that look like tokens are refused: use `env:VAR` |
| `ttm runner config mcp allow <AGENT>` | `--mcp <NAME>` (repeatable; default: every local server) | Let an agent (handle) use it |
| `ttm runner config mcp deny <AGENT>` | `--mcp <NAME>` (repeatable) | Take it back |
| `ttm runner config mcp team-jobs <NAME> on\|off` | | Whether jobs delegated by someone else may use it (default `off`) |
| `ttm runner config mcp remove <NAME>` | | Remove it |
| `ttm runner config mcp list` | | Table of name, command, `allow`, team jobs + the line "Announced to the server: names and allowed agents only" |

```sh
ttm runner config mcp add github-local --command npx --arg -y --arg @modelcontextprotocol/server-github --env GITHUB_TOKEN=env:GITHUB_TOKEN
ttm runner config mcp allow dev --mcp github-local
```

### runner config claude-token

```text
printf %s "$TOKEN" | ttm runner config claude-token
ttm runner config claude-token --remove
```

Saves (in the keyring) a Claude Code OAuth token from `claude setup-token`, read on stdin, used for teammates' jobs in a
separate Claude configuration directory per run. With a token the runner announces the `team_isolation` feature. See
[Runner](runner.md).

`ttm runner status` also shows `features` and a local MCP table (name, allow, team jobs).

JSON for `status`: `runner_id`, `name`, `running`, `pid`, `capacity`, `accept_team_jobs`, `harnesses`, `repos`,
`worktree_root`, `features`, `local_mcp`. For `config`: the contents of `runner.toml`. For `down`: `stopped`, `pid`. For `install-service`:
`installed`, `path` (the service's `PATH`; `null` on Windows). For `uninstall-service`: `removed` (`null` when none was
installed).

## agent

| Command | Flags | Description |
| --- | --- | --- |
| `ttm agent run <KEY>` | | Run the task right now on this machine and print events; exits 0 only on `succeeded` |
| `ttm agent logs <RUN>` | `-f`, `--follow` | The run's events; `-f` polls every 2 seconds |
| `ttm agent stop <RUN>` | | Kill switch |

JSON for `run`: `run_id`, `status`, `branch`, `worktree`, `pr_url`, `paused_until`. `logs --json` prints each event as one
JSON line (`id`, `at`, `kind`, `message`, `data`). Human `logs`: `10:00:02 › edited src/login.rs` (glyph by `kind`: `·`
thought, `›` action, `◆` mcp_call, `✕` error, `NN%` progress).

### Agent administration

Needs the `agent_v2` feature; every command takes `--workspace <ID>` (default: the current one). `<AGENT>` is a handle
(`dev`, `@dev`), a name or an id. Reading needs `factory:read`; changes need `factory:admin` (owner/admin of the
workspace).

| Command | Description |
| --- | --- |
| `ttm agent list` | Table: 🤖 name + owner's badge, `@handle`, role, model, skills (`◐` = a version waits for review), MCP, status |
| `ttm agent show <AGENT>` | Header + git identity, profile, skills, MCP, projects |
| `ttm agent create [--preset <CODE>] [fields]` | Wizard in a terminal (a preset or one question at a time); `--yes` + flags for scripts |
| `ttm agent edit <AGENT> [fields] [--git-email <E>] [--git-name <N>] [--bio <TEXT>]` | Flags only; `""` = back to the default |
| `ttm agent pause\|resume\|archive <AGENT>` | With a confirmation (`--yes`) |
| `ttm agent skills add <AGENT> <SLUG>[@<V>]` / `skills rm <AGENT> <SLUG>` | Attach (pin a version with `@V`) / detach a skill |
| `ttm agent mcp add <AGENT> <SERVER\|local:NAME> [--tools a,b\|*] [--required]` / `mcp rm <AGENT> <SERVER>` | Grant / remove an MCP server; `--required` = run only on runners that provide that local MCP |

Fields of `create`/`edit`: `--name`, `--handle`, `--role`, `--model` (`opus`, `sonnet`, `haiku` or an id), `--budget`
(tokens per run), `--daily-budget`, `--turns`, `--permission` (`read`, `edit`, `full`, `custom`), `--skills a,b`,
`--instructions-file <FILE>` (`-` = stdin).

```text
3 agent(s)
   NAME              HANDLE  ROLE  MODEL   SKILLS        MCP  STATUS
🤖 DEV · Code ✦ Mei-er @dev  DEV   sonnet  ttm-tdd ◐     1    active
```

## workflow

Needs `workflow_v2` (`validate --file` needs `ascode_v2`). `--project <KEY>` defaults to the linked project. Changes need
`factory:admin`.

| Command | Description |
| --- | --- |
| `ttm workflow show [--version <N\|draft>]` | Header (name · vN · draft?), statuses table (glyph by category, name, role, auto-assign 🤖/👤, WIP), transitions table (from → to, guard, ⚡ rule count) |
| `ttm workflow validate [--file .ttm/workflow.yaml]` | With `--file`: checks the as-code file on the server; without: checks the draft. One line per issue `✕ <rule\|code> <field>: <message>`; **exits 1 on issues** |
| `ttm workflow simulate <TASK> --to <STATUS> [--version <V>] [--roles a,b]` | Each step ✓/✕ + reason; exits 1 when blocked |
| `ttm workflow diff [<FROM> <TO>]` | Default: the project's version and the draft; `+`/`-`/`~` for statuses, transitions, rules |
| `ttm workflow publish [--note <TEXT>]` | Shows the diff → confirms → publishes the draft as a new version (`note` ≤ 280) |
| `ttm workflow upgrade [--to <N>]` | Shows the preview (tasks that move) → confirms → upgrades the project |
| `ttm workflow copy [--name <NAME>]` | Copies it into an editable workflow of the workspace |

```text
OK: AUR-12 can move to qc
  ok  doing -> qc  (transition t3, 1 rule)
```

## automation

Automation rules of the project (needs `workflow_v2`; `--project <KEY>`). `enable`/`disable` need `factory:admin`.

| Command | Description |
| --- | --- |
| `ttm automation list` | Table: `●`/`⏸` on/off, name, trigger summary, runs in 7 days |
| `ttm automation show <RULE>` | When / If / Then as a tree |
| `ttm automation test <RULE> --task <KEY>` | Dry run (nothing changes): each action ✓ would run / ✕ skipped + reason |
| `ttm automation enable\|disable <RULE>` | Turn on / off |
| `ttm automation runs [<RULE>] [--limit 20]` | Recent runs |

`<RULE>`: id, slug or name.

## skill

Skills of the workspace (needs `agent_v2`; `--workspace <ID>`). See [Runner](runner.md) for how the runner installs them.

| Command | Description |
| --- | --- |
| `ttm skill list [--tag <T>] [--builtin]` | Skills table (`--builtin` includes built-in skills) |
| `ttm skill push <DIR> [--slug <S>] [--note <TEXT>]` | Checks locally (SKILL.md + front matter, ≤ 5 MB, ≤ 200 files, safe paths), shows the manifest + sha256, creates the skill when missing (`--slug` defaults to the directory name), uploads the deterministic zip → `v<N> pending review` or `v<N> ready`. Nothing changed → `No changes` |
| `ttm skill pull <SLUG> [--version <V>] [--dir <DIR>]` | Downloads, checks the sha, unpacks (default `.ttm/skills/<slug>` in a linked repo, else `./<slug>`) |
| `ttm skill show <SLUG> [--version <V>]` | Versions and review state |
| `ttm skill review list` | Review queue |
| `ttm skill review approve <SLUG>@<V> [--note <TEXT>]` | Shows the diff with the current version → confirms → approves |
| `ttm skill review reject <SLUG>@<V> --note <TEXT>` | Rejects (a note is required) |
| `ttm skill import --git <URL> --path <P> [--ref main] [--slug <S>]` | Imports from GitHub/GitLab |

```text
OK: ttm-tdd v3 pending review
  sha256 3c9e7159… · 3 file(s) · 1.2 KB
Next: ttm skill review approve ttm-tdd@3
```

## mcp-server

Remote MCP servers of the workspace, used by agents through the server's gateway (needs `agent_v2`; `--workspace <ID>`).

| Command | Description |
| --- | --- |
| `ttm mcp-server list` | Table: name, URL (cut), auth, status, tool count, connected (workspace/me) |
| `ttm mcp-server add [--name] [--url] [--transport http\|sse] [--auth none\|workspace\|delegator] [--secret-header <H>] [--secret-env <VAR>]` | Wizard; a secret only through `--secret-env VAR` or a hidden prompt in a terminal, never a flag value |
| `ttm mcp-server connect <NAME> [--for workspace\|me] [--no-browser]` | Opens the OAuth page (or prints the URL), waits until connected (polls every 2 seconds, at most 5 minutes) |
| `ttm mcp-server remove <NAME>` | Confirms, warns about agents using it |

## cr

Change requests: the plan (from `ttm sync`) and configuration (workflow, rules, agents, skills; needs `ascode_v2`).

| Command | Flags | Description |
| --- | --- | --- |
| `ttm cr list` | `--project <KEY>`, `--kind plan\|workflow\|rule\|agent\|skill`, `--status pending\|applied\|rejected\|all` (default `pending`) | Table: short id, kind, status, author + badge, files, age |
| `ttm cr show <ID>` | | Header + diff of each file |
| `ttm cr approve <ID>` | | Confirms then applies (409 `stale` when the server changed meanwhile) |
| `ttm cr reject <ID>` | `--note <TEXT>` | Rejects |

## inbox

```text
ttm inbox [--unread] [--kind factory,…] [--watch] [--interval 5]
ttm inbox read [<ID>… | --all]
```

Your notifications: table of `●` unread, kind/reason, text (cut), age, link. `--watch` prints the list, then polls every
`--interval` seconds (at least 2), one line per new notification; `Ctrl-C` stops cleanly; with `--json` it prints NDJSON.

```text
● 14:02 skill_review  Skill ttm-tdd v3 waiting for review → https://toantranmei.com/factory/skills/…
```

## template

The public template library (needs `templates`).

| Command | Description |
| --- | --- |
| `ttm template list [--kind workflow\|agent\|skill\|rule-pack\|bundle] [--query <Q>]` | Search templates |
| `ttm template publish --from workflow\|agent\|skill <SOURCE> [--slug] [--name] [--price <VND>] [--workspace]` | Export preview (shows what is left out) → confirm → publish. Prices are set on the web: the command prints the `/me/items/<id>` link |
| `ttm template install <SLUG> [--workspace <ID>]` | Preview (creates: statuses, rules, agents, skills + "runs code" flag) → confirm → install; skills waiting for review → `Next: ttm skill review list` |

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
