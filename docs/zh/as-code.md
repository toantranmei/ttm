---
title: 配置即代码
description: 仓库中的 .ttm 目录：工作流、自动化规则、agent 和 skill；本地检查、按类型的变更请求、冲突与 ttm cr。
order: 20
section: guides
---

从 0.2.0 起，除了计划（`plan/`）之外，`ttm sync` 还会把项目的**配置**以文件形式同步到仓库：工作流、自动化规则、agent 和
skill。网页仍是唯一来源：在仓库中的修改只会成为待审批的**变更请求**（CR），绝不会直接覆盖。仅当服务器在 `discovery.features`
中声明 `ascode_v2` 时才启用；旧服务器仍像 0.1 一样只同步计划。

## `.ttm/` 结构

```text
.ttm/
  project.toml   提交：关联的项目、已同步的 rev
  plan/          提交：Markdown 格式的计划
  workflow.yaml  提交：项目的工作流
  rules/         提交：自动化规则，每条一个 <slug>.yaml
  agents/        提交：agent，每个一个 <handle>.yaml（绝不含密钥）
  skills/        提交：skill，每个一个含 SKILL.md 的 <slug>/ 目录
  base/          忽略：上次同步时的计划（三方比较的共同祖先）
  base-config/   忽略：上次同步时的配置
  cache/         忽略：钩子用的规则、被替换的副本、config_base.json
```

`ttm sync` 以 `?include=config,skills` 下载包，写入上述目录，并保存 `config_base`（服务器的配置版本），随 CR 一起发回。

## 发送前的本地检查

每个**被修改**的配置文件都会先检查；有问题时 `ttm sync` 停止，不创建 CR，退出码 1：

| 检查 | 错误 |
| --- | --- |
| `workflow.yaml`、`rules/*.yaml`、`agents/*.yaml` 必须是有效的 YAML | `✕ yaml <文件>:<行> …` |
| `agents/*.yaml` 任何层级都不能有类似密钥的键：`token`、`*_token`、`*secret*`、`*password*`、`api_key`、`*_api_key`、`apikey` | `secret_in_file` |
| `skills/<slug>/` 通过与 `ttm skill push` 相同的检查：有带 `name`/`description` front matter 的 `SKILL.md`，≤ 200 个文件，≤ 5 MB，路径安全 | `skill_md_missing`、`front_matter`、`path_unsafe`、`too_large`、`too_many_files` |

密钥（MCP 令牌、API key）永远不放在仓库中：它们由服务器的 MCP 网关保存（见[安全](bao-mat.md)），或在 runner 的环境变量中
（`runner.toml` 中的 `env:VAR`）。

## 按类型的变更请求

服务器为计划创建一个 CR，并为每个被修改的配置对象创建一个**配置请求**：

```text
✓ 2 configuration request(s) created; they apply once approved
KIND      TARGET  ID
────────────────────────
agent     dev     cfg_8f2…
workflow  Aurora  cfg_91a…
Next: ttm cr show cfg_8f2…
```

| Kind | 对象 | 在哪里审批 |
| --- | --- | --- |
| `plan` | 任务、计划 | 网页或 `ttm cr approve` |
| `workflow` | `workflow.yaml` | 同上；应用后成为工作流的新草稿 |
| `rule` | `rules/<slug>.yaml` | 同上 |
| `agent` | `agents/<handle>.yaml` | 同上（需 owner/admin） |
| `skill` | `skills/<slug>/` | 批准 skill CR 即批准该 skill 版本 |

命令：`ttm cr list [--kind … --status …]`、`ttm cr show <id>`（逐文件差异）、`ttm cr approve <id>`、`ttm cr reject <id> --note
"…"`。见[命令参考](lenh.md)。

## 冲突

同一字段在仓库和网页上都被修改 → `409`，附 `{kind, target, path, base, local, server}` 列表；`ttm sync` 打印冲突，不发送任何内容，
退出码 1。解决方式与计划冲突相同：修改文件后再次 `ttm sync`，或用 `ttm sync --resolve` 让本地 agent 合并三个版本（YAML 文件使用
同样的三方合并）。

## AGENTS.md 的 "rules" 不同于 `.ttm/rules/`

| | `ttm rules apply` | `.ttm/rules/` |
| --- | --- | --- |
| 是什么 | 给**本地 agent** 的规则（`AGENTS.md`/`CLAUDE.md` 中的区块、钩子 `ttm hook pre-bash`/`stop`） | 项目的**自动化**规则（"当 … 如果 … 则 …"），由服务器运行 |
| 谁修改 | 服务器生成，`ttm rules apply` 写入 | 你修改文件，`ttm sync` 作为 `rule` CR 发送 |
| 查看 | `ttm rules apply` | `ttm automation list`, `show`, `test` |
