---
title: 登录与 token
description: 通过浏览器、设备码或个人 token 登录；scope、有效期、token 的存储位置、CI、退出登录以及多账号。
order: 4
section: guides
---

`ttm` 和 `ttm-mcp` 使用 **Bearer token**（`Authorization: Bearer ttm_…`）调用 API。获取 token 有三种方式：

| 方式 | 命令 | 适用场景 |
| --- | --- | --- |
| 同一台机器上的浏览器（默认） | `ttm login` | 有浏览器的个人电脑 |
| 设备码 | `ttm login --device` | SSH、容器、没有浏览器的服务器 |
| 已有的个人 token | `ttm login --token ttm_pat_…` 或环境变量 `TTM_TOKEN` | CI、脚本、限制权限的 token |

## 通过浏览器登录

```sh
ttm login
```

1. `ttm` 在 `127.0.0.1` 上打开一个随机端口，生成一对 PKCE（S256）和一个随机的 `state`。
2. 浏览器打开授权页面 `<host>/ket-noi/cli`（URL 也会打印到 stderr，必要时可以手动打开）。
3. 页面显示应用名称（**ttm CLI**，“toantranmei 官方应用”）、权限列表；如果你属于多个工作区，还会显示 **限定工作区** 选项（留空 =
   所有工作区）。点击 **允许**。
4. 浏览器跳回 `http://127.0.0.1:<端口>/callback`；`ttm` 校验 `state`，用授权码换取 token，然后调用 `/me` 进行验证。

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

你有 **5 分钟** 完成操作。点击 **拒绝** 后命令会以 `sign-in was refused in the browser (access_denied)` 结束。

| 标志 | 作用 |
| --- | --- |
| `--no-browser` | 不自动打开浏览器，只打印 URL（也可通过环境变量 `TTM_NO_BROWSER` 开启） |
| `--scope "<列表>"` | 申请非默认的 scope，以空格分隔 |

> **注意：** 回调页面只监听运行 `ttm` 的那台机器的 `127.0.0.1`。通过 SSH 登录到另一台机器时，笔记本上的浏览器无法访问它；请使用
> `ttm login --device`。

## 通过设备码登录

```sh
ttm login --device
```

```text

On any device, open https://toantranmei.com/activate and enter the code:

    BCDF-GHJK

```

在手机或另一台电脑上打开 `https://toantranmei.com/activate`，必要时先登录网页，输入代码（不区分大小写，带不带连字符都可以），查看权限后点击
**允许**。`ttm` 每 5 秒轮询一次服务器（如果服务器返回 `slow_down`，间隔再增加 5 秒），在你允许后保存 token。

- 代码由 8 个辅音字母（`BCDFGHJKLMNPQRSTVWXZ`）组成，格式为 `XXXX-XXXX`，有效期 **10 分钟**。过期时：
  `the code expired before it was approved; run ttm login --device again`。
- 在网页上点击 **拒绝**：`device login failed: OAuth error access_denied`。
- `--device` 不能与 `--token` 同时使用。

## 个人 token（PAT）

在 **账户 → 连接**（`<host>/me/ket-noi`，**Token** 标签页）创建 token：填写名称，选择权限，选择工作区（或 *所有工作区*）以及有效期
**7、30、90 或 365 天**（默认 30）。`ttm_pat_…` token 只显示**一次**，请立即复制。

把 token 保存到本机：

```sh
ttm login --token ttm_pat_AbCd...
printf %s "$MY_PAT" | ttm login --token -     # 从 stdin 读取，不会留在 shell 历史中
```

`ttm` 在保存前会调用 `/me` 进行验证：

- token 错误、已过期或已被撤销：`the server rejected this token (… invalid_token, HTTP 401)`，不保存任何内容；
- token 有效但缺少 `workspace:read`：仍然保存，并打印 `Token saved for <host>.`；
- 字符串不以 `ttm_` 开头：发出警告，但仍会尝试。

`ttm token` 系列命令只会**打开网页**（API 不允许用 token 创建或列出 token，以免泄露的 token 自行生成更多 token）：

```sh
ttm token list                                            # 打开 <host>/me/ket-noi
ttm token create --name "CI" --scope notee:read,workspace:read --expires 90d
ttm token revoke                                          # 打开页面以撤销
```

`create` 会把名称、scope 和有效期（`--expires` 接受 `1`–`365` 天，写作 `30` 或 `30d`）作为 URL 参数带上；点击 **创建 token**
之前请再检查一遍页面上的各项内容。

## Scope

| Scope | 允许 | 包含 |
| --- | --- | --- |
| `workspace:read` | 查看个人资料（`/me`）、工作区、成员、agent、通知 | |
| `notee:read` | 读取页面树、搜索、以 Markdown 读取页面、版本、修改建议、数据库 schema 与查询、读取评论、下载文件、导出 | |
| `notee:comment` | 创建、回复、编辑、删除、关闭评论；对页面提出**修改建议**（`mode: suggest`） | `notee:read` |
| `notee:write` | 创建、编辑、移动、复制页面；恢复版本；接受/拒绝修改建议；创建和编辑数据库，写入和删除行；上传文件、导入 | `notee:read`、`notee:comment` |
| `factory:read` | 查看项目、任务、计划（导出）、变更请求（change request）、规则、run 的事件；提交规则结果（`rule_check`） | |
| `factory:write` | 创建项目、编辑任务、变更状态、审批、提交变更请求、创建子任务 | `factory:read` |
| `factory:run` | 注册并运行 runner、领取作业（job）、发送事件、结束 run、`ttm agent run`、为当前 run 创建子任务 | `factory:read` |
| `offline_access` | 签发 refresh token（保持登录） | |

`ttm login` 默认申请**全部** scope：`workspace:read notee:read notee:write notee:comment factory:read factory:write
factory:run offline_access`。可以用 `--scope` 缩小范围，例如只读 Notee：

```sh
ttm login --scope "workspace:read notee:read"
```

实际权限 = scope 与你对每个资源所拥有权限的**交集**：即使 token 有 `notee:write`，也无法编辑你只有查看权限的页面。限定在某个工作区的
token 只能看到该工作区（在其他工作区搜索会返回空结果）。

有些操作**永远**不能用 token 完成，只能在网页上操作：管理 token、共享（授予权限、链接）、付费、删除页面、创建或修改工作区、添加
agent。用 token 调用这些操作会返回 `403 token_not_allowed`。缺少 scope 会返回 `403 insufficient_scope`，并附带所需的 scope（见
[错误码](ma-loi.md)）。

## 有效期

| Token | 前缀 | 有效期 |
| --- | --- | --- |
| CLI 登录（`ttm login`、`--device`） | `ttm_oat_` | 7 天，自动续期 |
| CLI 的 refresh token | `ttm_ort_` | 90 天，滚动：每次续期都会签发新的 refresh token，重新计 90 天 |
| 个人 token | `ttm_pat_` | 1–365 天，由你选择（默认 30），不续期 |
| 远程 MCP 应用（Claude.ai、ChatGPT 等） | `ttm_oat_` | 1 小时；仅当应用申请了 `offline_access` 时才有 90 天的滚动 refresh token |
| Runner | `ttm_job_` | 12 小时，runner 会在过期前自动重新注册 |
| 单次 agent 运行 | `ttm_job_` | 随作业的时长而定 |

当已保存的 token 过期且有 refresh token 时，`ttm` 和 `ttm-mcp` 会自动申请新 token 并保存。无法续期时，命令会报告
`the saved token has expired; run ttm login`。来自 `TTM_TOKEN` 的 token 永远不会自动续期。

> **注意：** CLI 的访问 token 有效期 7 天，会自动续期；只要你至少每 90 天使用一次 `ttm`（或 `ttm-mcp`），就会保持登录。
> 超过 90 天未使用，refresh token 过期，需要重新 `ttm login`。（这些有效期随即将发布的服务器更新生效；在此之前两种 token
> 都是 30 天。）

## Token 存储在哪里

**读取**时，`ttm` 和 `ttm-mcp` 使用第一个含有 token 的来源：

| 顺序 | 来源 | 说明 |
| --- | --- | --- |
| 1 | 环境变量 `TTM_TOKEN` | 优先于一切，包括 `--profile`。不续期。 |
| 2 | 操作系统 keyring | 服务名 `ttm`，账户 `<profile>@<host>`（例如 `default@https://toantranmei.com`）。macOS Keychain、Windows Credential Manager、Linux 上的 Secret Service。 |
| 3 | `credentials.toml` | `~/.config/ttm/credentials.toml`（Windows `%APPDATA%\ttm\credentials.toml`），权限 0600 |

**写入**时，`ttm` 优先尝试 keyring（如果文件中有旧值则一并删除）。keyring 不可用（Linux 没有 Secret Service、容器）或通过
`TTM_NO_KEYRING=1` 关闭时，token 会写入 `credentials.toml`：

```toml
[credentials."default@https://toantranmei.com"]
access_token = "ttm_oat_…"
refresh_token = "ttm_ort_…"
expires_at = "2026-12-04T05:00:00Z"
scope = "workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run offline_access"
```

`ttm status` 会显示 token 的来源，并且只显示开头几个字符：

```text
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
```

runner 的 token 单独保存，账户为 `<profile>#runner@<host>`，并且永远不会从 `TTM_TOKEN` 读取。

## CI 与没有 keyring 的机器

创建 scope 最小的 PAT，保存到 CI 的 secret 中，然后设置 `TTM_TOKEN`；无需 `ttm login`：

```yaml
# GitHub Actions
- name: 计划与服务器一致
  env:
    TTM_TOKEN: ${{ secrets.TTM_TOKEN }}   # 具有 factory:read 的 PAT
  run: ttm sync --check
```

```sh
export TTM_TOKEN=ttm_pat_…
ttm whoami --json
```

设置了 `TTM_TOKEN` 时，`ttm login` 会警告 `TTM_TOKEN is set and will keep overriding the token saved by this login`。

## 退出登录与撤销

```sh
ttm logout
```

```text
Signed out of https://toantranmei.com (profile `default`).
```

`ttm logout` 会在服务器上撤销访问 token 和 refresh token（RFC 7009，`/api/v1/oauth/revoke`），然后把 token 从 keyring 和
`credentials.toml` 中删除。如果服务器无法确认（断网），token 仍会从本机删除，命令会发出警告；这时请再到网页上撤销。来自 `TTM_TOKEN`
的 token 需要你自己取消该环境变量。

在 `<host>/me/ket-noi` 上：

- **Token** 标签页：所有未过期的 token（PAT、CLI 登录、runner 作业）、最后使用时间、**撤销** 按钮；
- **已连接的应用** 标签页：OAuth 应用（Claude.ai、ChatGPT 等），**断开连接** 按钮会撤销该应用的整组 token；
- **Runner** 标签页：正在运行 `ttm runner` 的机器。

撤销立即生效：之后的所有请求都会返回 `401 invalid_token`。

## 多账号、多服务器

每个 **profile** 都有自己的 host、工作区和 token：

```sh
ttm --profile work login                                  # production 上的 "work" profile
ttm --profile dev login --host http://localhost:3000      # 指向本地服务器的 "dev" profile
ttm --profile dev whoami
TTM_PROFILE=work ttm notes search "OKR"
```

登录时的 `--host`（或 `TTM_HOST`）会写入 profile；首次登录一个尚未设置 host 的 profile 时，也会写入当前使用的 host。token 绑定到
（profile, host）这一对：同一个 profile 换了 `--host` 就是另一个 token。在 `config.toml` 中用 `default_profile` 选择默认
profile（见 [配置](cau-hinh.md)）。
