---
title: Troubleshooting and FAQ
description: How to diagnose problems, common errors by symptom when installing, signing in, using MCP, Notee, Factory and the runner, and frequently asked questions.
order: 16
section: more
---

## Diagnose first

```sh
ttm doctor            # host, discovery, version, token, keyring, git, claude, ttm-mcp
ttm status --json     # host, profile, token source, current workspace
ttm -vv whoami        # debug log of the requests (token masked)
```

When you report a bug on GitHub, attach the output of `ttm doctor`, `ttm --version`, your operating system and the
install channel.

## Installation

**`brew install` says Xcode is too old.**
Homebrew refuses when Xcode is outdated. Update Xcode from the App Store, or remove Xcode and use the Command Line Tools:
`sudo rm -rf /Applications/Xcode.app && xcode-select --install`.

**After installing with npm, `ttm` says it can't find the binary.**
npm skipped the `postinstall` script (npm 11 warns about scripts in globally installed packages). Reinstall and allow
exactly these two packages:
`npm install -g --allow-scripts=@toantranmei/cli,@toantranmei/mcp @toantranmei/cli @toantranmei/mcp`.

**`command not found: ttm` after running the install script.**
Open a new terminal, or run `source ~/.cargo/env`. Check that `~/.cargo/bin` is in your `PATH`.

**Windows on ARM.**
There is no ARM build; install the x64 build (PowerShell, MSI or npm), and Windows runs it under emulation.

**macOS blocks a binary downloaded with the browser.**
`xattr -d com.apple.quarantine ./ttm ./ttm-mcp`, or install with the script/Homebrew.

**Windows SmartScreen warns about the MSI.**
The binaries aren't code-signed. Check the SHA-256 and attestation ([Installation](cai-dat.md)), then choose
**More info → Run anyway**.

**`ttm self-update` reports `cannot write into …`.**
You don't have write access to the install directory (usually because you installed with Homebrew, npm or MSI). Update
with the matching package manager.

**`self-update` reports `GitHub answered 403`.**
GitHub's anonymous API rate limit. Try again later, or reinstall with the script.

## Signing in

**`ttm login` opens the browser but never finishes.**
You're SSH'd into another machine (the callback page is on that machine's `127.0.0.1`) or a firewall blocks the local
port. Use `ttm login --device`.

**`timed out after 5 minutes waiting for the browser`.**
Run it again and click **Allow** within 5 minutes. Use `--no-browser` to open the URL yourself.

**`the code expired before it was approved`.**
The device code lives for 10 minutes. Run `ttm login --device` again.

**`` not signed in to …; run `ttm login` `` even though you signed in.**
A token is tied to (profile, host). You may be using a different profile or host: check `ttm status`. `TTM_HOST`,
`TTM_PROFILE` and `--host` all change that pair.

**`invalid_token` after a while.**
The token expired (for `ttm login`: `ttm` went unused for more than 90 days, so the refresh token expired) or was
revoked. Run `ttm login` again. If `ttm status` reports the
source as `TTM_TOKEN`, that variable holds an old token: unset or replace it.

**`insufficient_scope … [requires scope X]` / `{"required": "X"}`.**
The token lacks scope X. PAT: create a new token with X. Narrow `ttm login --scope`: run `ttm login` again (it requests
every scope by default).

**`keyring unavailable …; saving the token to credentials.toml`.**
The machine has no keyring (Linux without Secret Service, containers). The token is still saved in a 0600 file. Set
`TTM_NO_KEYRING=1` to silence the warning.

**`ttm doctor` reports `token … valid but lacks workspace:read`.**
The PAT doesn't have `workspace:read`: Notee/Factory commands still work, but `whoami` and `workspace list` don't.

## MCP

**The app doesn't see the `ttm` server.**
Fully restart the app after `ttm mcp install`. Check that `command` in the config is an absolute path that exists
(graphical apps don't see your shell's `PATH`). Claude Code: `claude mcp list`.

**Every tool returns `unauthorized`.**
`ttm-mcp` can't find a token. Run `ttm login` with the same profile/host the MCP config uses (by default
`default_profile`); if the config has `--profile`/`--host`, sign in with exactly those values. On a machine without a
keyring, add `TTM_TOKEN` to the `env` of the MCP config.

**`ttm mcp install` reports `the existing config is not valid JSON; fix it first`.**
The app's config file is broken; ttm wrote nothing. Fix the file (an older backup is at `<file>.bak-<number>` if you
installed before), then run it again.

**``Claude Code (`claude`) is not on PATH``.**
Install Claude Code, or run the command printed with the message by hand (`claude mcp add --scope user ttm -- …`).

**`factory-*` tools return `no_run`.**
Those tools are for an agent inside a runner's run. Outside a run, only `factory-get-task`, `factory-get-context` (pass
`task`) and `factory-get-rules` (pass `project`) work.

**The model picks the wrong tool or uses a lot of context.**
Reduce the number of tools: `ttm-mcp --toolsets notee` (or `ttm mcp install --toolsets notee`).

**Connecting remote MCP in Claude.ai fails.**
Check that the URL is exactly `https://toantranmei.com/api/v1/mcp`. Sign in on the web first, then click **Connect**
again. Denied by mistake: connect again. Remove the old connection at `/me/ket-noi` → **Connected apps**.

## Notee

**`ttm notes edit` prints `Page updated.` but nothing changes as expected, with `warning: … old_str not found; skipped`.**
`--old` must match the Markdown the server returns exactly, case-sensitive, including escapes (`\*`, `\_`) and tab
indentation. Read it again with `ttm notes get` and copy that exact passage.

**`old_str matches N places`.**
Add surrounding text so it matches only one place, or use `--replace-all`.

**An edit turns into a suggestion (`Suggestion … created`) even without `--suggest`.**
The workspace sets `agent_write_mode = suggest`, or the token only has `notee:comment`. Someone with edit access reviews
it with `ttm notes resolve`.

**`would_delete_children` with `--replace`.**
The new content is missing the `<page>`/`<database>` tags of child pages. Keep those tags, or add
`--allow-deleting-content` if you really want to delete them.

**`too_many_blocks`.**
The write changes more than 1,000 blocks. Split it into several smaller `--old/--new` edits.

**`rate_limited` when an agent edits continuously.**
Each token gets 60 Notee writes per hour. Batch changes into fewer writes.

**`ambiguous_anchor` when commenting.**
The quote appears more than once; add `--prefix`/`--suffix` (MCP: `prefix`/`suffix`).

**SQL reports `sql_table: Table 'Rows' is not allowed`.**
The table name is lowercase `rows`, unquoted. Columns need double quotes: `"Trạng thái"`.

**`query_timeout`.**
The query took longer than 3 seconds: add `WHERE`, `LIMIT`, drop heavy functions.

**`ttm notes get` on a database row doesn't show column values.**
Column values are only in `--json` (`properties`).

## Factory

**`this directory is not linked to a project`.**
Run `ttm project link <KEY>` at the repo root, then commit `.ttm/project.toml` and `.ttm/plan`.

**`ttm sync --check` exits 1 in CI.**
The plan in the repo has drifted from the server. Run `ttm sync` on your dev machine and commit `.ttm/plan`, or review the
pending change request.

**`Conflicts with the server`.**
The same field changed on both sides. Fix `.ttm/plan` and run `ttm sync` again, or `ttm sync --resolve` (needs `claude`).

**Local edits don't reach the server.**
A change request is awaiting review (`ttm project status` shows its id); `ttm sync` doesn't send again until it's
decided. Review it on the web (the author can't approve their own).

**`the ttm markers are broken`.**
`AGENTS.md` or `CLAUDE.md` has only one of the two markers `<!-- ttm:begin -->` / `<!-- ttm:end -->`. Fix it by hand,
then run `ttm rules apply`.

**A hook blocks the commit: `commit message must follow Conventional Commits`.**
Use the form `type(scope): summary`, for example `fix(auth): retry on 503`. A project can turn this rule off on the web.

**`ttm task done` reports `conflict` with `approval-required`, `tests-required`…**
The workflow blocks the status change. See the table in [Error codes](ma-loi.md).

## Runner

**``no agent harness found; install Claude Code (`claude`) first``.**
Install Claude Code. If it still reports this when running as a service: the service uses the `PATH` recorded at install
time (versions before 0.1.4 recorded none). Run `ttm runner install-service` again from a shell where `which claude`
finds it (see [Runner](runner.md)).

**A job is refused: `refused AUR-12: no local clone is configured for project AUR`.**
Run `ttm runner config --repo AUR=~/code/aurora`, then restart the runner.

**`the job's repository … is not the origin of …`.**
The clone's `origin` isn't the project's repo. Fix the remote (`git remote set-url origin …`) or point `--repo` at the
right clone.

**Job refused: `` refused AUR-12: the job's worktree_hint `…` is an absolute path; worktrees stay under the runner's worktree_root … ``.**
The server sent a `worktree_hint` that isn't under `worktree_root` (absolute, with `..`, or through a symlink leading
out). The runner refuses it (`invalid_worktree`) and creates nothing; report it to the server's administrator.

**The runner starts again after `ttm runner down`.**
A service (launchd `KeepAlive`, systemd, Task Scheduler) keeps it alive. Stop it for good with `ttm runner uninstall-service`.

**A run ends `paused`.**
Claude reported that the usage limit was reached; the runner stops taking jobs until the reset time, and the server
requeues the work.

**No pull request after a successful run.**
You need `gh` signed in (`gh auth status`) and a repo with a remote. The branch is still pushed; open the PR by hand.

**Out of disk space because of worktrees.**
Lower `--keep-days`, or delete folders in `~/.ttm/worktrees` and run `git worktree prune` in the repo.

## FAQ

**Is ttm free and open source?**
Yes, MIT, at <https://github.com/toantranmei/ttm>. Usage limits (number of agents, concurrency…) follow your workspace's
plan on toantranmei.com.

**What's the difference between `ttm-mcp` and remote MCP?**
Same tools. `ttm-mcp` runs on your machine with the token from `ttm login`, which suits local apps and the runner. Remote
MCP needs no install and uses OAuth in Claude.ai and ChatGPT.

**Whose Claude token does the agent use?**
Yours: the runner calls the Claude Code signed in on your machine. The server never receives a Claude key, and agents
running on your machine don't use toantranmei credits.

**Can I use it with a self-hosted server or localhost?**
Yes: `--host http://localhost:3000` or a separate profile. ttm reads every address from discovery at `/.well-known/ttm`.

**Multiple accounts on one machine?**
One profile per account: `ttm --profile work login`, `ttm --profile personal login`. Choose the default profile with
`default_profile`.

**How do I use it in CI?**
A minimal PAT + `TTM_TOKEN`, for example `ttm sync --check` to block PRs that make the plan drift. See
[Signing in and tokens](dang-nhap.md).

**Is it on Cargo/crates.io?**
Not published on crates.io yet; install from source with `cargo install --git https://github.com/toantranmei/ttm ttm-cli`.

**Is there an `.mcpb` bundle for Claude Desktop?**
Yes, since 0.1.4: download `ttm-mcp-<os>.mcpb` from GitHub Releases (see [MCP](mcp.md)). With older releases, use
`ttm mcp install --client claude-desktop` or remote MCP.

**Is my data used to train models?**
ttm doesn't send data to third parties. Content you give to Claude, ChatGPT… is subject to that provider's policy.
