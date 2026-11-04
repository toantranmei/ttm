---
title: Factory
description: Liên kết repo với dự án Factory, cấu trúc .ttm, đồng bộ kế hoạch bằng change request, giải xung đột, luật cho agent và lệnh task.
order: 8
section: guides
---

Factory quản lý dự án cho cả người và agent: **project** (có key, ví dụ `AUR`), **task** (`AUR-12`), **workflow** (trạng
thái, chuyển trạng thái, cổng duyệt), **vai trò** (`PM`, `BA`, `UI/UX`, `DEV`, `GUARDIAN`, `QC` hoặc vai trò riêng của
workspace) và **người làm** (người hoặc agent). **Web là nguồn sự thật**: repo giữ một bản sao kế hoạch dạng Markdown;
sửa ở local đi lên dưới dạng **change request** để người có vai trò duyệt, không bao giờ ghi đè.

Các lệnh dưới đây (trừ `project init/link`) chạy trong một repo đã liên kết: `ttm` tìm `.ttm/project.toml` ở thư mục hiện
tại hoặc các thư mục cha.

## Liên kết repo

### Tạo project mới cho repo này

```sh
cd ~/code/aurora
ttm project init --key AUR --name "Aurora"
```

```text
Created project AUR (Aurora) and linked /Users/an/code/aurora.
```

| Cờ | Mặc định |
| --- | --- |
| `--key <KEY>` | (bắt buộc) tiền tố key task, được viết hoa |
| `--name <NAME>` | tên thư mục gốc của repo |
| `--repo-url <URL>` | `git remote get-url origin` |
| `--workspace <ID>` | workspace hiện tại của profile |

Nhà cung cấp repo suy từ URL: `github.com` → `github`, có chữ `gitlab` → `gitlab`, còn lại (hoặc không có remote) →
`local`. Nhánh chính lấy từ `origin/HEAD`, nếu không có thì `main` nếu tồn tại, nếu không thì `master`. Key đã có người dùng:
`409 conflict` (`rule: key_taken`).

### Liên kết với project có sẵn

```sh
ttm project link AUR          # key hoặc id
```

```text
Linked /Users/an/code/aurora to AUR (Aurora), plan rev 42.
Commit .ttm/project.toml and .ttm/plan; run `ttm rules apply` to install the project rules.
```

`ttm` tìm project trong các workspace của bạn, tải kế hoạch và ghi `.ttm/`. Không thấy:
``no project `AUR` in your workspaces``.

```sh
ttm project status            # key, id, host, rev đã đồng bộ, change request đang chờ
ttm project open              # mở <host>/factory?project=<id> trên trình duyệt
```

## Thư mục `.ttm`

```text
.ttm/
├─ project.toml        liên kết (commit)
├─ plan/               bản sao kế hoạch để đọc và sửa (commit)
│  ├─ index.md
│  └─ tasks/
│     ├─ AUR-1.md
│     └─ AUR-12.md
├─ base/               bản server ở lần đồng bộ trước, gốc để so 3 phía (bỏ qua)
├─ cache/              rules.json cho hook, dropped/ (bỏ qua)
├─ resolve/            chỗ làm việc tạm khi giải xung đột (bỏ qua)
└─ .gitignore          base/, cache/, resolve/
```

`project.toml`:

```toml
host = "https://toantranmei.com"
project_id = "0190f400-…"
key = "AUR"
synced_rev = 42
default_branch = "main"
pending_change_request = "0190f4a1-…"   # chỉ có khi một change request đang chờ duyệt
```

### File kế hoạch

`plan/index.md` tóm tắt sprint, mốc và bảng task (chỉ để đọc; sprint và mốc sửa trên web). Mỗi task là một file
`plan/tasks/<KEY>.md`, front matter có **giá trị JSON** (cũng là YAML hợp lệ):

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

| Sửa được ở local | Ghi chú |
| --- | --- |
| `title`, `state`, `role`, `estimate`, `priority`, `type`, `labels`, `ref` | Giá trị JSON |
| `assignee` | `"user:<id>"`, `"agent:<id>"` hoặc `null` |
| `deps` | Mảng key task |
| Mô tả | Phần thân trước `## Acceptance criteria` |
| Tiêu chí chấp nhận | Danh sách `- [ ]` / `- [x]` dưới `## Acceptance criteria` |
| File mới `tasks/<KEY>.md` | Tạo task mới |

`parent`, `status` và `rev` chỉ để đọc. Xoá file task hay sửa `index.md` không đổi gì trên server (bị liệt kê là
`skipped`). Đổi `state` đi qua workflow như trên web: guard và cổng duyệt vẫn áp, chuyển bị từ chối nằm trong `skipped`.

## Đồng bộ: `ttm sync`

```sh
ttm sync
```

1. Tải kế hoạch mới nhất của server.
2. So `.ttm/plan` (local) với `.ttm/base` (gốc chung). Có sửa local thì gửi lên một **change request** dựa trên
   `synced_rev`; nếu đã có change request đang chờ duyệt thì không gửi lại.
3. Đưa thay đổi của server vào `.ttm/plan` theo từng file: file bạn đang sửa giữ bản local cho tới khi change request được
   quyết; `.ttm/base` thành bản server; `synced_rev` cập nhật.

```text
Plan at rev 43 (2 file(s) updated from the server).
Local edits sent as change request 0190f4a1-…; they apply once approved on the web.
```

Change request được duyệt trên web bởi vai trò mà workflow quy định (mặc định `PM`), cổng duyệt của người áp dụng, và
**người tạo không tự duyệt** (`not-author`). Khi được áp, `rev` của project tăng; lần `ttm sync` sau kéo về.

Nếu server thấy sửa local không đổi gì so với hiện tại:

```text
Up to date: the server found nothing to change in the local edits (copies in .ttm/cache/dropped).
```

`.ttm/plan` lấy bản server; bản local bị thay được giữ trong `.ttm/cache/dropped/`.

### Kiểm tra trong CI: `--check`

```sh
ttm sync --check
```

```text
Plan out of sync: local rev 42, server rev 43.
  local:  tasks/AUR-12.md
  server: tasks/AUR-14.md
```

`--check` không ghi gì và không gửi gì; **thoát 0** khi khớp (`Plan in sync (rev 43).`), **thoát 1** khi lệch. Dùng trong
CI với một PAT có `factory:read`:

```yaml
- run: ttm sync --check
  env:
    TTM_TOKEN: ${{ secrets.TTM_TOKEN }}
```

`--check` không dùng chung với `--resolve`.

### Xung đột

Khi cùng một trường của cùng task đổi ở cả local và server (khác giá trị), server trả `409` với danh sách xung đột:

```text
Conflicts with the server (both sides changed the same fields):
  AUR-12 estimate: local 5 / server 3
Run `ttm sync --resolve` to merge them with a local agent, or edit .ttm/plan and retry.
```

Lệnh thoát 1 và không đổi file nào. Hai cách xử lý:

- **Tự sửa**: chỉnh `.ttm/plan` cho khớp ý định rồi `ttm sync` lại.
- **`ttm sync --resolve`**: để agent local hợp nhất.

`--resolve` cần Claude Code (`claude`) trên `PATH`:

1. Tạo nhánh `ttm/resolve-<rev>` trong **worktree riêng** `<worktree_root>/<KEY>/ttm-resolve-<rev>` từ nhánh chính mới
   nhất (worktree đang làm việc của bạn giữ nguyên).
2. Ghi ba phiên bản của các file xung đột vào `.ttm/resolve/base`, `.ttm/resolve/local`, `.ttm/resolve/server` trong
   worktree đó.
3. Chạy `claude -p "<prompt của server + hướng dẫn>" --allowedTools Read,Write,Edit,Glob,Grep`; agent ghi bản hợp nhất
   vào `.ttm/resolve/merged`.
4. Gửi bản hợp nhất thành change request mới, rồi xoá worktree.

Agent không ghi file nào: `the agent did not write any merged file in …`. Bản hợp nhất vẫn xung đột:
`the merged plan still conflicts on N field(s); run sync again`.

## Luật cho agent: `ttm rules apply`

```sh
ttm rules apply
```

```text
updated AGENTS.md
updated CLAUDE.md
updated .claude/settings.json
```

1. Ghi **khối được quản lý** vào `AGENTS.md` và `CLAUDE.md` (tạo file nếu chưa có), giữa hai dấu:

   ```markdown
   <!-- ttm:begin -->
   …luật của project do server biên dịch từ workflow…
   <!-- ttm:end -->
   ```

   Phần còn lại của file không bị đụng. Chỉ còn một trong hai dấu thì lệnh dừng:
   `the ttm markers are broken (…); fix the file and retry`.
2. Gộp **hook Claude Code** vào `.claude/settings.json` (giữ hook và thiết lập khác, không thêm trùng):

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

3. Lưu danh sách luật vào `.ttm/cache/rules.json` để hook chạy được khi offline.

Chạy lại khi luật không đổi: `Rules already up to date.` Commit `AGENTS.md`, `CLAUDE.md`, `.claude/settings.json` để cả
nhóm (và agent của runner) dùng chung. Runner tự chạy `ttm rules apply` trong mỗi worktree của job.

### Hook làm gì

Claude Code gọi hook với JSON trên stdin. Hook **thoát 2** để chặn hành động; thông báo trên stderr quay về cho agent:

```text
Blocked by project rule `no_push_main`: do not push to main directly; push a branch and open a pull request
```

| Luật | Hook | Chặn |
| --- | --- | --- |
| `no_push_main` | `pre-bash` | `git push` tới một nhánh được bảo vệ (kể cả `HEAD:refs/heads/main`, `+main`, và `git push` trần khi đang ở nhánh đó) |
| `no_verify` | `pre-bash` | `git commit`/`git push` có `--no-verify`, hoặc `git commit -n` |
| `conventional_commits` | `pre-bash` | `git commit -m` mà dòng đầu không theo Conventional Commits: `type(scope)!: tóm tắt`, `type` thuộc `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`, `merge` |
| `tests_before_done` | `stop` | Agent kết thúc mà chưa chạy lệnh test của project (lệnh phải xuất hiện trong transcript) |

- Hook tách dòng lệnh theo `&&`, `||`, `;`, `|`, hiểu dấu nháy và bỏ qua `git -C <dir>`, `git -c k=v`.
- Commit soạn trong editor (không có `-m`) không bị kiểm.
- Nhánh được bảo vệ: `hooks.pre_bash.protected_branches` trong luật server gửi (cài đặt `protected_branches` của project,
  mặc định là nhánh chính của project), lưu ở `.ttm/cache/rules.json` khi `ttm rules apply`. Chưa có luật thì dùng
  `default_branch` trong `.ttm/project.toml`, nếu không có nữa thì `main` và `master`. Thông báo chặn liệt kê các nhánh đó
  (`do not push to trunk/release directly; …`).
- Ngoài một repo đã liên kết, hook cho phép mọi thứ. Thiếu `.ttm/cache/rules.json` thì ba luật đầu bật.
- Server bật hay tắt từng luật theo cài đặt project (`commit_convention = none` tắt luật commit; `tests_before_done` bật
  khi workflow có guard `tests_pass` hoặc có `test_commands`).
- Mỗi kết quả được gửi lên thành sự kiện `rule_check` (bỏ qua nếu offline, tối đa 3 giây) và hiện trên Execution Map; trong
  một run nó gắn với `TTM_TASK`/`TTM_RUN_ID`.

Thử hook bằng tay:

```sh
echo '{"tool_input":{"command":"git push origin main"},"cwd":"'"$PWD"'"}' | ttm hook pre-bash; echo "exit $?"
```

## Task

```sh
ttm task list                     # task của project đã liên kết
ttm task list --mine --state doing
ttm task show AUR-12
ttm task claim AUR-12             # giao cho chính bạn
ttm task done AUR-12              # chuyển sang done (guard và cổng duyệt vẫn áp)
ttm task approve AUR-12           # duyệt bước đang chờ duyệt
ttm task approve AUR-12 --reject
ttm task split AUR-12 -f subtasks.json
```

```text
AUR-10     done           DEV       Bình             Màn hình đăng nhập
AUR-12     doing          DEV       Dev bot          Đăng nhập bằng Google
AUR-14     inbox          QC        -                Kiểm thử đăng nhập
```

Cột: key, trạng thái (hoặc loại), vai trò, người làm, tiêu đề. `--state` khớp id trạng thái hoặc **loại** (category, một
trong 9 cột chuẩn của board: `inbox`, `refinement`, `ready`, `ux-design`, `in-progress`, `design-review`, `qc`,
`approval`, `done`); `--mine` chỉ giữ task giao cho bạn.

`ttm task done` gửi `to: "done"` (id trạng thái hoặc loại). Workflow có thể từ chối với `409 conflict` và `rule`:

| `rule` | Nghĩa |
| --- | --- |
| `approval-required` | Cần người có vai trò duyệt trước (`ttm task approve`) |
| `tests-required`, `ci-required` | Lần chạy cuối chưa có kết quả test / CI đạt |
| `blocked-by-dependency` | Task phụ thuộc (hoặc task con) chưa xong |
| `wip-limit` | Cột đích đã đủ giới hạn WIP |
| `sprint-required` | Cần đưa task vào sprint đang chạy |
| `budget-exceeded` | Hết ngân sách token của task |
| `transition-not-allowed` | Workflow không cho chuyển từ trạng thái hiện tại |

`ttm task claim` dùng `If-Match` theo `rev` của task; ai đó vừa sửa task thì `412 precondition_failed`
(`rule: rev-conflict`): chạy lại.

### Tạo task con

`subtasks.json` (mảng, hoặc `{"tasks": [...]}`); tên trường viết **camelCase** như API, hoặc **snake_case** như tool MCP
`factory-create-subtasks` (`acceptance_criteria`, `depends_on`): `ttm` đổi sang tên server đọc trước khi gửi.

```json
[
  { "title": "API callback OAuth", "role": "DEV", "estimate": 2, "type": "feature" },
  { "title": "Kiểm thử đăng nhập", "role": "QC", "estimate": 1, "dependencies": ["#0"],
    "acceptanceCriteria": ["Đăng nhập lần đầu tạo tài khoản", "Đăng nhập lại không tạo trùng"] }
]
```

| Trường | Kiểu | Ghi chú |
| --- | --- | --- |
| `title` | chuỗi | Bắt buộc |
| `role` | chuỗi | Vai trò; scheduler giao cho thành viên giữ vai trò đó |
| `estimate` | số nguyên | Cộng dồn lên task cha |
| `type`, `priority`, `description` | chuỗi | |
| `labels` | mảng chuỗi | |
| `acceptanceCriteria` (hoặc `acceptance_criteria`) | mảng chuỗi | |
| `dependencies` (hoặc `depends_on`) | mảng | Key task có sẵn, hoặc `#<n>` = task con thứ n (đếm từ 0) của cùng file |

`kind` được hiểu là `type`. Khoá khác bị bỏ, kèm cảnh báo trên stderr:

```text
warning: subtask #1: unknown key `owner` ignored (known: title, type, priority, estimate, role, description, labels, acceptanceCriteria, dependencies, or acceptance_criteria, depends_on)
2 subtask(s) created under AUR-12.
```

Giới hạn mặc định (workflow chỉnh được): sâu tối đa **2 cấp** dưới task gốc, **10** task con mỗi run, **30** dưới một task
gốc; vai trò phải được phép chia (mặc định `PM`, `BA`, `DEV`). Vượt: `409 conflict` với `rule: split-limit` và `params`
cho biết giới hạn hiện tại (`max_depth`, `per_run`, `per_root`, `depth`, `run_count`, `root_count`). Task con kế thừa epic,
`ref`, sprint của cha và bắt đầu ở trạng thái `inbox` (hoặc `ready` nếu workflow cho agent). Vượt sức chứa sprint thì vẫn
tạo nhưng nằm ngoài sprint, kèm `warning`.

## Agent trong một task

Agent do runner chạy dùng các tool MCP `factory-*` (lấy task, context, báo tiến độ, xin duyệt, tạo task con, đính kèm PR
hay kết quả test, đọc luật, giữ vùng code). Xem [Runner](runner.md) và [Tool MCP](mcp-cong-cu.md).
