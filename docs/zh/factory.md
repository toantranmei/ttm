---
title: Factory
description: 将仓库关联到 Factory 项目，.ttm 目录结构，通过变更请求同步计划，解决冲突，agent 规则以及任务命令。
order: 8
section: guides
---

Factory 同时为人和 agent 管理项目：**项目**（project，带 key，例如 `AUR`）、**任务**（task，`AUR-12`）、**工作流**（workflow：状态、状态流转、审批关卡）、**角色**（`PM`、`BA`、`UI/UX`、`DEV`、`GUARDIAN`、`QC`
或工作区自定义的角色）以及**执行者**（人或 agent）。**网页是唯一事实来源**：仓库保存一份 Markdown 形式的计划副本；本地的修改会以**变更请求**（change request）的形式提交上去，由具有相应角色的人审批，永远不会直接覆盖。

下面的命令（`project init/link` 除外）需要在已关联的仓库中运行：`ttm` 会在当前目录或其父目录中查找 `.ttm/project.toml`。

## 关联仓库

### 为当前仓库创建新项目

```sh
cd ~/code/aurora
ttm project init --key AUR --name "Aurora"
```

```text
Created project AUR (Aurora) and linked /Users/an/code/aurora.
```

| 标志 | 默认值 |
| --- | --- |
| `--key <KEY>` | （必填）任务 key 的前缀，会转为大写 |
| `--name <NAME>` | 仓库根目录的名称 |
| `--repo-url <URL>` | `git remote get-url origin` |
| `--workspace <ID>` | profile 的当前工作区 |

仓库提供方根据 URL 推断：`github.com` → `github`，包含 `gitlab` → `gitlab`，其他情况（或没有 remote）→ `local`。主分支取自
`origin/HEAD`；没有的话，存在 `main` 就用 `main`，否则用 `master`。key 已被占用：`409 conflict`（`rule: key_taken`）。

### 关联到已有项目

```sh
ttm project link AUR          # key 或 id
```

```text
Linked /Users/an/code/aurora to AUR (Aurora), plan rev 42.
Commit .ttm/project.toml and .ttm/plan; run `ttm rules apply` to install the project rules.
```

`ttm` 会在你的各个工作区中查找该项目，下载计划并写入 `.ttm/`。找不到时：``no project `AUR` in your workspaces``。

```sh
ttm project status            # key、id、host、已同步的 rev、待审批的变更请求
ttm project open              # 在浏览器中打开 <host>/factory?project=<id>
```

## `.ttm` 目录

```text
.ttm/
├─ project.toml        关联信息（提交）
├─ plan/               供阅读和编辑的计划副本（提交）
│  ├─ index.md
│  └─ tasks/
│     ├─ AUR-1.md
│     └─ AUR-12.md
├─ base/               上次同步时的服务器版本，三方比较的基准（忽略）
├─ cache/              供 hook 使用的 rules.json、dropped/（忽略）
├─ resolve/            解决冲突时的临时工作区（忽略）
└─ .gitignore          base/、cache/、resolve/
```

`project.toml`：

```toml
host = "https://toantranmei.com"
project_id = "0190f400-…"
key = "AUR"
synced_rev = 42
default_branch = "main"
pending_change_request = "0190f4a1-…"   # 仅在有变更请求等待审批时存在
```

### 计划文件

`plan/index.md` 汇总 sprint、里程碑和任务表（只读；sprint 和里程碑在网页上修改）。每个任务是一个文件
`plan/tasks/<KEY>.md`，front matter 使用 **JSON 值**（同时也是合法的 YAML）：

```markdown
---
key: "AUR-12"
title: "Đăng nhập bằng Google"
state: "doing"
role: "DEV"
assignee: "agent:0190f3ff-…"
deps: ["AUR-10"]
estimate: 3
priority: "high"
type: "feature"
labels: ["auth"]
ref: null
parent: null
status: "in-progress"
rev: 7
---

Cho phép đăng nhập bằng tài khoản Google.

## Acceptance criteria

- [x] Nút "Đăng nhập bằng Google"
- [ ] Liên kết với tài khoản có sẵn theo email
```

| 可在本地修改 | 说明 |
| --- | --- |
| `title`、`state`、`role`、`estimate`、`priority`、`type`、`labels`、`ref` | JSON 值 |
| `assignee` | `"user:<id>"`、`"agent:<id>"` 或 `null` |
| `deps` | 任务 key 数组 |
| 描述 | `## Acceptance criteria` 之前的正文 |
| 验收标准 | `## Acceptance criteria` 下的 `- [ ]` / `- [x]` 列表 |
| 新文件 `tasks/<KEY>.md` | 创建新任务 |

`parent`、`status` 和 `rev` 是只读的。删除任务文件或修改 `index.md` 不会改变服务器上的任何内容（会被列为 `skipped`）。修改
`state` 与在网页上一样经过工作流：guard 和审批关卡依然生效，被拒绝的流转会出现在 `skipped` 中。

## 同步：`ttm sync`

```sh
ttm sync
```

1. 下载服务器上最新的计划。
2. 将 `.ttm/plan`（本地）与 `.ttm/base`（共同基准）比较。如果有本地修改，就基于 `synced_rev` 提交一个**变更请求**；如果已有变更请求在等待审批，则不重复提交。
3. 按文件将服务器的变更合入 `.ttm/plan`：你正在修改的文件保留本地版本，直到该变更请求有结论；`.ttm/base` 更新为服务器版本；`synced_rev`
   随之更新。

```text
Plan at rev 43 (2 file(s) updated from the server).
Local edits sent as change request 0190f4a1-…; they apply once approved on the web.
```

变更请求在网页上由工作流规定的角色（默认 `PM`）审批，人工审批关卡同样适用，并且**创建者不能自己审批**（`not-author`）。应用后，项目的
`rev` 会增加；下一次 `ttm sync` 会拉取下来。

如果服务器发现本地修改与当前内容相比没有任何变化：

```text
Up to date: the server found nothing to change in the local edits (copies in .ttm/cache/dropped).
```

`.ttm/plan` 采用服务器版本；被替换的本地版本保存在 `.ttm/cache/dropped/` 中。

### 在 CI 中检查：`--check`

```sh
ttm sync --check
```

```text
Plan out of sync: local rev 42, server rev 43.
  local:  tasks/AUR-12.md
  server: tasks/AUR-14.md
```

`--check` 不写入也不提交任何内容；一致时**以 0 退出**（`Plan in sync (rev 43).`），不一致时**以 1 退出**。在 CI 中配合具有
`factory:read` 的 PAT 使用：

```yaml
- run: ttm sync --check
  env:
    TTM_TOKEN: ${{ secrets.TTM_TOKEN }}
```

`--check` 不能与 `--resolve` 同时使用。

### 冲突

当同一任务的同一字段在本地和服务器上都被修改（且值不同）时，服务器返回 `409` 以及冲突列表：

```text
Conflicts with the server (both sides changed the same fields):
  AUR-12 estimate: local 5 / server 3
Run `ttm sync --resolve` to merge them with a local agent, or edit .ttm/plan and retry.
```

命令以 1 退出，不修改任何文件。有两种处理方式：

- **手动修改**：按你的意图调整 `.ttm/plan`，然后再次执行 `ttm sync`。
- **`ttm sync --resolve`**：让本地 agent 进行合并。

`--resolve` 需要 `PATH` 中有 Claude Code（`claude`）：

1. 从最新的主分支创建分支 `ttm/resolve-<rev>`，位于**独立的 worktree** `<worktree_root>/<KEY>/ttm-resolve-<rev>` 中（你正在工作的
   worktree 保持不变）。
2. 把冲突文件的三个版本写入该 worktree 中的 `.ttm/resolve/base`、`.ttm/resolve/local`、`.ttm/resolve/server`。
3. 运行 `claude -p "<服务器的 prompt + 说明>" --allowedTools Read,Write,Edit,Glob,Grep`；agent 把合并结果写入
   `.ttm/resolve/merged`。
4. 把合并结果作为新的变更请求提交，然后删除该 worktree。

agent 没有写入任何文件：`the agent did not write any merged file in …`。合并结果仍有冲突：
`the merged plan still conflicts on N field(s); run sync again`。

## Agent 规则：`ttm rules apply`

```sh
ttm rules apply
```

```text
updated AGENTS.md
updated CLAUDE.md
updated .claude/settings.json
```

1. 在 `AGENTS.md` 和 `CLAUDE.md` 中写入**受管理的区块**（文件不存在时会创建），位于两个标记之间：

   ```markdown
   <!-- ttm:begin -->
   …由服务器根据工作流编译出的项目规则…
   <!-- ttm:end -->
   ```

   文件的其余部分不会被改动。如果只剩下其中一个标记，命令会停止：
   `the ttm markers are broken (…); fix the file and retry`。
2. 把 **Claude Code hook** 合并到 `.claude/settings.json`（保留其他 hook 和设置，不重复添加）：

   ```json
   {
     "hooks": {
       "PreToolUse": [
         { "matcher": "Bash", "hooks": [{ "type": "command", "command": "ttm hook pre-bash" }] }
       ],
       "Stop": [
         { "hooks": [{ "type": "command", "command": "ttm hook stop" }] }
       ]
     }
   }
   ```

3. 把规则列表保存到 `.ttm/cache/rules.json`，使 hook 在离线时也能运行。

规则没有变化时再次运行：`Rules already up to date.` 请提交 `AGENTS.md`、`CLAUDE.md`、`.claude/settings.json`，让整个团队（以及
runner 的 agent）共用。runner 会在每个作业的 worktree 中自动运行 `ttm rules apply`。

### Hook 做什么

Claude Code 通过 stdin 传入 JSON 调用 hook。hook **以 2 退出**表示阻止该操作；stderr 上的消息会返回给 agent：

```text
Blocked by project rule `no_push_main`: do not push to main directly; push a branch and open a pull request
```

| 规则 | Hook | 阻止 |
| --- | --- | --- |
| `no_push_main` | `pre-bash` | 推送到受保护分支的 `git push`（包括 `HEAD:refs/heads/main`、`+main`，以及在该分支上执行不带参数的 `git push`） |
| `no_verify` | `pre-bash` | 带 `--no-verify` 的 `git commit`/`git push`，或 `git commit -n` |
| `conventional_commits` | `pre-bash` | 首行不符合 Conventional Commits 的 `git commit -m`：`type(scope)!: 摘要`，`type` 属于 `feat`、`fix`、`docs`、`style`、`refactor`、`perf`、`test`、`build`、`ci`、`chore`、`revert`、`merge` |
| `tests_before_done` | `stop` | agent 在未运行项目测试命令的情况下结束（命令必须出现在 transcript 中） |

- hook 按 `&&`、`||`、`;`、`|` 拆分命令行，能理解引号，并跳过 `git -C <dir>`、`git -c k=v`。
- 在编辑器中撰写的提交（没有 `-m`）不做检查。
- 受保护分支：服务器下发规则中的 `hooks.pre_bash.protected_branches`（项目设置 `protected_branches`，默认是项目的默认
  分支），由 `ttm rules apply` 保存在 `.ttm/cache/rules.json`。没有缓存的规则时使用 `.ttm/project.toml` 中的
  `default_branch`；仍没有则为 `main` 和 `master`。拦截消息会列出这些分支（`do not push to trunk/release directly; …`）。
- 在已关联的仓库之外，hook 放行一切。缺少 `.ttm/cache/rules.json` 时，前三条规则生效。
- 服务器根据项目设置启用或关闭每条规则（`commit_convention = none` 关闭提交规则；当工作流有 `tests_pass` guard 或设置了
  `test_commands` 时，`tests_before_done` 生效）。
- 每个结果都会作为 `rule_check` 事件上报（离线时跳过，最多 3 秒），并显示在 Execution Map 上；在一次 run 中，它会关联到
  `TTM_TASK`/`TTM_RUN_ID`。

手动测试 hook：

```sh
echo '{"tool_input":{"command":"git push origin main"},"cwd":"'"$PWD"'"}' | ttm hook pre-bash; echo "exit $?"
```

## 任务

```sh
ttm task list                     # 已关联项目的任务
ttm task list --mine --state doing
ttm task show AUR-12
ttm task claim AUR-12             # 分配给你自己
ttm task done AUR-12              # 转到 done（guard 和审批关卡依然生效）
ttm task approve AUR-12           # 审批正在等待审批的步骤
ttm task approve AUR-12 --reject
ttm task split AUR-12 -f subtasks.json
```

```text
AUR-10     done           DEV       Bình             Màn hình đăng nhập
AUR-12     doing          DEV       Dev bot          Đăng nhập bằng Google
AUR-14     inbox          QC        -                Kiểm thử đăng nhập
```

列：key、状态（或类别）、角色、执行者、标题。`--state` 匹配状态 id 或**类别**（category，看板 9 个标准列之一：`inbox`、`refinement`、`ready`、`ux-design`、`in-progress`、`design-review`、`qc`、`approval`、`done`）；`--mine`
只保留分配给你的任务。

`ttm task done` 发送 `to: "done"`（状态 id 或类别）。工作流可能以 `409 conflict` 和 `rule` 拒绝：

| `rule` | 含义 |
| --- | --- |
| `approval-required` | 需要先由具有相应角色的人审批（`ttm task approve`） |
| `tests-required`、`ci-required` | 最近一次运行还没有通过的测试 / CI 结果 |
| `blocked-by-dependency` | 依赖的任务（或子任务）尚未完成 |
| `wip-limit` | 目标列已达到 WIP 上限 |
| `sprint-required` | 需要把任务放入进行中的 sprint |
| `budget-exceeded` | 任务的 token 预算已用完 |
| `transition-not-allowed` | 工作流不允许从当前状态流转 |

`ttm task claim` 按任务的 `rev` 使用 `If-Match`；如果有人刚修改过该任务，会返回 `412 precondition_failed`
（`rule: rev-conflict`）：重新执行即可。

### 创建子任务

`subtasks.json`（数组，或 `{"tasks": [...]}`）；字段名可用与 API 一致的 **camelCase**，也可用与 MCP 工具
`factory-create-subtasks` 一致的 **snake_case**（`acceptance_criteria`、`depends_on`）：`ttm` 在发送前会改成服务器读取的名称。

```json
[
  { "title": "API callback OAuth", "role": "DEV", "estimate": 2, "type": "feature" },
  { "title": "Kiểm thử đăng nhập", "role": "QC", "estimate": 1, "dependencies": ["#0"],
    "acceptanceCriteria": ["Đăng nhập lần đầu tạo tài khoản", "Đăng nhập lại không tạo trùng"] }
]
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `title` | 字符串 | 必填 |
| `role` | 字符串 | 角色；scheduler 会分配给担任该角色的成员 |
| `estimate` | 整数 | 累加到父任务上 |
| `type`、`priority`、`description` | 字符串 | |
| `labels` | 字符串数组 | |
| `acceptanceCriteria`（或 `acceptance_criteria`） | 字符串数组 | |
| `dependencies`（或 `depends_on`） | 数组 | 已有任务的 key，或 `#<n>` = 同一文件中的第 n 个子任务（从 0 开始计数） |

`kind` 视为 `type`。其他键会被丢弃，并在 stderr 上给出警告：

```text
warning: subtask #1: unknown key `owner` ignored (known: title, type, priority, estimate, role, description, labels, acceptanceCriteria, dependencies, or acceptance_criteria, depends_on)
2 subtask(s) created under AUR-12.
```

默认限制（可在工作流中调整）：根任务之下最多 **2 层**，每次 run 最多 **10** 个子任务，一个根任务之下最多 **30** 个；角色必须被允许拆分（默认
`PM`、`BA`、`DEV`）。超出时：`409 conflict`，带 `rule: split-limit` 和说明当前限制的 `params`（`max_depth`、`per_run`、`per_root`、`depth`、`run_count`、`root_count`）。子任务继承父任务的
epic、`ref`、sprint，初始状态为 `inbox`（如果工作流交给 agent，则为 `ready`）。超出 sprint 容量时仍会创建，但不在 sprint 内，并附带
`warning`。

## 任务中的 agent

由 runner 运行的 agent 使用 `factory-*` MCP 工具（获取任务、上下文、报告进度、请求审批、创建子任务、附加 PR 或测试结果、读取规则、锁定代码区域）。见
[Runner](runner.md) 和 [MCP 工具](mcp-cong-cu.md)。
