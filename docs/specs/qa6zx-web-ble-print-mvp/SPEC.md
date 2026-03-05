# Detonger 纯 Web BLE 打印 MVP（Style-Playbook）（#qa6zx）

## 状态

- Status: 已完成
- Created: 2026-03-05
- Last: 2026-03-05

## 背景 / 问题陈述

- 当前仓库已经有 Rust CLI（BLE 连接 + 打印协议），但 `web/` 目录仅占位，尚无可运行网页入口。
- 目标用户需要在 Chrome 桌面环境中直接用网页控制 Detonger P2 打印，不希望依赖本地桥接服务。
- 若不补齐 Web MVP，协议能力只能通过 CLI 使用，无法验证“浏览器端 BLE 控制”这一关键交付价值。

## 目标 / 非目标

### Goals

- 新建 `web/app`，提供可运行的 Vite + React + TypeScript 网页打印工具。
- 将协议编码能力抽离为可复用 Rust crate，并提供 Wasm 导出给 Web 端。
- 实现 Web Bluetooth 连接、会话复用、文本打印、PNG 打印、错误提示。
- 建立最小质量门禁：Rust + Web 测试与构建链路均可自动验证。

### Non-goals

- 不支持 Safari/Firefox/iOS。
- 不支持非 P2 型号自动探测或协议适配。
- 不引入云端服务、账号体系、后台静默任务。

## 范围（Scope）

### In scope

- Rust: `detonger-protocol` 与 `detonger-wasm` crate。
- Web: `web/app` React SPA + Web Bluetooth 客户端。
- 文档：README、runbook/spec 同步。
- CI：新增最小 GitHub Actions 校验工作流。

### Out of scope

- 生产域名/HTTPS 证书部署。
- 打印模板管理、批量任务队列。

## 需求（Requirements）

### MUST

- 提供“连接打印机 -> 打印文本 -> 打印 PNG”完整路径。
- 复用 Rust 协议编码逻辑，避免前后端协议实现漂移。
- 每条 vendor message 必须独立执行一次 BLE write。
- Web 端必须展示连接状态与错误信息。
- 提供 Vitest + Playwright smoke（可在 mock BLE 下运行）。

### SHOULD

- UI 显示应用版本信息与运行环境提示。
- 支持断连后显式重连。

### COULD

- 提供 width-test 预览入口。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 用户点击连接按钮，触发 Chrome `requestDevice`，选择 Detonger 设备后建立 GATT 连接。
- 用户输入文本并打印：前端将文本渲染为位图 PNG，调用 Wasm 编码后逐包写入蓝牙特征。
- 用户上传 PNG 并打印：前端读取 PNG bytes，调用 Wasm 编码后逐包写入。
- 同一连接可重复执行多次打印，直到用户主动断开或连接丢失。

### Edge cases / errors

- 用户拒绝授权：提示 `permission_denied`。
- 服务/特征 UUID 不匹配：提示 `service_not_found` / `char_not_found`。
- 写入中断：提示 `write_failed` 并保留重试入口。
- 非 Chrome 或非安全上下文：页面显示不可用原因。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `detonger-protocol` | Rust API | internal | New | 本文档内定义 | detonger | CLI / Wasm | 纯协议编码，不依赖 BLE |
| `detonger-wasm` | Wasm API | internal | New | 本文档内定义 | detonger | `web/app` | `wasm-bindgen` 导出 |
| `WebBlePrinterClient` | TypeScript API | internal | New | 本文档内定义 | detonger | React UI | Web Bluetooth 适配层 |

### 契约文档（按 Kind 拆分）

None（本阶段直接在 SPEC 内冻结接口，后续如复杂化再拆分 `contracts/`）。

## 验收标准（Acceptance Criteria）

- Given Chrome 桌面环境
  When 用户点击连接并选择目标设备
  Then 页面显示 `connected` 且可触发打印。

- Given 文本输入 + 参数（threshold/x-offset）
  When 用户执行打印
  Then 打印机输出可读内容，且偏移参数生效。

- Given PNG 文件输入
  When 用户执行打印
  Then 打印机按图片内容输出，无“只走纸不出字”。

- Given BLE mock 测试环境
  When 执行 `bun run test` 与 `bun run test:smoke`
  Then 关键流程测试通过。

## 实现前置条件（Definition of Ready / Preconditions）

- 目标平台与非目标已锁定。
- 协议复用策略（Rust -> Wasm）已锁定。
- 风险边界（仅 Chrome 桌面）已确认。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Rust unit tests：协议分包边界与编码回归。
- Web unit tests：状态机、错误映射、打印调度。
- Web smoke tests：页面加载、连接入口、文本/PNG 打印触发（mock BLE）。

### Quality checks

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `bun run lint`
- `bun run test`
- `bun run build`

## 文档更新（Docs to Update）

- `README.md`: 增加 Web MVP 用法。
- `web/README.md`: 更新为运行指引与能力边界。
- `docs/runbook.md`: 补充 Web 打印手工验收流程。

## 计划资产（Plan assets）

- Directory: `docs/specs/qa6zx-web-ble-print-mvp/assets/`
- None

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 建立 `docs/specs` 并落位本规格
- [x] M2: 抽离 `detonger-protocol` 并让 CLI 路径继续可用
- [x] M3: 新增 `detonger-wasm` 导出层
- [x] M4: 搭建 `web/app` 并实现 Web Bluetooth 打印核心链路
- [x] M5: 增补测试与 CI（Rust + Web + Playwright smoke）
- [x] M6: 完成文档同步与本地验收
- [x] M7: 远端 PR + checks + review-loop 收敛

## 方案概述（Approach, high-level）

- 在 Rust workspace 内保持“协议层/传输层分离”：编码在 `detonger-protocol`，BLE transport 保留在 `detonger-printer`。
- Wasm 只导出编码能力，不导出 BLE；BLE 由浏览器 Web Bluetooth API 负责。
- Web UI 采用可观测状态驱动，错误码统一映射后展示，避免“失败无反馈”。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：Web Bluetooth 浏览器兼容性有限；真机连接受系统权限影响。
- 需要决策的问题：None（本轮已锁定关键决策）。
- 假设（需主人确认）：执行阶段可使用 Detonger P2 真机做最终手工验收。

## 变更记录（Change log）

- 2026-03-05: 创建规格并冻结实现边界（Rust->Wasm + React/Vite + Bun + Vitest/Playwright smoke）。
- 2026-03-05: 完成 M1-M6，进入快车道远端收敛阶段（M7）。
- 2026-03-05: 完成 M7，PR `#1` 在最新提交 `80774ac` 上 checks 全绿，review-loop 收敛无阻塞项。

## 参考（References）

- `/Users/ivan/.style-playbook-skills/skills/style-playbook/references/projects/display-ambient-light-desktop.md`
- `/Users/ivan/.style-playbook-skills/skills/style-playbook/references/projects/paste-preset.md`
- `/Users/ivan/.style-playbook-skills/skills/style-playbook/references/projects/isolappurr-usb-hub.md`
