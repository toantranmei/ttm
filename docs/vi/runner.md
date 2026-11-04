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
```

Chạy lại `ttm runner up` (hoặc khởi động lại dịch vụ) để áp cấu hình mới.

### Vì sao phải khai báo repo

Runner **không bao giờ clone một URL server gửi tới**. Mỗi job nêu repo của nó; runner chỉ chạy khi:

1. project của job có clone trong `[repos]` (theo key, không phân biệt hoa thường, hoặc theo id);
2. `origin` của clone đó trùng repo của job (so sau khi chuẩn hoá: bỏ giao thức, user, cổng, `.git`, `/` cuối; ví dụ
   `git@github.com:a/b.git` = `https://github.com/a/b`); với project `local`, đường dẫn của job phải trùng đường dẫn đã
   khai báo;
3. tên nhánh của job hợp lệ theo luật của git.

Không thoả thì job bị từ chối và báo về server (`repo_not_configured` hoặc `invalid_branch`), terminal in:

```text
ttm runner: refused AUR-12: no local clone is configured for project AUR; add it with `ttm runner config --repo AUR=<path to your clone>`
```

Nhờ vậy một tài khoản bị chiếm hay một job giả không thể bắt máy bạn tải và chạy mã từ nơi lạ.

### Sức chứa

Số agent chạy cùng lúc = `--max-parallel` nếu đặt; nếu không, `clamp(số nhân CPU / 4, 1, 3)`, hạ xuống để mỗi agent có ít
nhất **4 GB RAM trống**. Server có thể giới hạn thấp hơn theo gói (Free 1, Pro 3, Team 6 mỗi runner). Khi agent báo hết hạn
mức sử dụng của Claude, runner **tạm dừng nhận job** tới giờ reset (run đó kết thúc với trạng thái `paused` kèm
`paused_until`), server xếp lại việc thay vì thử liên tục.

## Một job chạy thế nào

1. **Kiểm tra** repo và nhánh (mục trên).
2. **Worktree**: `git fetch origin <nhánh chính>`, rồi `git worktree add -B <nhánh> <đường dẫn> origin/<nhánh chính>`.
   Đường dẫn mặc định `<worktree_root>/<KEY>/<nhánh>` (ký tự lạ thành `-`).
3. **Luật**: chạy `ttm rules apply` trong worktree (khối `AGENTS.md`/`CLAUDE.md`, hook Claude Code).
4. **MCP**: ghi `<cache>/mcp-<run>.json` (quyền 0600) trỏ tới `ttm-mcp --toolsets <nhóm của job, mặc định factory,notee>`
   với `TTM_TOKEN` là token của job, `TTM_RUN_ID`, `TTM_TASK`, `TTM_PROJECT`.
5. **Agent**: `claude -p <prompt> --output-format stream-json --verbose --mcp-config <file> [--model <m>]
   [--max-turns <n>] [--allowedTools <danh sách>]` trong worktree; model, số lượt và tool cho phép do job quy định. Thời
   gian tối đa theo job, mặc định **120 phút**.
6. **Sự kiện**: mỗi dòng stream-json thành một sự kiện gửi lên theo lô (mỗi giây hoặc 25 sự kiện):

   | Claude Code | Sự kiện |
   | --- | --- |
   | khởi động | `log` (`agent started <model>`) |
   | văn bản của agent | `thought` |
   | gọi tool | `action` (`TodoWrite` → `plan`) |
   | kết quả cuối | `response`, kèm số token vào/ra |
   | lệnh test (`cargo test`, `pnpm test`, `pytest`, `go test`, `vitest`, `playwright test`…) | `log` `tests passed`/`tests failed`; kết quả được đính vào run như `test_result` |

7. **Kết thúc**: nếu thành công, commit phần agent để lại (`chore(<key>): changes from agent run <run>`), push nhánh, mở
   PR bằng `gh pr create` nếu có `gh`, rồi báo trạng thái (`succeeded`, `failed`, `cancelled`, `paused`), commit, PR, token
   đã dùng. File MCP tạm bị xoá.

Worktree của job đã xong được giữ `keep_days` ngày để bạn xem lại, rồi bị xoá khi `ttm runner up` khởi động lần sau.

### Kết nối với server

- Runner chỉ mở kết nối **đi ra** (chạy được sau NAT, không cần mở cổng): heartbeat mỗi 20 giây và long-poll job mỗi 15
  giây khi còn chỗ trống.
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
runner:    macbook-an (0190f500-…)
running:   yes
capacity:  2
harnesses: claude-code
repo:      AUR = /Users/an/code/aurora
repo:      WEB = /Users/an/code/web
```

```sh
ttm runner down     # gửi SIGTERM (Windows: taskkill) tới tiến trình trong runner.pid
```

`Ctrl-C` hay `ttm runner down` dừng runner; agent đang chạy bị huỷ. Nếu runner chạy dưới dịch vụ, dịch vụ sẽ khởi động lại
nó; gỡ dịch vụ để tắt hẳn.

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

> **Lưu ý:** dịch vụ không thừa hưởng `PATH` của shell. Nếu log báo `no agent harness found; install Claude Code (claude)
> first` dù `claude` chạy được trong terminal, thêm `PATH` cho dịch vụ. macOS: thêm vào plist rồi nạp lại
>
> ```xml
> <key>EnvironmentVariables</key>
> <dict>
>   <key>PATH</key>
>   <string>/Users/an/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin</string>
> </dict>
> ```
>
> Linux: `systemctl --user edit ttm-runner` và thêm `Environment=PATH=…` trong `[Service]`. Linux cũng cần
> `loginctl enable-linger $USER` nếu muốn runner chạy khi bạn chưa đăng nhập.

### Gỡ dịch vụ

```sh
# macOS
launchctl unload -w ~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist
rm ~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist

# Linux
systemctl --user disable --now ttm-runner.service
rm ~/.config/systemd/user/ttm-runner.service && systemctl --user daemon-reload
```

```powershell
# Windows
schtasks /Delete /TN ttm-runner /F
```

## Chạy một task ngay: `ttm agent`

```sh
ttm agent run AUR-12
```

```text
Run 0190f610-… for AUR-12 on branch aur-12-dang-nhap-google
[log] worktree /Users/an/.ttm/worktrees/AUR/aur-12-dang-nhap-google on aur-12-dang-nhap-google from main
[log] agent started claude-sonnet-4-5
[thought] Mình sẽ đọc context của task trước.
[action] mcp__ttm__factory-get-context
[action] Bash: cargo test -p auth
[log] tests passed: cargo test -p auth
[response] Đã thêm đăng nhập Google, test đều qua.
Run 0190f610-…: succeeded
worktree: /Users/an/.ttm/worktrees/AUR/aur-12-dang-nhap-google
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
| Token của job (`ttm_job_`) | `ttm-mcp` của agent trong đúng run đó | File MCP tạm (0600), xoá khi run xong |

Thu hồi runner: tab **Runner** hoặc **Token** trên `/me/ket-noi`.

## Mô hình bảo mật

- **Chỉ kết nối đi ra**; không mở cổng nào trên máy bạn.
- **Chỉ repo bạn khai báo**, kiểm `origin` từng job; không clone URL lạ; tên nhánh được kiểm trước khi dùng.
- **Mỗi task một worktree và một nhánh**; worktree của bạn không bị đụng. Hook `pre-bash` chặn push thẳng `main`/`master`,
  `--no-verify`, commit sai quy ước.
- **Token của job** chỉ dùng được trong run của nó và hết hạn theo job; nó nằm trong file 0600 và bị xoá sau run.
- **Agent chạy với quyền người dùng hệ điều hành của bạn**, giới hạn bởi danh sách tool cho phép mà job gửi xuống. Đây
  không phải sandbox: chỉ chạy runner cho project và workspace bạn tin, và xem lại PR trước khi merge.
- Prompt cuối của mỗi bước đi qua máy bạn (để chạy Claude Code); server không bao giờ nhận key Claude của bạn.
