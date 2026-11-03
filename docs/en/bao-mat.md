---
title: Security
description: Tokens, scopes, storage and how to report a vulnerability.
order: 7
---

# Security

- **Token prefixes**: `ttm_pat_` (created by hand), `ttm_oat_` (sign-in), `ttm_ort_` (refresh), `ttm_job_` (runner).
  Never paste a token into a chat, issue or commit. If one leaks, revoke it at once at `/me/ket-noi`.
- **Scopes**: `workspace:read`, `notee:read`, `notee:write`, `notee:comment`, `factory:read`, `factory:write`,
  `factory:run`, `offline_access`. Effective access is the intersection of the scopes and your own permissions. A token
  can be limited to one workspace.
- **Lifetime**: 30 days by default. Refresh tokens rotate; reusing an old refresh token revokes the whole family.
- **Storage**: the OS keyring, else `credentials.toml` with mode 0600. ttm never logs tokens; `ttm status` shows only the
  first characters.
- **Sign-in**: PKCE S256 and a random `state`; the callback page listens on `127.0.0.1` only.
- **Updates**: `ttm self-update` checks SHA-256, and the GitHub attestation when `gh` is installed.
- **Open source** (MIT): orchestration logic runs on the server; `ttm` only authenticates, calls the API and runs work on
  your machine.

## Reporting a vulnerability

E-mail `toantran.mei@gmail.com` privately (do not open a public issue). We answer within 72 hours.
