---
title: Runner
description: Chạy agent của Factory trên máy bạn - cấu hình repo, sức chứa, dịch vụ nền, ttm agent, yêu cầu của Claude Code, token và mô hình bảo mật.
order: 9
section: guides
---

**Runner** là `ttm runner up`: một tiến trình trên máy dev nhận việc (job) mà server Factory giao, chạy coding agent
(**Claude Code**) bằng tài khoản Claude của bạn trong một git worktree riêng cho mỗi task, gửi hoạt động về Execution Map
theo thời gian thực, rồi đẩy nhánh và mở pull request. Server quyết định giao việc gì, model nào, prompt ra sao; runner
chỉ thực thi.

## Yêu cầu

| Cần | Vì sao |
| --- | --- |
| `ttm` và `ttm-mcp` cùng thư mục (hoặc `ttm-mcp` trên `PATH`) | Agent dùng tool MCP `factory-*` và `notee-*` |
| Đăng nhập với scope `factory:run` (mặc định của `ttm login`) | Đăng ký runner, nhận job |
| `git` | Worktree, commit, push |
| [Claude Code](https://docs.anthropic.com/claude-code) (`claude` trên `PATH`), đã đăng nhập gói Claude hoặc đặt `ANTHROPIC_API_KEY` | Harness `claude-code` |
| `gh` (GitHub CLI) đã `gh auth login` (tuỳ chọn) | Mở pull request sau khi đẩy nhánh |
| Một bản clone của repo dự án, có remote `origin` | Runner chỉ làm việc trong clone bạn đã khai báo |

`ttm doctor` kiểm `git`, `claude` và `ttm-mcp`.

## Bắt đầu

```sh
ttm runner config --repo AUR=~/code/aurora --max-parallel 2
ttm runner up
```

```text
Runner 0190f500-… up: 2 agent(s) at once. Ctrl-C to stop.
```

Giao một task cho agent trên web (hoặc `ttm agent run AUR-12`), runner nhận job trong vài giây. Theo dõi trên Execution
Map hoặc bằng `ttm agent logs <run> -f`.

## Cấu hình: `ttm runner config`

```sh
ttm runner config --name "macbook-an" --max-parallel 2 --keep-days 7 \
  --worktree-root ~/orca/workspaces --repo AUR=~/code/aurora --repo WEB=~/code/web
ttm runner config --accept-team-jobs true --auto-merge false
```

```text
Saved /Users/an/.config/ttm/runner.toml
```

| Cờ | Mặc định | Việc |
| --- | --- | --- |
| `--name <NAME>` | tên máy (`HOSTNAME`/`COMPUTERNAME`/`hostname`) | Tên hiện trên web; đăng ký lại cùng tên giữ nguyên id runner |
| `--max-parallel <N>` | tính từ máy | Số agent chạy cùng lúc (≥ 1) |
| `--accept-team-jobs true\|false` | `false` | Nhận cả job cho task của đồng đội, không chỉ task của bạn |
| `--worktree-root <DIR>` | `worktree_root` của `config.toml`, rồi `~/.ttm/worktrees` | Nơi tạo worktree |
| `--keep-days <N>` | `3` | Số ngày giữ worktree của job đã xong |
| `--repo KEY=PATH` | | Clone local của project `KEY` (lặp lại được; key viết hoa, đường dẫn được chuẩn hoá, `~/` được mở rộng) |
| `--auto-merge true\|false` | `false` | Tự merge phần tử đầu của merge queue và push |

Chỉ cờ bạn truyền mới đổi; giá trị khác giữ nguyên. File `runner.toml` nằm cạnh `config.toml`:

```toml
name = "macbook-an"
max_parallel = 2
accept_team_jobs = false
worktree_root = "~/orca/workspaces"
keep_days = 7
runner_id = "0190f500-…"     # ghi bởi lần đăng ký đầu
auto_merge = false

[repos]
AUR = "/Users/an/code/aurora"
WEB = "/Users/an/code/web"

[mcp.local.github-local]                         # MCP local (0.2.0), xem mục dưới
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "env:GITHUB_TOKEN" }
allow = ["dev", "reviewer"]
team_jobs = false
```

Chạy lại `ttm runner up` (hoặc khởi động lại dịch vụ) để áp cấu hình mới. Riêng mục `[mcp.local.*]` được đọc lại ở mỗi
job, không cần khởi động lại.

### Vì sao phải khai báo repo

Runner **không bao giờ clone một URL server gửi tới**. Mỗi job nêu repo của nó; runner chỉ chạy khi:

1. project của job có clone trong `[repos]` (theo key, không phân biệt hoa thường, hoặc theo id);
2. `origin` của clone đó trùng repo của job (so sau khi chuẩn hoá: bỏ giao thức, user, cổng, `.git`, `/` cuối; ví dụ
   `git@github.com:a/b.git` = `https://github.com/a/b`); với project `local`, đường dẫn của job phải trùng đường dẫn đã
   khai báo;
3. tên nhánh của job hợp lệ theo luật của git;
4. worktree nằm dưới `worktree_root`: `worktree_hint` của job chỉ được là đường dẫn tương đối; đường dẫn tuyệt đối (`/…`,
   `C:\…`), có `..`, `\` hoặc `:`, hay đi qua một symlink dẫn ra ngoài `worktree_root` đều bị từ chối.

Không thoả thì job bị từ chối và báo về server (`repo_not_configured`, `invalid_branch` hoặc `invalid_worktree`), terminal
in:

```text
ttm runner: refused AUR-12: no local clone is configured for project AUR; add it with `ttm runner config --repo AUR=<path to your clone>`
```

Nhờ vậy một tài khoản bị chiếm hay một job giả không thể bắt máy bạn tải và chạy mã từ nơi lạ.

### Sức chứa

Số agent chạy cùng lúc = `--max-parallel` nếu đặt; nếu không, `clamp(số nhân CPU / 4, 1, 3)`, hạ xuống để mỗi agent có ít
nhất **4 GB RAM trống**. Server có thể giới hạn thấp hơn theo gói (Free 1, Pro 3, Team 6 mỗi runner). Khi agent báo hết hạn
mức sử dụng của Claude, runner **tạm dừng nhận job** tới giờ reset (run đó kết thúc với trạng thái `paused` kèm
`paused_until`), server xếp lại việc thay vì thử liên tục.

## MCP local: `ttm runner config mcp`

MCP local là server MCP chạy **trên máy bạn** (ví dụ server GitHub với token của bạn). Mặc định **không agent nào** được
dùng: bạn phải cho phép từng agent (theo handle).

```sh
ttm runner config mcp add github-local --command npx --arg -y --arg @modelcontextprotocol/server-github \
  --env GITHUB_TOKEN=env:GITHUB_TOKEN
ttm runner config mcp allow dev --mcp github-local     # bỏ --mcp: mọi MCP local
ttm runner config mcp deny dev --mcp github-local
ttm runner config mcp team-jobs github-local on         # cho cả job do người khác giao
ttm runner config mcp remove github-local
ttm runner config mcp list
```

| Khoá | Ý nghĩa |
| --- | --- |
| tên (`[mcp.local.<tên>]`) | `^[a-z0-9][a-z0-9-]{0,31}$`, không được là `ttm` (server MCP của chính runner) |
| `command`, `args` | Lệnh khởi động server |
| `env` | `K = "env:VAR"`: đọc biến `VAR` của runner **lúc chạy** (không ghi giá trị thật vào file); `K = "giá trị"` chỉ cho giá trị không bí mật. `--env` từ chối giá trị trông giống token |
| `allow` | Handle agent được dùng |
| `team_jobs` | `true`: job do người khác giao cũng được dùng (mặc định `false`) |

Một job nhận MCP local khi: server có tên trong `job.mcp.local`, agent của job nằm trong `allow`, và job không phải job
của đồng đội hoặc `team_jobs = true`. Biến `env:VAR` thiếu thì MCP đó bị bỏ (kèm sự kiện cảnh báo); tên trùng `ttm` hoặc
trùng một MCP từ xa cũng bị bỏ. `ttm runner config mcp list` ghi rõ: **chỉ tên, agent được phép và `team_jobs` được báo
lên server**; lệnh, tham số và biến môi trường không bao giờ rời máy. Các lệnh này sửa `runner.toml` bằng `toml_edit` nên
comment của bạn được giữ.

### Token Claude cho job của đồng đội: `ttm runner config claude-token`

```sh
claude setup-token | ttm runner config claude-token     # đọc token trên stdin, lưu vào keyring
ttm runner config claude-token --remove
```

Xem [Cô lập job của đồng đội](#cô-lập-job-của-đồng-đội).

## Một job chạy thế nào

1. **Kiểm tra** repo và nhánh (mục trên).
2. **Worktree**: `git fetch origin <nhánh chính>`, rồi `git worktree add -B <nhánh> <đường dẫn> origin/<nhánh chính>`.
   Đường dẫn: `<worktree_root>/<worktree_hint>` (server gợi ý `<key viết thường>/<nhánh bỏ ttm/>`, ví dụ
   `aur/aur-12-dang-nhap-google`); job không có gợi ý thì `<worktree_root>/<KEY>/<nhánh>` (ký tự lạ thành `-`).
3. **Luật**: chạy `ttm rules apply` trong worktree (khối `AGENTS.md`/`CLAUDE.md`, hook Claude Code).
4. **Skill và hook** (0.2.0): cài skill của job (mục [Skill](#skill)), chuẩn bị thư mục hook riêng của run (mục
   [Danh tính git](#danh-tính-git-và-hook-commit-msg)).
5. **MCP**: ghi `<cache>/mcp-<run>.json` (quyền 0600) trỏ tới `ttm-mcp --toolsets <nhóm của job, mặc định factory,notee>`
   với `TTM_TOKEN` là token của job, `TTM_RUN_ID`, `TTM_TASK`, `TTM_PROJECT`, cộng MCP từ xa qua gateway và MCP local được
   cấp (mục [MCP](#mcp-qua-gateway-và-mcp-local)). Claude Code chạy với `--strict-mcp-config`: agent chỉ thấy các server này.
6. **Agent**: `claude -p <prompt> --output-format stream-json --verbose --mcp-config <file> --settings
   <cache>/settings-<run>.json [--model <m>] [--max-turns <n>] [--allowedTools <danh sách>]` trong worktree; model, số
   lượt và tool cho phép do job quy định. Thời gian tối đa theo job, mặc định **120 phút**.
7. **Sự kiện**: mỗi dòng stream-json thành một sự kiện gửi lên theo lô (mỗi giây hoặc 25 sự kiện):

   | Claude Code | Sự kiện |
   | --- | --- |
   | khởi động | `log` (`agent started <model>`) |
   | văn bản của agent | `thought` |
   | gọi tool | `action` (`TodoWrite` → `plan`) |
   | kết quả cuối | `response`, kèm số token vào/ra |
   | lệnh test (`cargo test`, `pnpm test`, `pytest`, `go test`, `vitest`, `playwright test`…) | `log` `tests passed`/`tests failed`; kết quả được đính vào run như `test_result` |

8. **Kết thúc**: nếu thành công, commit phần agent để lại (`chore(<key>): changes from agent run <run>`), push nhánh, mở
   PR bằng `gh pr create` nếu có `gh`, rồi báo trạng thái (`succeeded`, `failed`, `cancelled`, `paused`), commit, PR, token
   đã dùng. File MCP tạm bị xoá.

Worktree của job đã xong được giữ `keep_days` ngày để bạn xem lại, rồi bị xoá khi `ttm runner up` khởi động lần sau.

Mỗi run in một dòng khi bắt đầu và một dòng khi kết thúc (stderr):

```text
● AUR-12 running · sonnet · worktree /Users/an/orca/workspaces/aur/aur-12-dang-nhap-google
✓ AUR-12 succeeded → https://github.com/an/aurora/pull/7
✕ AUR-13 refused: repo_not_configured add it with `ttm runner config --repo AUR=<path to your clone>`
```

### Danh tính git và hook `commit-msg`

- Job có `git` (agent v2): commit mang tên và email của **agent** (`GIT_AUTHOR_*`, `GIT_COMMITTER_*`), ví dụ
  `DEV · Code <dev+aurora@agents.toantranmei.com>`. Job không có `git` (task của người) giữ danh tính git của máy.
- **Mọi job**, ở mọi chế độ `commit_identity` (`agent`, `delegator`, `delegator_with_trailer`), chạy git với
  `core.hooksPath` trỏ tới thư mục hook riêng `<cache>/hooks/<run>/` (qua `GIT_CONFIG_COUNT/KEY/VALUE`, không sửa file
  cấu hình nào). Hook `commit-msg` ở đó:
  1. xoá mọi dòng (không phân biệt hoa thường) khớp `^co-authored-by:.*(claude|anthropic)`, `generated (with|by) .*claude`,
     `🤖 generated with`, `noreply@anthropic.com`;
  2. thêm trailer của job (chỉ chế độ `delegator_with_trailer`: `Co-authored-by: <tên agent> <email agent>`, một lần);
  3. chạy hook `commit-msg` gốc của repo (từ `core.hooksPath` hoặc `.git/hooks`). Các hook gốc khác được nối qua shim.
- Claude Code luôn chạy với `--settings` có `{"includeCoAuthoredBy": false, "attribution": {"commit": "", "pr": ""}}`.
- Commit phần còn lại và push của runner dùng cùng môi trường. Thư mục hook bị xoá sau run.

Quy tắc: không commit nào do runner tạo mang dòng Claude/Anthropic.

### Skill

Job có thể mang các skill **đã được duyệt** (`job.skills`: slug, phiên bản, URL tải ký sẵn 10 phút, SHA-256, kích thước).
Runner:

1. dùng bản cache `<cache>/skills/<sha256>.zip` nếu có, không thì tải (60 giây, dừng khi vượt kích thước khai báo hoặc
   5 MB);
2. tính SHA-256; lệch → **từ chối job** với `skill_hash_mismatch`;
3. giải nén an toàn (không `..`, không symlink, ≤ 200 file, ≤ 5 MB) vào `<worktree>/.claude/skills/ttm-<slug>/`; thư
   mục đó đã được git theo dõi thì bỏ skill (cảnh báo);
4. thêm một lần dòng `/.claude/skills/ttm-*/` vào `info/exclude` của repo, để skill không bao giờ bị commit;
5. xoá các thư mục đã cài sau run, dù thành công, lỗi, huỷ hay hết giờ.

### MCP qua gateway và MCP local

- **MCP từ xa** của workspace (`ttm mcp-server …`): job nhận `{name, url}` trỏ tới gateway của server
  (`/api/v1/mcp/gw/<server>`) và gọi bằng token của job. Secret của MCP server (OAuth, header bí mật) nằm ở server,
  không bao giờ xuống máy bạn.
- **MCP local**: xem [MCP local](#mcp-local-ttm-runner-config-mcp).

### Cô lập job của đồng đội

Job của đồng đội (`team_job`: người giao không phải chủ runner) không được thấy cấu hình Claude cá nhân của bạn:

- luôn chạy với `--setting-sources project,local` (bỏ `~/.claude/settings.json` của bạn);
- khi đã lưu token bằng `ttm runner config claude-token`, chạy thêm với `CLAUDE_CONFIG_DIR` trống riêng cho mỗi run và
  `CLAUDE_CODE_OAUTH_TOKEN` là token đó, nên không thấy hook, plugin hay skill cá nhân `~/.claude/skills`;
- runner chỉ báo tính năng `team_isolation` khi đã có token. Chưa có token thì runner **từ chối** mọi job của đồng đội
  (`unsupported_feature`, báo lên server, không bao giờ chạy với cấu hình Claude của máy), và `ttm runner status` nhắc lệnh
  cần chạy.

Kết quả spike (Claude Code 2.1.318, chỉ đọc `claude --help`): `--setting-sources` có, nhưng tài liệu không nói rõ có bỏ
plugin và skill cá nhân hay không, nên chỉ cờ này **chưa đủ**; vì vậy dùng thêm `CLAUDE_CONFIG_DIR` trống. Sẽ kiểm lại bằng
job thật ở đợt giả lập M4.

### Tính năng (`features`)

Khi đăng ký, runner báo tính năng nó có: `git_identity`, `skills`, `mcp_gateway`, `mcp_local`, và `team_isolation` khi có
token Claude cho job đồng đội; cùng danh sách MCP local (`name`, `allow`, `team_jobs`). Heartbeat gửi lại danh sách MCP
local khi `runner.toml` đổi. Job có `requires` chứa tính năng runner không có bị từ chối với `unsupported_feature`.

### Kết nối với server

- Runner chỉ mở kết nối **đi ra** (chạy được sau NAT, không cần mở cổng): heartbeat mỗi 20 giây và long-poll job
  (`?wait=25`, server giữ yêu cầu tối đa 25 giây) khi còn chỗ trống. Yêu cầu long-poll đang chờ không bị huỷ khi tới lượt
  heartbeat, nên job server đã giao không bị mất.
- Heartbeat mang danh sách run đang chạy; server trả về các run cần dừng (**kill switch** từ web hoặc `ttm agent stop`)
  hoặc lệnh tắt runner (“turned off from the web”).
- Token runner sống 12 giờ và được gia hạn tự động 30 phút trước khi hết (đăng ký lại, token cũ bị thu hồi). Bị từ chối
  `401` liên tục hơn 3 lần thì runner dừng.

### Tự merge (`auto_merge`)

Khi bật, mỗi phút runner lấy phần tử đầu merge queue của các project có trong `[repos]`, merge `origin/<nhánh của run>`
vào nhánh chính trong worktree tạm `ttm/merge-<id>` (`git merge --no-ff`, commit `merge: <KEY> (<nhánh>)`), push
`HEAD:<nhánh chính>` và báo `merged` hoặc `conflict` (server tạo task “resolve” cho xung đột). Việc báo kết quả dùng token
người dùng của bạn (cần `factory:write`).

## Theo dõi và dừng

```sh
ttm runner status
```

```text
● macbook-an running
     runner  0190f500-…
        pid  4242
   capacity  2
  team jobs  no
  harnesses  claude-code
  worktrees  /Users/an/orca/workspaces
   features  git_identity, skills, mcp_gateway, mcp_local

PROJECT  CLONE
───────────────────────────────
AUR      /Users/an/code/aurora
WEB      /Users/an/code/web
```

Thêm bảng MCP local (tên, lệnh, agent được phép, team jobs) khi có, và cảnh báo khi job của đồng đội chưa được cô lập.

```sh
ttm runner down     # gửi SIGTERM (Windows: taskkill) tới tiến trình trong runner.pid
```

`Ctrl-C` hay `ttm runner down` dừng runner; agent đang chạy bị huỷ. Nếu runner chạy dưới dịch vụ, dịch vụ sẽ khởi động lại
nó; chạy `ttm runner uninstall-service` để tắt hẳn.

## Dịch vụ nền: `ttm runner install-service`

```sh
ttm runner install-service
ttm --profile work runner install-service     # dịch vụ dùng profile work
```

| Hệ điều hành | Cài gì | Log |
| --- | --- | --- |
| macOS | LaunchAgent `~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist` (`RunAtLoad`, `KeepAlive`), nạp bằng `launchctl load -w` | `<cache>/runner.log` (`~/.cache/ttm/runner.log`) |
| Linux | `~/.config/systemd/user/ttm-runner.service` (`Restart=on-failure`), `systemctl --user enable --now` | `journalctl --user -u ttm-runner` |
| Windows | Task Scheduler `ttm-runner`, chạy khi đăng nhập (`schtasks /Create /SC ONLOGON`) | (không có file log) |

Dịch vụ chạy `<đường dẫn ttm> [--profile <tên>] runner up`; `--profile` chỉ được ghi khi bạn truyền nó tường minh.

Dịch vụ mang theo `PATH`: launchd và systemd khởi động dịch vụ với `PATH` tối giản (`/usr/bin:/bin:…`), thiếu `claude`,
`gh`, đôi khi cả `git`. Vì vậy plist (`EnvironmentVariables`) và unit (`Environment="PATH=…"`) ghi `PATH` của shell chạy
`install-service` (bỏ mục rỗng hoặc tương đối như `.`), cộng các thư mục còn thiếu trong `~/.local/bin`, `~/.cargo/bin`,
`/opt/homebrew/bin`, `/opt/homebrew/sbin`, `/usr/local/bin`, `/usr/bin`, `/bin`, `/usr/sbin`, `/sbin`. Lệnh in `PATH` đã ghi:

```text
Runner service installed: /Users/an/Library/LaunchAgents/com.toantranmei.ttm-runner.plist
PATH of the service: /Users/an/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/Users/an/.cargo/bin:/opt/homebrew/sbin:/usr/sbin:/sbin
```

Cài thêm công cụ vào thư mục khác sau này thì chạy lại `ttm runner install-service` từ shell đã có `PATH` mới (lệnh ghi đè
và nạp lại dịch vụ). Trên Windows, tác vụ chạy với môi trường của tài khoản (`PATH` đặt trong Windows), nên không cần gì
thêm. Linux cũng cần `loginctl enable-linger $USER` nếu muốn runner chạy khi bạn chưa đăng nhập.

### Gỡ dịch vụ: `ttm runner uninstall-service`

```sh
ttm runner uninstall-service
```

Dừng dịch vụ rồi gỡ nó: macOS `launchctl unload -w` và xoá plist; Linux `systemctl --user disable --now`, xoá unit,
`daemon-reload`; Windows `schtasks /End` rồi `schtasks /Delete /F /TN ttm-runner`. Kết quả:
`Runner service stopped and removed: <file>`, hoặc `No runner service installed.` khi chưa cài.

## Chạy một task ngay: `ttm agent`

```sh
ttm agent run AUR-12
```

```text
Run 0190f610-… for AUR-12 on branch aur-12-dang-nhap-google
[log] worktree /Users/an/.ttm/worktrees/aur/aur-12-dang-nhap-google on aur-12-dang-nhap-google from main
[log] agent started claude-sonnet-4-5
[thought] Mình sẽ đọc context của task trước.
[action] mcp__ttm__factory-get-context
[action] Bash: cargo test -p auth
[log] tests passed: cargo test -p auth
[response] Đã thêm đăng nhập Google, test đều qua.
Run 0190f610-…: succeeded
worktree: /Users/an/.ttm/worktrees/aur/aur-12-dang-nhap-google
pull request: https://github.com/an/aurora/pull/57
```

`agent run` đăng ký máy này nếu cần, xin server một job cho task (`factory:run`), chạy nó ngay ở foreground với cấu hình
`runner.toml` (vẫn phải khai báo `--repo`), in sự kiện và **thoát 0 chỉ khi run `succeeded`**. `Ctrl-C` huỷ run. Task đang
có run khác: `409 conflict` (`rule: run-active`).

```sh
ttm agent logs 0190f610-…            # sự kiện tới giờ
ttm agent logs 0190f610-… -f         # theo dõi, hỏi lại mỗi 2 giây (Ctrl-C để thoát)
ttm agent logs 0190f610-… --json     # mỗi sự kiện một dòng JSON
ttm agent stop 0190f610-…            # kill switch
```

```text
2026-11-04T05:31:02Z [log] agent started claude-sonnet-4-5
2026-11-04T05:31:09Z [action] Bash: cargo test -p auth
```

`agent stop` báo server dừng run; runner đang chạy nó nhận lệnh ở heartbeat kế tiếp (≤ 20 giây), giết tiến trình agent và
báo `cancelled`.

## Token mà runner dùng

| Token | Dùng cho | Lưu ở |
| --- | --- | --- |
| Token người dùng (`ttm login`) | Đăng ký runner, `ttm agent run`, báo kết quả merge (`auto_merge`) | Keyring `<profile>@<host>` (hoặc `TTM_TOKEN`) |
| Token runner (`ttm_job_`, 12 giờ) | Heartbeat, nhận job, gửi sự kiện, kết thúc run | Keyring `<profile>#runner@<host>`, không bao giờ từ `TTM_TOKEN` |
| Token của job (`ttm_job_`) | `ttm-mcp` của agent trong đúng run đó, MCP gateway | File MCP tạm (0600), xoá khi run xong |
| Token Claude cho job đồng đội (`claude setup-token`) | `CLAUDE_CODE_OAUTH_TOKEN` của job đồng đội | Keyring `<profile>#claude-team` |

Thu hồi runner: tab **Runner** hoặc **Token** trên `/me/ket-noi`.

## Mô hình bảo mật

- **Chỉ kết nối đi ra**; không mở cổng nào trên máy bạn.
- **Chỉ repo bạn khai báo**, kiểm `origin` từng job; không clone URL lạ; tên nhánh được kiểm trước khi dùng.
- **Mỗi task một worktree và một nhánh**; worktree của bạn không bị đụng. Hook `pre-bash` chặn push thẳng vào nhánh được bảo
  vệ, `--no-verify`, commit sai quy ước. Worktree luôn nằm dưới `worktree_root`.
- **Token của job** chỉ dùng được trong run của nó và hết hạn theo job; nó nằm trong file 0600 và bị xoá sau run.
- **Agent chạy với quyền người dùng hệ điều hành của bạn**, giới hạn bởi danh sách tool cho phép mà job gửi xuống. Đây
  không phải sandbox: chỉ chạy runner cho project và workspace bạn tin, và xem lại PR trước khi merge.
- **Không dòng Claude/Anthropic** trong commit hay PR (hook `commit-msg` của run, `--settings`).
- **Skill chỉ là bản đã duyệt**, kiểm SHA-256, không bao giờ được commit, xoá sau run.
- **MCP local tắt mặc định**, chỉ cho agent bạn cho phép; server chỉ biết tên. Job của đồng đội không thấy cấu hình Claude
  cá nhân của bạn.
- **Agent đang chạy job không bao giờ có tool quản trị** (`factory:admin`): MCP của job bỏ toolset `admin`.
- Prompt cuối của mỗi bước đi qua máy bạn (để chạy Claude Code); server không bao giờ nhận key Claude của bạn.
