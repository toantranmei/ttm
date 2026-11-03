---
title: 安全
description: 令牌、权限范围、存储方式以及如何报告漏洞。
order: 7
---

# 安全

- **令牌前缀**:`ttm_pat_`(手动创建)、`ttm_oat_`(登录)、`ttm_ort_`(刷新)、`ttm_job_`(runner)。不要把令牌粘贴到聊天、
  issue 或提交中。一旦泄露,请立即在 `/me/ket-noi` 吊销。
- **权限范围(scope)**:`workspace:read`、`notee:read`、`notee:write`、`notee:comment`、`factory:read`、`factory:write`、
  `factory:run`、`offline_access`。实际权限是 scope 与你本人权限的交集。令牌可以限定在单个工作区。
- **有效期**:默认 30 天。刷新令牌每次使用都会轮换;重复使用旧的刷新令牌会吊销整组令牌。
- **存储**:操作系统钥匙串;不可用时使用权限 0600 的 `credentials.toml`。ttm 从不在日志中记录令牌;`ttm status` 只显示开头几个字符。
- **登录**:PKCE S256 和随机 `state`;回调页只监听 `127.0.0.1`。
- **更新**:`ttm self-update` 校验 SHA-256,安装了 `gh` 时还会验证 GitHub attestation。
- **开源**(MIT):调度逻辑运行在服务器端;`ttm` 只负责认证、调用 API 和在你的电脑上执行任务。

## 报告漏洞

请私下发送邮件至 `toantran.mei@gmail.com`(不要公开提交 issue)。我们会在 72 小时内回复。
