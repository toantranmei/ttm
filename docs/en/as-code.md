---
title: Configuration as code
description: The .ttm directory with the workflow, Automation rules, agents and skills in the repository; local checks, change requests by kind, conflicts and ttm cr.
order: 20
section: guides
---

Since 0.2.0, besides the plan (`plan/`), `ttm sync` also syncs the project's **configuration** as files in the
repository: workflow, Automation rules, agents and skills. The web stays the source of truth: edits in the repository
become **change requests** (CR) waiting for approval, never overwrites. This only happens when the server lists
`ascode_v2` in `discovery.features`; older servers keep syncing only the plan as in 0.1.

## The `.ttm/` layout

```text
.ttm/
  project.toml   committed: the linked project, synced rev
  plan/          committed: the plan as Markdown
  workflow.yaml  committed: the project's workflow
  rules/         committed: Automation rules, one <slug>.yaml each
  agents/        committed: agents, one <handle>.yaml each (never secrets)
  skills/        committed: skills, one <slug>/ directory with a SKILL.md each
  base/          ignored: the plan as of the last sync (common ancestor for 3-way comparisons)
  base-config/   ignored: the configuration as of the last sync
  cache/         ignored: rules for hooks, replaced copies, config_base.json
```

`ttm sync` downloads the bundle with `?include=config,skills`, writes the directories above and keeps `config_base` (the
server's configuration version) to send back with the CR.

## Local checks before sending

Every **changed** configuration file is checked first; on any problem `ttm sync` stops, creates no CR and exits 1:

| Check | Error |
| --- | --- |
| `workflow.yaml`, `rules/*.yaml`, `agents/*.yaml` must be valid YAML | `✕ yaml <file>:<line> …` |
| `agents/*.yaml` has no secret-like key at any depth: `token`, `*_token`, `*secret*`, `*password*`, `api_key`, `*_api_key`, `apikey` | `secret_in_file` |
| `skills/<slug>/` passes the same checks as `ttm skill push`: a `SKILL.md` with `name`/`description` front matter, ≤ 200 files, ≤ 5 MB, safe paths | `skill_md_missing`, `front_matter`, `path_unsafe`, `too_large`, `too_many_files` |

Secrets (MCP tokens, API keys) never live in the repository: they are kept by the server's MCP gateway (see
[Security](bao-mat.md)) or in the runner's environment (`env:VAR` in `runner.toml`).

## Change requests by kind

The server creates one CR for the plan and one **configuration request** per configuration object that changed:

```text
✓ 2 configuration request(s) created; they apply once approved
KIND      TARGET  ID
────────────────────────
agent     dev     cfg_8f2…
workflow  Aurora  cfg_91a…
Next: ttm cr show cfg_8f2…
```

| Kind | Object | Approved where |
| --- | --- | --- |
| `plan` | Tasks, plan | Web or `ttm cr approve` |
| `workflow` | `workflow.yaml` | Same; applying it makes a new workflow draft |
| `rule` | `rules/<slug>.yaml` | Same |
| `agent` | `agents/<handle>.yaml` | Same (owner/admin) |
| `skill` | `skills/<slug>/` | Approving a skill CR approves that skill version |

Commands: `ttm cr list [--kind … --status …]`, `ttm cr show <id>` (a diff per file), `ttm cr approve <id>`, `ttm cr reject
<id> --note "…"`. See [Command reference](lenh.md).

## Conflicts

The same field changed both in the repository and on the web → `409` with `{kind, target, path, base, local, server}`;
`ttm sync` prints the conflicts, sends nothing and exits 1. Resolve them like plan conflicts: edit the files and run `ttm
sync` again, or `ttm sync --resolve` to let a local agent merge the three versions (YAML files use the same 3-way merge).

## AGENTS.md "rules" are not `.ttm/rules/`

| | `ttm rules apply` | `.ttm/rules/` |
| --- | --- | --- |
| What | Rules for **local agents** (block in `AGENTS.md`/`CLAUDE.md`, hooks `ttm hook pre-bash`/`stop`) | The project's **Automation** rules ("when … if … then …") run by the server |
| Who edits | The server generates them, `ttm rules apply` writes them | You edit the files, `ttm sync` sends them as `rule` CRs |
| See | `ttm rules apply` | `ttm automation list`, `show`, `test` |
