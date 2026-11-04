---
title: Signing in and tokens
description: Sign in with the browser, a device code or a personal access token; scopes, lifetimes, where tokens are stored, CI, signing out and multiple accounts.
order: 4
section: guides
---

`ttm` and `ttm-mcp` call the API with a **Bearer token** (`Authorization: Bearer ttm_…`). There are three ways to get one:

| Method | Command | Best for |
| --- | --- | --- |
| Browser on the same machine (default) | `ttm login` | A personal machine with a browser |
| Device code | `ttm login --device` | SSH, containers, servers without a browser |
| An existing personal access token | `ttm login --token ttm_pat_…` or the `TTM_TOKEN` variable | CI, scripts, tokens with limited permissions |

## Sign in with the browser

```sh
ttm login
```

1. `ttm` opens a random port on `127.0.0.1`, creates a PKCE pair (S256) and a random `state`.
2. Your browser opens the consent page `<host>/ket-noi/cli` (the URL is also printed to stderr so you can open it
   yourself if needed).
3. The page shows the app name (**ttm CLI**, "Official app by toantranmei"), the list of permissions, and a **Limit to
   workspace** field when you belong to several workspaces (leave it empty = all workspaces). Click **Allow**.
4. The browser returns to `http://127.0.0.1:<port>/callback`; `ttm` checks `state`, exchanges the code for a token, then
   calls `/me` to verify it.

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

You have **5 minutes** to finish. If you click **Deny**, the command stops with
`sign-in was refused in the browser (access_denied)`.

| Flag | What it does |
| --- | --- |
| `--no-browser` | Don't open the browser automatically, only print the URL (also enabled by the `TTM_NO_BROWSER` variable) |
| `--scope "<list>"` | Request scopes other than the default, separated by spaces |

> **Note:** the callback page only listens on `127.0.0.1` of the machine running `ttm`. When you SSH into another
> machine, the browser on your laptop can't reach it; use `ttm login --device`.

## Sign in with a device code

```sh
ttm login --device
```

```text

On any device, open https://toantranmei.com/activate and enter the code:

    BCDF-GHJK

```

Open `https://toantranmei.com/activate` on your phone or another machine, sign in on the web if needed, enter the code
(case-insensitive, with or without the hyphen), review the permissions and click **Allow**. `ttm` polls the server every
5 seconds (adding 5 seconds whenever the server answers `slow_down`) and saves the token once you allow it.

- The code is 8 consonant letters (`BCDFGHJKLMNPQRSTVWXZ`) in the form `XXXX-XXXX` and lives for **10 minutes**. When it
  expires: `the code expired before it was approved; run ttm login --device again`.
- Clicking **Deny** on the web: `device login failed: OAuth error access_denied`.
- `--device` can't be combined with `--token`.

## Personal access tokens (PAT)

Create a token under **Account → Connections** (`<host>/me/ket-noi`, **Tokens** tab): set a name, choose permissions,
choose a workspace (or *All workspaces*) and a lifetime of **7, 30, 90 or 365 days** (default 30). The `ttm_pat_…` token
is shown **only once**; copy it right away.

Save the token on your machine:

```sh
ttm login --token ttm_pat_AbCd...
printf %s "$MY_PAT" | ttm login --token -     # read from stdin, keeps it out of your shell history
```

`ttm` calls `/me` to verify it before saving:

- wrong, expired or revoked token: `the server rejected this token (… invalid_token, HTTP 401)`, nothing is saved;
- valid token without `workspace:read`: still saved, prints `Token saved for <host>.`;
- a string that doesn't start with `ttm_`: a warning, but `ttm` still tries it.

The `ttm token` commands only **open the web page** (the API doesn't let a token create or list tokens, so a leaked token
can't mint more tokens):

```sh
ttm token list                                            # opens <host>/me/ket-noi
ttm token create --name "CI" --scope notee:read,workspace:read --expires 90d
ttm token revoke                                          # opens the page to revoke
```

`create` passes the name, scopes and lifetime (`--expires` takes `1`–`365` days, written `30` or `30d`) as URL
parameters; double-check the fields on the page before you click **Create token**.

## Scopes

| Scope | Allows | Includes |
| --- | --- | --- |
| `workspace:read` | View the profile (`/me`), workspaces, members, agents, notifications | |
| `notee:read` | Read the page tree, search, read pages as Markdown, versions, suggested edits, database schemas and queries, read comments, download files, export | |
| `notee:comment` | Create, reply to, edit, delete and resolve comments; **suggest edits** to pages (`mode: suggest`) | `notee:read` |
| `notee:write` | Create, edit, move and duplicate pages; restore versions; accept/reject suggestions; create and edit databases, write and delete rows; upload files, import | `notee:read`, `notee:comment` |
| `factory:read` | View projects, tasks, plans (export), change requests, rules, run events; submit rule results (`rule_check`) | |
| `factory:write` | Create projects, edit tasks, change status, approve, submit change requests, create subtasks | `factory:read` |
| `factory:run` | Register and run runners, claim jobs, send events, finish runs, `ttm agent run`, create subtasks of the run itself | `factory:read` |
| `offline_access` | Issue a refresh token (stay signed in) | |

`ttm login` requests **all** of them by default: `workspace:read notee:read notee:write notee:comment factory:read
factory:write factory:run offline_access`. Narrow it with `--scope`, for example read-only Notee:

```sh
ttm login --scope "workspace:read notee:read"
```

Effective permission = the **intersection** of the scopes and your permissions on each resource: a token with
`notee:write` still can't edit a page you can only view. A token limited to one workspace only sees that workspace
(searches in other workspaces return nothing).

Some things can **never** be done with a token, only on the web: managing tokens, sharing (granting access, links),
billing, deleting pages, creating or editing workspaces, adding agents. Calling them with a token returns
`403 token_not_allowed`. A missing scope returns `403 insufficient_scope` with the required scope (see
[Error codes](ma-loi.md)).

## Lifetimes

| Token | Prefix | Lives |
| --- | --- | --- |
| CLI sign-in (`ttm login`, `--device`) | `ttm_oat_` | 30 days |
| CLI refresh token | `ttm_ort_` | 30 days, rotated on every use |
| Personal access token | `ttm_pat_` | 1–365 days, your choice (default 30), not renewed |
| Remote MCP apps (Claude.ai, ChatGPT…) | `ttm_oat_` | 1 hour; 30-day refresh token if the app requests `offline_access` |
| Runner | `ttm_job_` | 12 hours, the runner re-registers itself before it expires |
| A single agent run | `ttm_job_` | As long as the job |

When a saved token has expired and a refresh token is available, `ttm` and `ttm-mcp` request a new token and save it.
If it can't be renewed, the command reports `the saved token has expired; run ttm login`. A token taken from `TTM_TOKEN`
is never renewed automatically.

> **Note:** the CLI refresh token is issued at the same time and with the same 30-day lifetime as the access token, so
> in practice you'll need to run `ttm login` again roughly every 30 days.

## Where tokens are stored

When **reading**, `ttm` and `ttm-mcp` use the first source that has a token:

| Order | Source | Notes |
| --- | --- | --- |
| 1 | The `TTM_TOKEN` variable | Wins over everything, including `--profile`. Not renewed. |
| 2 | The OS keyring | Service `ttm`, account `<profile>@<host>` (for example `default@https://toantranmei.com`). macOS Keychain, Windows Credential Manager, Secret Service on Linux. |
| 3 | `credentials.toml` | `~/.config/ttm/credentials.toml` (Windows `%APPDATA%\ttm\credentials.toml`), mode 0600 |

When **writing**, `ttm` tries the keyring first (and removes any old copy from the file). If the keyring isn't available
(Linux without Secret Service, containers) or is turned off with `TTM_NO_KEYRING=1`, the token goes to
`credentials.toml`:

```toml
[credentials."default@https://toantranmei.com"]
access_token = "ttm_oat_…"
refresh_token = "ttm_ort_…"
expires_at = "2026-12-04T05:00:00Z"
scope = "workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run offline_access"
```

`ttm status` shows where the token comes from and only displays its first few characters:

```text
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
```

The runner's token is stored separately, under the account `<profile>#runner@<host>`, and is never read from
`TTM_TOKEN`.

## CI and machines without a keyring

Create a PAT with the minimum scopes, store it as a CI secret and set `TTM_TOKEN`; no `ttm login` needed:

```yaml
# GitHub Actions
- name: Plan matches the server
  env:
    TTM_TOKEN: ${{ secrets.TTM_TOKEN }}   # PAT with factory:read
  run: ttm sync --check
```

```sh
export TTM_TOKEN=ttm_pat_…
ttm whoami --json
```

While `TTM_TOKEN` is set, `ttm login` warns `TTM_TOKEN is set and will keep overriding the token saved by this login`.

## Sign out and revoke

```sh
ttm logout
```

```text
Signed out of https://toantranmei.com (profile `default`).
```

`ttm logout` revokes the access token and refresh token on the server (RFC 7009, `/api/v1/oauth/revoke`), then removes
the token from the keyring and `credentials.toml`. If the server can't confirm (no network), the token is still removed
from your machine and the command warns; in that case, also revoke it on the web. For a token taken from `TTM_TOKEN`,
you have to unset that variable yourself.

On `<host>/me/ket-noi`:

- **Tokens** tab: every unexpired token (PATs, CLI sign-ins, runner jobs), last used, a **Revoke** button;
- **Connected apps** tab: OAuth apps (Claude.ai, ChatGPT…), a **Disconnect** button that revokes the whole token family;
- **Runners** tab: machines running `ttm runner`.

Revocation takes effect immediately: every subsequent request returns `401 invalid_token`.

## Multiple accounts, multiple servers

Each **profile** has its own host, workspace and token:

```sh
ttm --profile work login                                  # profile "work" on production
ttm --profile dev login --host http://localhost:3000      # profile "dev" pointing at a local server
ttm --profile dev whoami
TTM_PROFILE=work ttm notes search "OKR"
```

`--host` (or `TTM_HOST`) at sign-in is written to the profile; the first sign-in to a profile with no host also records
the host in use. A token is tied to the (profile, host) pair: the same profile with a different `--host` is a different
token. Choose the default profile in `config.toml` with `default_profile` (see [Configuration](cau-hinh.md)).
