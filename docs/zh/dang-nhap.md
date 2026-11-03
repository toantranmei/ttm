---
title: 登录
description: 通过浏览器、设备码或个人令牌登录 ttm;选择服务器和配置档。
order: 2
---

# 登录

```sh
ttm login
```

ttm 会在浏览器中打开授权页(`/ket-noi/cli`),选择工作区和权限后点击 **允许**。浏览器会返回本地页面
(`http://127.0.0.1:<端口>/callback`)提示已登录。流程使用 PKCE,5 分钟后超时。

## 本机没有浏览器(SSH、容器)

```sh
ttm login --device
```

ttm 会显示一个 `XXXX-XXXX` 格式的 8 位代码。在手机或其他电脑上打开 `<服务器>/activate`,输入代码并允许。

## 已有令牌

在 **账户 → 连接**(`/me/ket-noi`)创建个人访问令牌,然后:

```sh
ttm login --token ttm_pat_...
echo "$TTM_PAT" | ttm login --token -
```

## 令牌保存位置

| 顺序 | 来源 |
| --- | --- |
| 1 | 环境变量 `TTM_TOKEN`(CI、没有钥匙串的机器) |
| 2 | 操作系统钥匙串(服务 `ttm`,账户 `<配置档>@<服务器>`) |
| 3 | `~/.config/ttm/credentials.toml`(Windows:`%APPDATA%\ttm`),权限 0600 |

登录令牌有效期 30 天。过期时,若有刷新令牌 ttm 会自动续期,否则需要重新 `ttm login`。

## 服务器与配置档

服务器选择顺序:`--host` > `TTM_HOST` > 配置档 > `https://toantranmei.com`。

```sh
ttm --profile dev login --host http://localhost:3000
ttm --profile dev whoami
```

配置档位于 `~/.config/ttm/config.toml`:

```toml
default_profile = "dev"

[profiles.dev]
host = "http://localhost:3000"
workspace = "<工作区 id>"
```

## 退出登录

```sh
ttm logout
```

在服务器上吊销令牌并从本机删除。在 `/me/ket-noi` 可以查看并吊销所有令牌和已连接的应用。
