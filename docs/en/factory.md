---
title: Factory
description: Link a repo to a Factory project, the .ttm layout, syncing the plan through change requests, resolving conflicts, agent rules, and task commands.
order: 8
section: guides
---

Factory manages projects for both people and agents: **projects** (with a key, for example `AUR`), **tasks** (`AUR-12`),
**workflows** (states, transitions, approval gates), **roles** (`PM`, `BA`, `UI/UX`, `DEV`, `GUARDIAN`, `QC`, or the
workspace's own roles), and **assignees** (people or agents). **The web is the source of truth**: the repo keeps a
Markdown copy of the plan; local edits go up as **change requests** for someone with the right role to approve, and
never overwrite.

The commands below (except `project init/link`) run inside a linked repo: `ttm` looks for `.ttm/project.toml` in the
current directory or its parents.

## Link a repo

### Create a new project for this repo

```sh
cd ~/code/aurora
ttm project init --key AUR --name "Aurora"
```

```text
Created project AUR (Aurora) and linked /Users/an/code/aurora.
```

| Flag | Default |
| --- | --- |
| `--key <KEY>` | (required) task key prefix, uppercased |
| `--name <NAME>` | the name of the repo's root directory |
| `--repo-url <URL>` | `git remote get-url origin` |
| `--workspace <ID>` | the profile's current workspace |

The repo provider is inferred from the URL: `github.com` → `github`, containing `gitlab` → `gitlab`, anything else (or
no remote) → `local`. The default branch comes from `origin/HEAD`; otherwise `main` if it exists, otherwise `master`. A
key that is already taken: `409 conflict` (`rule: key_taken`).

### Wizard (0.2)

In a terminal, `ttm project init` without flags asks in turn: name → key (suggested from the name, `^[A-Z][A-Z0-9]{1,9}$`)
→ workspace → repository (default `origin`) → workflow (`Professional — 9 statuses, 6 agents`, `Balanced — 5 statuses, 3
agents, about 40% fewer tokens`, or another template) → preset agents (reuse an agent with the same role or create one) →
runner (`this` writes `[repos] KEY = <directory>` to `runner.toml`). Every question has a flag: `--name --key --workspace
--repo --workflow professional|balanced|kanban|scrum|autopilot|<slug> --agents create|reuse|none --runner this|none`;
`--yes` takes the defaults. Not a terminal with only `--key` behaves as in 0.1.6. After the wizard `ttm` prints the
equivalent command (`Equivalent: …`) on stderr.

### Link to an existing project

```sh
ttm project link AUR          # key or id
```

```text
Linked /Users/an/code/aurora to AUR (Aurora), plan rev 42.
Commit .ttm/project.toml and .ttm/plan; run `ttm rules apply` to install the project rules.
```

`ttm` looks for the project in your workspaces, downloads the plan, and writes `.ttm/`. Not found:
``no project `AUR` in your workspaces``.

```sh
ttm project status            # key, id, host, synced rev, pending change request
ttm project open              # open <host>/factory?project=<id> in the browser
```

## The `.ttm` directory

```text
.ttm/
├─ project.toml        link (commit)
├─ plan/               copy of the plan to read and edit (commit)
│  ├─ index.md
│  └─ tasks/
│     ├─ AUR-1.md
│     └─ AUR-12.md
├─ base/               server copy from the last sync, base for 3-way diff (ignored)
├─ cache/              rules.json for hooks, dropped/ (ignored)
├─ resolve/            scratch space while resolving conflicts (ignored)
└─ .gitignore          base/, cache/, resolve/
```

`project.toml`:

```toml
host = "https://toantranmei.com"
project_id = "0190f400-…"
key = "AUR"
synced_rev = 42
default_branch = "main"
pending_change_request = "0190f4a1-…"   # only present while a change request awaits approval
```

### Plan files

`plan/index.md` summarizes sprints, milestones, and the task table (read-only; edit sprints and milestones on the web).
Each task is a file `plan/tasks/<KEY>.md` whose front matter holds **JSON values** (which are also valid YAML):

```markdown
---
key: "AUR-12"
title: "Đăng nhập bằng Google"
state: "doing"
role: "DEV"
assignee: "agent:0190f3ff-…"
deps: ["AUR-10"]
estimate: 3
priority: "high"
type: "feature"
labels: ["auth"]
ref: null
parent: null
status: "in-progress"
rev: 7
---

Cho phép đăng nhập bằng tài khoản Google.

## Acceptance criteria

- [x] Nút "Đăng nhập bằng Google"
- [ ] Liên kết với tài khoản có sẵn theo email
```

| Editable locally | Notes |
| --- | --- |
| `title`, `state`, `role`, `estimate`, `priority`, `type`, `labels`, `ref` | JSON values |
| `assignee` | `"user:<id>"`, `"agent:<id>"`, or `null` |
| `deps` | Array of task keys |
| Description | The body before `## Acceptance criteria` |
| Acceptance criteria | The `- [ ]` / `- [x]` list under `## Acceptance criteria` |
| New file `tasks/<KEY>.md` | Creates a new task |

`parent`, `status`, and `rev` are read-only. Deleting a task file or editing `index.md` changes nothing on the server
(they are listed as `skipped`). Changing `state` goes through the workflow just like on the web: guards and approval
gates still apply, and rejected transitions end up in `skipped`.

## Sync: `ttm sync`

```sh
ttm sync
```

1. Downloads the server's latest plan.
2. Compares `.ttm/plan` (local) with `.ttm/base` (the common base). If there are local edits, sends them up as a
   **change request** based on `synced_rev`; if a change request is already awaiting approval, it doesn't send again.
3. Brings the server's changes into `.ttm/plan` file by file: files you are editing keep their local version until the
   change request is decided; `.ttm/base` becomes the server copy; `synced_rev` is updated.

```text
Plan at rev 43 (2 file(s) updated from the server).
Local edits sent as change request 0190f4a1-…; they apply once approved on the web.
```

Change requests are approved on the web by the role the workflow specifies (`PM` by default), human approval gates
apply, and **the author can't approve their own** (`not-author`). Once applied, the project's `rev` goes up; the
next `ttm sync` pulls it down.

If the server finds that the local edits change nothing compared with the current plan:

```text
Up to date: the server found nothing to change in the local edits (copies in .ttm/cache/dropped).
```

`.ttm/plan` takes the server copy; the replaced local copies are kept in `.ttm/cache/dropped/`.

### Check in CI: `--check`

```sh
ttm sync --check
```

```text
Plan out of sync: local rev 42, server rev 43.
  local:  tasks/AUR-12.md
  server: tasks/AUR-14.md
```

`--check` writes nothing and sends nothing; it **exits 0** when in sync (`Plan in sync (rev 43).`) and **exits 1** when
out of sync. Use it in CI with a PAT that has `factory:read`:

```yaml
- run: ttm sync --check
  env:
    TTM_TOKEN: ${{ secrets.TTM_TOKEN }}
```

`--check` can't be combined with `--resolve`.

### Conflicts

When the same field of the same task changed both locally and on the server (to different values), the server returns
`409` with a list of conflicts:

```text
Conflicts with the server (both sides changed the same fields):
  AUR-12 estimate: local 5 / server 3
Run `ttm sync --resolve` to merge them with a local agent, or edit .ttm/plan and retry.
```

The command exits 1 and changes no files. You can handle it two ways:

- **By hand**: edit `.ttm/plan` to match your intent, then run `ttm sync` again.
- **`ttm sync --resolve`**: let a local agent merge them.

`--resolve` needs Claude Code (`claude`) on your `PATH`:

1. Creates a branch `ttm/resolve-<rev>` in a **separate worktree** `<worktree_root>/<KEY>/ttm-resolve-<rev>` from the
   latest default branch (your working worktree is left untouched).
2. Writes the three versions of the conflicting files to `.ttm/resolve/base`, `.ttm/resolve/local`, and
   `.ttm/resolve/server` in that worktree.
3. Runs `claude -p "<server prompt + instructions>" --allowedTools Read,Write,Edit,Glob,Grep`; the agent writes the
   merged version to `.ttm/resolve/merged`.
4. Sends the merged version as a new change request, then removes the worktree.

If the agent writes no file: `the agent did not write any merged file in …`. If the merged version still conflicts:
`the merged plan still conflicts on N field(s); run sync again`.

## Agent rules: `ttm rules apply`

```sh
ttm rules apply
```

```text
updated AGENTS.md
updated CLAUDE.md
updated .claude/settings.json
```

1. Writes a **managed block** into `AGENTS.md` and `CLAUDE.md` (creating the files if needed), between two markers:

   ```markdown
   <!-- ttm:begin -->
   …project rules compiled by the server from the workflow…
   <!-- ttm:end -->
   ```

   The rest of the file is left alone. If only one of the two markers remains, the command stops:
   `the ttm markers are broken (…); fix the file and retry`.
2. Merges **Claude Code hooks** into `.claude/settings.json` (keeping other hooks and settings, without adding
   duplicates):

   ```json
   {
     "hooks": {
       "PreToolUse": [
         { "matcher": "Bash", "hooks": [{ "type": "command", "command": "ttm hook pre-bash" }] }
       ],
       "Stop": [
         { "hooks": [{ "type": "command", "command": "ttm hook stop" }] }
       ]
     }
   }
   ```

3. Saves the rule list to `.ttm/cache/rules.json` so the hooks work offline.

Running it again when the rules haven't changed: `Rules already up to date.` Commit `AGENTS.md`, `CLAUDE.md`, and
`.claude/settings.json` so the whole team (and the runner's agents) share them. The runner runs `ttm rules apply` in
every job worktree on its own.

### What the hooks do

Claude Code calls a hook with JSON on stdin. The hook **exits 2** to block the action; the message on stderr goes back
to the agent:

```text
Blocked by project rule `no_push_main`: do not push to main directly; push a branch and open a pull request
```

| Rule | Hook | Blocks |
| --- | --- | --- |
| `no_push_main` | `pre-bash` | `git push` to a protected branch (including `HEAD:refs/heads/main`, `+main`, and a bare `git push` while on that branch) |
| `no_verify` | `pre-bash` | `git commit`/`git push` with `--no-verify`, or `git commit -n` |
| `conventional_commits` | `pre-bash` | `git commit -m` whose first line doesn't follow Conventional Commits: `type(scope)!: summary`, with `type` one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`, `merge` |
| `tests_before_done` | `stop` | The agent finishes without running the project's test command (the command must appear in the transcript) |

- The hook splits the command line on `&&`, `||`, `;`, `|`, understands quotes, and skips `git -C <dir>`, `git -c k=v`.
- Commits written in an editor (no `-m`) aren't checked.
- Protected branches: `hooks.pre_bash.protected_branches` of the rules the server sends (the project's
  `protected_branches` setting, by default its default branch), saved in `.ttm/cache/rules.json` by `ttm rules apply`.
  Without cached rules, the `default_branch` of `.ttm/project.toml`; without that, `main` and `master`. The block message
  names them (`do not push to trunk/release directly; …`).
- Outside a linked repo, the hooks allow everything. If `.ttm/cache/rules.json` is missing, the first three rules are on.
- The server turns each rule on or off from the project settings (`commit_convention = none` turns off the commit rule;
  `tests_before_done` is on when the workflow has a `tests_pass` guard or `test_commands`).
- Each result is sent up as a `rule_check` event (skipped when offline, at most 3 seconds) and shows on the Execution
  Map; inside a run it is tied to `TTM_TASK`/`TTM_RUN_ID`.

Try a hook by hand:

```sh
echo '{"tool_input":{"command":"git push origin main"},"cwd":"'"$PWD"'"}' | ttm hook pre-bash; echo "exit $?"
```

## Tasks

```sh
ttm task list                     # tasks of the linked project
ttm task list --mine --state doing
ttm task show AUR-12
ttm task claim AUR-12             # assign it to yourself
ttm task done AUR-12              # move to done (guards and approval gates still apply)
ttm task approve AUR-12           # approve the step awaiting approval
ttm task approve AUR-12 --reject
ttm task split AUR-12 -f subtasks.json
```

```text
AUR-10     done           DEV       Bình             Màn hình đăng nhập
AUR-12     doing          DEV       Dev bot          Đăng nhập bằng Google
AUR-14     inbox          QC        -                Kiểm thử đăng nhập
```

Columns: key, state (or category), role, assignee, title. `--state` matches a state id or a **category** (one of the
board's 9 standard columns: `inbox`, `refinement`, `ready`, `ux-design`, `in-progress`, `design-review`, `qc`,
`approval`, `done`); `--mine` keeps only tasks assigned to you.

`ttm task done` sends `to: "done"` (a state id or category). The workflow can reject it with `409 conflict` and a `rule`:

| `rule` | Meaning |
| --- | --- |
| `approval-required` | Someone with the right role must approve first (`ttm task approve`) |
| `tests-required`, `ci-required` | The last run has no passing test / CI result yet |
| `blocked-by-dependency` | A dependency (or subtask) isn't done |
| `wip-limit` | The target column has reached its WIP limit |
| `sprint-required` | The task must be in the active sprint |
| `budget-exceeded` | The task's token budget is used up |
| `transition-not-allowed` | The workflow doesn't allow the transition from the current state |

`ttm task claim` sends `If-Match` with the task's `rev`; if someone just edited the task, you get
`412 precondition_failed` (`rule: rev-conflict`): run it again.

### Create subtasks

`subtasks.json` (an array, or `{"tasks": [...]}`); field names in **camelCase** like the API, or in **snake_case** like the
`factory-create-subtasks` MCP tool (`acceptance_criteria`, `depends_on`): `ttm` renames them to what the server reads
before sending.

```json
[
  { "title": "API callback OAuth", "role": "DEV", "estimate": 2, "type": "feature" },
  { "title": "Kiểm thử đăng nhập", "role": "QC", "estimate": 1, "dependencies": ["#0"],
    "acceptanceCriteria": ["Đăng nhập lần đầu tạo tài khoản", "Đăng nhập lại không tạo trùng"] }
]
```

| Field | Type | Notes |
| --- | --- | --- |
| `title` | string | Required |
| `role` | string | Role; the scheduler assigns it to a member who holds that role |
| `estimate` | integer | Rolls up into the parent task |
| `type`, `priority`, `description` | string | |
| `labels` | array of strings | |
| `acceptanceCriteria` (or `acceptance_criteria`) | array of strings | |
| `dependencies` (or `depends_on`) | array | Existing task keys, or `#<n>` = the nth subtask (counting from 0) in the same file |

`kind` is read as `type`. Other keys are dropped with a warning on stderr:

```text
warning: subtask #1: unknown key `owner` ignored (known: title, type, priority, estimate, role, description, labels, acceptanceCriteria, dependencies, or acceptance_criteria, depends_on)
2 subtask(s) created under AUR-12.
```

Default limits (adjustable in the workflow): at most **2 levels** below the root task, **10** subtasks per run, **30**
under one root task; the role must be allowed to split (`PM`, `BA`, `DEV` by default). Going over: `409 conflict` with
`rule: split-limit` and `params` giving the current limits (`max_depth`, `per_run`, `per_root`, `depth`, `run_count`,
`root_count`). Subtasks inherit the parent's epic, `ref`, and sprint, and start in the `inbox` state (or `ready` if the
workflow allows it for agents). If the sprint is over capacity, they are still created but left outside the sprint,
with a `warning`.

## Agents in a task

Agents run by the runner use the `factory-*` MCP tools (get the task and its context, report progress, request
approval, create subtasks, attach PRs or test results, read the rules, claim code areas). See [Runner](runner.md) and
[MCP tools](mcp-cong-cu.md).

## Factory v2 commands (0.2)

| Command | Does |
| --- | --- |
| `ttm workflow show\|validate\|simulate\|diff\|publish\|upgrade\|copy` | Show, check, simulate, compare, publish, upgrade, copy workflows |
| `ttm automation list\|show\|test\|enable\|disable\|runs` | Automation rules: When/If/Then view, dry run on a task, history |
| `ttm agent list\|show\|create\|edit\|pause\|resume\|archive`, `agent skills add\|rm`, `agent mcp add\|rm` | Agent administration (handle, git identity, skills, MCP) |
| `ttm skill list\|push\|pull\|show\|review\|import` | Skills: deterministic packing, version push, review |
| `ttm mcp-server list\|add\|connect\|remove` | The workspace's remote MCP servers (secrets kept by the gateway) |
| `ttm cr list\|show\|approve\|reject` | Change requests of the plan and configuration |
| `ttm inbox [--watch]`, `ttm inbox read` | Notification inbox |
| `ttm template list\|publish\|install` | Template library |
| `ttm workspace members` | Members and badges |

Each command first checks the server's `discovery.features`; a server without the feature → `This server does not
support … yet` (`unsupported_feature`, exit code 2). Full syntax: [Command reference](lenh.md). Workflow, rules, agents and
skills as files in `.ttm/`: [Configuration as code](as-code.md). Output conventions: [Command line interface](giao-dien.md).
