---
title: Command line interface
description: Output conventions of ttm 0.2: result line, Next, status glyphs, badges, colours, plain and JSON modes, width.
order: 19
section: reference
---

Since 0.2.0 every `ttm` command prints through one shared library (`ttm-ui`), so the output looks the same everywhere.
Commands and output are always in English; your content (task titles, page names…) is shown as is.

## Every command follows one pattern

1. **The first line is the result**: `✓ Signed in as An <an@example.com>`, `! Plan out of sync…`, `2 workspace(s), current: Aurora`.
2. **Details**: tables, key–value pairs, diffs.
3. **The last line suggests a command**: `Next: ttm project init`.

Errors go to stderr in one pattern:

```text
✕ updating AUR-12: AUR-12 changed since rev 4.
  rule: rev-conflict  field: rev
  Fix: someone changed it meanwhile: fetch it again (ttm sync) and redo the change
  Help: https://toantranmei.com/help/ttm/ma-loi#rev-conflict
```

`Help:` links to the rule's entry in [Error codes](ma-loi.md).

## Status glyphs

| Glyph | `--plain` | Meaning | Categories |
| --- | --- | --- | --- |
| `○` | `todo` | Not started | `inbox`, `refinement`, `ready` |
| `●` | `doing` | In progress | `ux-design`, `in-progress` |
| `◐` | `review` | In review / waiting | `design-review`, `qc`, `approval` (yellow = waiting for a person) |
| `✓` | `done` | Done | `done` |
| `✕` | `blocked` | Failed / blocked | `blocked` |
| `⏸` | `paused` | Paused | `paused` |

Checks (`ttm doctor`, `ttm status`): `✓` ok, `!` warning, `✕` failure (`--plain`: `ok`, `warn`, `FAIL`). Callouts: `✓`
success, `!` warning, `✕` error, `›` note (`--plain`: `OK:`, `WARNING:`, `ERROR:`, `NOTE:`).

## Badges

| Badge | `--plain` | Meaning |
| --- | --- | --- |
| `🤖` | `[agent]` | Agent |
| `👤` | `[person]` | Person |
| `· Supporter` | `[Supporter]` | Supporter member |
| `✦ Mei-er` | `[Mei-er]` | Mei-er member |
| `✦✦ Sponsor` | `[Sponsor]` | Sponsor member |

Membership badges are always gold (a colour reserved for Mei-er). Agents of a workspace whose owner is a member carry the
owner's badge.

## Colours

| Token | Colour | Used for |
| --- | --- | --- |
| teal | `#5bb7a8` | Success, running |
| yellow | `#f2b84b` | Waiting, warning |
| coral | `#e97962` | Error, blocked |
| purple | `#b597d6` | Review, design |
| blue | `#55a4cf` | Information, links, suggested commands |
| gold | `#d4a72c` | Mei-er only |
| ink | bold, terminal text colour | Emphasis |
| muted | faint | Secondary text |

Truecolor terminals (`COLORTERM=truecolor|24bit`, Windows Terminal) get the exact colours; `TERM=*256color*` gets the
nearest of 256 colours; anything else gets the 16 basic colours.

## Modes

| When | Output |
| --- | --- |
| stdout is a terminal | Colours, frames, glyphs, emoji; spinners on stderr |
| Piped, `NO_COLOR` (non-empty), `TERM=dumb` | No colours, frames and glyphs kept |
| `CLICOLOR_FORCE=1` | Colours even when piped (`NO_COLOR` wins) |
| `--plain` or `TTM_PLAIN=1` | No colours, frames, glyphs or emoji: words instead of glyphs (for logs) |
| `--json` | Only JSON on stdout (0.1.6 shapes, new keys only added); errors still on stderr |

Tables never wrap: long cells are cut with `…` by display width (Vietnamese with diacritics, CJK and emoji are measured
correctly). Paths and URLs in detail lines are never cut, so they can be copied.

## Width

`TTM_COLUMNS` > `COLUMNS` > terminal size > 80, clamped to 40–200 columns.

## Wizards and `--yes`

Setup commands (`ttm project init`, `ttm agent create`, `ttm mcp-server add`…) ask one question at a time in a terminal;
**every question has a flag**. A flag that is given is not asked. `-y/--yes` takes the defaults and confirms; a required
flag that is missing fails with `missing_flag` (exit code 2). Not a terminal and no `--yes` → `missing_flag` ("pass
--workflow or run in a terminal") or `confirmation_required`. After a wizard, `ttm` prints on stderr the equivalent
command with every flag, to paste into a script:

```text
Equivalent: ttm project init --name 'Web todo' --key WEB --workflow balanced --agents create --runner this --yes
```

Secrets are never flag values: use `--secret-env VAR_NAME` (reads an environment variable) or type them hidden in a
terminal.

## Live output

Only `ttm inbox --watch` and `ttm agent logs -f` keep printing (one line per event, no full-screen UI). With `--json`
both print NDJSON (one object per line).
