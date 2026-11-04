---
title: Security
description: How tokens are stored and protected, minimal scopes, revocation, what the server does and does not see, runner isolation and how to report a vulnerability.
order: 15
section: more
---

## Tokens

- **Clear prefixes**: `ttm_pat_` (personal access token), `ttm_oat_` (sign-in token, OAuth app), `ttm_ort_` (refresh),
  `ttm_job_` (runner and agent run). Never paste a token into a chat, issue, commit or CI log. If a token leaks, revoke it
  at once at `<host>/me/ket-noi`.
- **The server stores only a hash** (SHA-256) of each token; the raw token is shown only once, when it is created.
- **Local storage**: the operating system keyring (macOS Keychain, Windows Credential Manager, Secret Service), service
  `ttm`. Without a keyring, `credentials.toml` with mode **0600**, written through a temporary file whose permissions are
  set before it is renamed, so there is never a moment when other users can read it.
- **Never logged**: everywhere a token is displayed, only the prefix and at most 4 characters are kept
  (`ttm_pat_Ab12…`), even with `-vv`.
- **`TTM_TOKEN`** wins over every other source: handy for CI, but child processes can read environment variables; set it
  only in the step that needs it.
- Tokens are sent in the `Authorization: Bearer` header. Requests with a token **do not use cookies**, so they are not
  subject to CSRF.

## Least privilege

- Pick just the scopes you need: a script that only reads Notee needs `workspace:read notee:read`; CI that checks the plan
  needs `factory:read`.
- **Restrict a token to one workspace** when you create a PAT, or with the *Limit to workspace* field when you approve a
  sign-in.
- To let an agent only **suggest** edits instead of writing directly: grant `notee:comment` without `notee:write`, or set
  `agent_write_mode = suggest` for the workspace.
- Effective permissions are always the intersection of the token's scopes and your own permissions on the resource: a
  token can never do more than you can.
- Sensitive actions **cannot be done with a token**, whatever the scope: creating/listing tokens, sharing, billing,
  deleting pages, creating workspaces, administration (`token_not_allowed`).

## Lifetime and revocation

- Tokens last 30 days by default (PAT 1–365 days, OAuth apps 1 hour, runner 12 hours, job tokens as long as the job).
- Refresh tokens **rotate** on every use; reusing an old refresh token (a sign of theft) revokes the whole token family.
- `ttm logout` revokes the token on the server before deleting it locally.
- The `/me/ket-noi` page shows when each token was last used; each connected app has a **Disconnect** button that revokes
  every token of that app; the **Runner** tab turns runners off remotely.

## Signing in

- The browser flow uses **PKCE S256** and a random `state`; the callback page listens only on `127.0.0.1`, ignores any
  request with the wrong `state`, and lives only until it receives the code (at most 5 minutes).
- Device flow: the 8-character code lasts 10 minutes and is single-use; the `/activate` page shows you the app name and
  permissions before you approve.
- The consent page for external apps (remote MCP) is clearly marked *External app*, shows where the browser will return
  to, and **warns when the app name resembles an official app**. Click **Allow** only if you yourself just connected that
  app.
- OAuth tokens are bound to the exact resource `https://toantranmei.com/api/v1/mcp`; the server refuses to issue tokens
  for any other resource.

## What the server sees

| The server sees | The server does not see |
| --- | --- |
| Every API request you (or your agent) send: pages read, edits, queries, comments | Your API key or Claude session |
| When each token was last used (time, IP, User-Agent) | Files on your machine that the agent does not read or upload |
| Run events: reasoning, actions, commands and file paths the agent uses, test results, token counts | The contents of `config.toml`, the keyring, `credentials.toml` |
| Rule results (`rule_check`) from hooks | Your repo, except the branches the agent pushes to your remote |
| The runner's machine name, operating system, version and harness | |

The prompt for each step is assembled by the server and sent down to your machine to run Claude Code; the
prompt-building logic lives on the server. Claude Code talks to Anthropic directly using your account, not through
toantranmei.com.

## Runner

- **Outbound connections only** (heartbeat and long-poll); no ports are opened.
- Works only in the **clone you declare** (`ttm runner config --repo`), and checks that the clone's `origin` matches the
  job's repo; it never clones a URL sent by the server. Branch names are validated before use.
- **One worktree and one branch per task**; your own worktree is never touched. The `pre-bash` hook blocks pushing
  straight to `main`/`master`, `--no-verify` and commits that break the conventions.
- A job token works only for its own run, lives in an MCP config file with mode 0600, and is deleted when the run ends.
- The agent runs with **your operating system user's permissions**, limited by the tool list the job allows. This is not
  a sandbox: run runners only for projects you trust, review pull requests before merging, and consider running the
  runner under a separate OS account or in a virtual machine.
- Kill switch: the stop button on the web or `ttm agent stop <run>`; the runner picks up the command within ≤ 20 seconds.

## Safe updates

`ttm self-update` refuses to install a release without checksums, verifies the SHA-256 of each archive, and verifies the
**GitHub artifact attestation** (Sigstore) when `gh` is available; a bad attestation stops the update. To verify by hand:
[Installation](cai-dat.md).

The binaries are not yet code-signed by Apple or Microsoft; the script, Homebrew and npm channels do not set the
quarantine flag. The source code is open (MIT) at <https://github.com/toantranmei/ttm>.

## Reporting a vulnerability

Send a private email to **toantran.mei@gmail.com** with a description, steps to reproduce and the version
(`ttm --version`). **Do not open a public issue** for a vulnerability. We respond within 72 hours, keep you posted on the
fix and credit you in the release notes if you wish.
