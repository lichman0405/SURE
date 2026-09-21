# SURE roadmap

[English](#english) · [中文](#中文)

This file is where SURE's development status lives. The README is for people who
want to install and use the tool; this page is for people who want to know where
the project stands and what comes after.

---

## English

### Where SURE is

**The v0.1 development plan is complete.** Every planned task — 204 of them,
across 18 phases — was implemented, verified and accepted, with the evidence for
each recorded in `progress/state.json`. `FINAL_REPORT.md` is the full account of
what was built and measured, and `progress/HANDOFF.md` and
`progress/DECISIONS.md` are the narrative record, including the corrections the
project made to its own claims along the way.

**v0.1.0 is the first release.** It is an honest v0.1, not a finished product:
the checker runs, the CLI works, and every verdict it prints is backed by
recorded evidence. What it can and cannot do is listed below and measured in
`FINAL_REPORT.md`. The machine-readable source of truth for the plan itself is
`tasks/phases.json` + `tasks/tasks.json`, with the acceptance evidence in
`progress/state.json`.

### What v0.1.0 deliberately does not do

These are limits by design or by missing external credentials, not bugs. Each is
stated in the release notes and in the documentation.

- **Checks never run your project's code.** Every check is inspection-only
  (`InspectOnly`); anything that would require executing the project is reported
  as *not checked* rather than attempted.
- **No AI model is consulted unless you configure one.** Out of the box there is
  no analysis provider, so the model-assessment stage of every run is marked
  NOT CHECKED — and a run with any stage not checked is never reported as clean.
  That is why a healthy project's first `sure check` exits `1`.
- **The binaries are not signed or notarized.** No Authenticode certificate and
  no Apple Developer ID is configured, so Windows may warn about an unknown
  publisher and macOS downloads meet Gatekeeper.
- **No package manager carries SURE.** No WinGet package, no npm package, no
  marketplace extension is published. The WinGet manifest renderer exists
  (`packaging/winget/`), but submitting it is a manual step that has not been
  taken.
- **The Linux artifact needs glibc 2.39 or newer**, because it is built on the
  current CI runner image. It does not run on Ubuntu 22.04 or Debian 12.
- **Two launcher defects are carried**, both measured and documented in
  `progress/DECISIONS.md`: under Windows PowerShell 5.1 the hook write path
  stores nothing, and Codex's declared `-File` invocation mangles a non-ASCII
  payload.
- **The Copilot integration is a template.** Nothing installs it.
- **v0.1 deliberately includes no team dashboard, no hosted model service, no
  multi-agent scheduler and no production deployment verifier.**

### What comes next

No dates are promised here — the list is the order of intent, not a schedule.

1. Lower the Linux artifact's glibc floor (build inside an older container).
2. Code signing and notarization, if the credentials for them appear.
3. Publish the WinGet package once the release artifact has settled.
4. Complete the Copilot integration package.
5. Model-provider setup UX, so configuring an analysis provider is a guided step
   rather than a file you write by hand.
6. The longer-term directions sketched in `docs/product/` — including team use —
   stay sketches until the single-user tool has earned them.

### How releases work

Semantic versioning from the first public release (`VERSIONING.md`). The release
process — what an archive contains, how the checksums work, what the release
workflow does and refuses to do — is `docs/development/RELEASE_PROCESS.md`.
Releases are created as drafts by automation and published by a person.

---

## 中文

### SURE 现在到哪了

**v0.1 开发计划已全部完成。** 计划内的 204 项任务、18 个阶段，逐项做完、验证并验收，
每项的证据都记录在 `progress/state.json` 里。完整说明在 `FINAL_REPORT.md`；
过程记录（包括项目纠正过自己哪些说法）在 `progress/HANDOFF.md` 和
`progress/DECISIONS.md`。

**v0.1.0 是第一个发布版本。** 它是一个诚实的 v0.1，不是成品：检查器能跑，命令行能用，
打印出来的每个结论都有记录的证据。它能做什么、不能做什么，下面逐条列出，
`FINAL_REPORT.md` 里有带测量的完整版本。计划本身的机器可读源是
`tasks/phases.json` + `tasks/tasks.json`，验收证据在 `progress/state.json`。

### v0.1.0 有意不做的事

这些是设计上的边界或缺外部凭据，不是缺陷。发布说明和文档里都写明了。

- **检查永远不会运行你的项目代码。** 所有检查都是只读检查（InspectOnly）；
  需要执行项目才能查的，一律标记为"未检查"，不会尝试。
- **不配置 AI 模型就不用模型。** 默认没有分析提供方，每次运行的"模型评估"阶段
  都标记为"未检查"——而有任何阶段未检查的运行，永远不会被判为干净。
  所以一个健康项目第一次跑 `sure check` 会返回 `1`，这是正常的。
- **二进制没有签名、没有公证。** 没有配置 Authenticode 证书，也没有 Apple
  Developer ID，所以 Windows 可能提示未知发布者，macOS 下载会遇到 Gatekeeper。
- **没有包管理器收录 SURE。** 没有 WinGet 包、npm 包、市场扩展。WinGet 清单的
  渲染器已有（`packaging/winget/`），但提交是人工步骤，还没做。
- **Linux 安装包要求 glibc 2.39 或更新**，因为它在当前 CI 镜像上构建。
  Ubuntu 22.04、Debian 12 上跑不了。
- **带着两个启动器缺陷**，均已测量并记录在 `progress/DECISIONS.md`：
  Windows PowerShell 5.1 下 hook 写入路径存不下数据；Codex 声明的 `-File`
  调用方式会弄坏非 ASCII 负载。
- **Copilot 集成只是模板**，装不了。
- **v0.1 有意不包含**团队仪表盘、托管模型服务、多智能体调度器和生产部署验证器。

### 接下来做什么

这里不承诺日期——下面是意向顺序，不是排期。

1. 降低 Linux 安装包的 glibc 下限（在更旧的容器里构建）。
2. 如果拿到凭据，做代码签名和公证。
3. 发布产物稳定后发布 WinGet 包。
4. 补完 Copilot 集成包。
5. 模型提供方的配置引导，让配模型成为有引导的步骤，而不是手写配置文件。
6. `docs/product/` 里的长期方向（包括团队使用）在单用户工具立住之前保持草案。

### 发布怎么做

第一个公开发布起用语义化版本（`VERSIONING.md`）。发布流程——安装包里有什么、
校验和怎么验证、发布工作流做什么和拒绝做什么——在
`docs/development/RELEASE_PROCESS.md`。发布由自动化建为草稿，由人来发布。
