---
title: Sign in
description: Sign in to ttm with the browser, a device code or a personal token; choose the server and profile.
order: 2
---

# Sign in

```sh
ttm login
```

ttm opens the consent page (`/ket-noi/cli`) in your browser; pick a workspace and scopes, then **Allow**. The browser
comes back to a local page (`http://127.0.0.1:<port>/callback`) that says you are signed in. The flow uses PKCE and
times out after 5 minutes.

## No browser on this machine (SSH, containers)

```sh
ttm login --device
```

ttm prints an 8-character code like `XXXX-XXXX`. Open `<server>/activate` on your phone or another computer, enter the
code and allow.

## Existing token

Create a personal access token under **Account → Connections** (`/me/ket-noi`), then:

```sh
ttm login --token ttm_pat_...
echo "$TTM_PAT" | ttm login --token -
```

## Where the token is kept

| Order | Source |
| --- | --- |
| 1 | `TTM_TOKEN` environment variable (CI, machines without a keyring) |
| 2 | The OS keyring (service `ttm`, account `<profile>@<server>`) |
| 3 | `~/.config/ttm/credentials.toml` (Windows: `%APPDATA%\ttm`), mode 0600 |

Sign-in tokens last 30 days. When one expires, ttm renews it with the refresh token if there is one; otherwise run
`ttm login` again.

## Server and profile

Server order: `--host` > `TTM_HOST` > profile > `https://toantranmei.com`.

```sh
ttm --profile dev login --host http://localhost:3000
ttm --profile dev whoami
```

Profiles live in `~/.config/ttm/config.toml`:

```toml
default_profile = "dev"

[profiles.dev]
host = "http://localhost:3000"
workspace = "<workspace id>"
```

## Sign out

```sh
ttm logout
```

Revokes the token on the server and removes it from this machine. See and revoke every token and connected app at
`/me/ket-noi`.
