# Rust 工程重置：BLE 打印库 + CLI（#cdwzw）

## 状态

- Status: 待实现
- Created: 2026-02-04
- Last: 2026-02-04

## 背景 / 问题陈述

- 现有仓库包含一套原型实现（Node/Python）用于验证与“德佟印立方 P2”标签打印机的蓝牙通信与打印效果。
- 目标是以 Rust 为主语言重新初始化工程，沉淀可复用的 BLE 打印库，并提供一个对人友好的 CLI 入口。
- 未来可能在同一仓库加入 Web 程序，因此需要在目录结构上提前预留、不互相冲突。

## 目标 / 非目标

### Goals

- 将仓库重置为以 Rust 为核心的工程结构（Cargo workspace）。
- 提供一个 Rust library crate：可通过 BLE 与目标打印机通信，并提供“打印一张标签”的主流程能力。
- 提供一个 CLI 工具：尽量暴露库能力，并提供便捷的打印/测试命令。
- 将现有原型内容归档到本地 `refs/`，并在 Git 层面忽略（不纳入提交），以便从干净结构开始演进。
- 给未来 Web 程序预留空间（不在本计划实现 Web）。

### Non-goals

- 不在本计划交付 Web 程序（仅做结构预留）。
- 不承诺一次覆盖所有型号与所有协议特性；优先聚焦 P2 + 当前已验证的核心路径。
- 不在计划阶段实现/测试代码（实现需在切换到 `impl` 后进行）。

## 范围（Scope）

### In scope

- 新的仓库目录结构约定（Rust workspace + 预留未来 Web 的目录）。
- `refs/` 归档策略与 `.gitignore` 规则（确保不会被提交）。
- 现有原型内容的归档清单（实现阶段会整体移动到 `refs/legacy-prototype/`）：
  - `.venv/`
  - `artifacts/`
  - `captures/`
  - `downloads/`
  - `node_modules/`（可选：也可删除后按需重装；但一律不入库）
  - `notes/`
  - `scripts/`
  - `package.json` / `package-lock.json`
  - `README.md` / `.gitignore`（会在主工程重写；旧版保留在 `refs/` 供参考）
- Rust library crate（BLE 通信 + 打印能力）与 CLI crate 的接口契约冻结。
- 质量门槛定义（fmt/clippy/test、最小可回归的离线测试资产与手工验收流程）。
- 文档：README/基础使用说明/协议与调试笔记的落位（避免实现依赖 `refs/`）。

### Out of scope

- 发布/打包/安装分发（如 Homebrew、pkg、MSI）与自动更新。
- GUI/桌面应用（Tauri 等）。
- 云端服务与账号体系。

## 需求（Requirements）

### MUST

- 仓库初始化为 Cargo workspace，至少包含两个成员：
  - `crates/detonger-printer/`：Rust library crate（BLE 打印库）
  - `crates/detonger-cli/`：Rust binary crate（CLI）
- 现有原型内容移动到 `refs/` 并被 Git 忽略（不进入提交）；仓库提交中不包含任何 `refs/**`。
- 初期目标平台仅 macOS（不要求 Windows/Linux 可用）。
- 纯 Rust 实现：主工程（workspace）不依赖 Node/Python 运行时与相关库；原型仅作为本地 `refs/` 参考。
- CLI 必须能完成以下核心场景：
  - 连接（或发现后连接）目标打印机
  - 从 PNG（或等价输入）打印一张标签
  - 打印宽度/定位测试图（用于校准与验收）
- Rust library 必须对外提供稳定的最小 API（供 CLI 与未来 Web 复用），详见契约文档。
- 必须将已知的 BLE/GATT 参数与协议关键约束沉淀为提交内的文档（而不是依赖 `refs/` 中的原型代码）。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `detonger`（CLI binary） | CLI | external | New | ./contracts/cli.md | detonger | 人类用户 / CI | 对齐“开发者工具”风格：小而可验证 |
| `detonger_printer`（Rust crate） | Rust API | internal | New | ./contracts/rust-api.md | detonger | CLI / 未来 Web | 以可测试、可替换 transport 为边界 |

### 契约文档（按 Kind 拆分）

- [contracts/README.md](./contracts/README.md)
- [contracts/cli.md](./contracts/cli.md)
- [contracts/rust-api.md](./contracts/rust-api.md)

## 验收标准（Acceptance Criteria）

- Given 仓库已按本计划完成初始化
  When 运行基础质量检查（fmt/clippy/test）
  Then 全部通过，且默认路径不依赖任何 `refs/**` 内容

- Given 本地存在一个可用的目标打印机（P2）并已开机
  When 使用 CLI 执行“打印宽度测试图”
  Then 打印机打印出预期的测试图形，且打印结束后不会额外多走一整张标签纸

- Given 同一张测试图与不同的 `x-offset`（以 dots 为单位）
  When 连续打印 2 次（使用不同 offset）
  Then 观察到水平位置发生可解释的位移（用于校准），且不会改变纸张对齐策略导致跳纸

- Given 用户传入不可连接的设备标识 / 超时
  When 执行 CLI 打印
  Then 以非 0 退出码失败，并输出可定位问题的错误信息（包含错误分类与下一步建议）

- Given 本计划目录中的文档/资产
  When 完成实现与交付
  Then 项目运行/交付不依赖 `docs/plan/` 下任何路径（本计划 `资产晋升` 为 `None`）

## 实现前置条件（Definition of Ready / Preconditions）

- 目标/非目标、范围（in/out）、约束已明确
- 验收标准覆盖 core path + 关键边界/异常
- 接口契约已定稿，实现与测试可以直接按契约落地
- 关键取舍已确认：
  - 目标平台：macOS only
  - BLE crate：`btleplug`（Tokio runtime）
  - `refs/`：仅本地逆向/信息收集参考，不入库（Git ignore）
  - pure Rust：主工程不依赖 Node/Python

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests:
  - 协议拆包/切包（“一条 vendor message 对应一次 GATT write”）的边界用例
  - PNG -> 黑白位图/阈值处理的确定性测试（离线 golden）
- Integration tests:
  - 不强制依赖真实硬件（避免 CI 不可用）；对硬件交互以手工验收为主

### Quality checks

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`

## 文档更新（Docs to Update）

- `README.md`: 项目定位、开发环境、CLI 使用示例、支持的设备/平台范围
- （新增）`docs/`: BLE/GATT 参数、协议约束、调试说明（路径在实现阶段冻结为 `docs/` 下稳定位置）

## 计划资产（Plan assets）

- Directory: `docs/plan/cdwzw-rust-workspace-reinit/assets/`
- None

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones）

- [ ] M1: Repo re-init（仅保留 Rust workspace + docs；原型移动到 `refs/` 并 gitignore；预留 `web/` 目录位）
- [ ] M2: `detonger-printer`：BLE transport（scan/connect/write characteristic；macOS 权限说明落入 docs）
- [ ] M3: 协议层（构建 Dz frame + bitmap 命令；确保“按 vendor message 边界写入”不会出现“只走纸不出字”）
- [ ] M4: 编码层（PNG decode + threshold + x-offset；生成 width-test pattern；输出可打印 job）
- [ ] M5: `detonger` CLI（scan / print png / print width-test；human+json 输出；明确退出码）
- [ ] M6: 手工验收与回归说明（用一张标签完成宽度/偏移校准；确认不跳纸；写入 runbook）

## 方案概述（Approach, high-level）

- 目录结构采用 Rust workspace + 多语言扩展预留：
  - Rust crates 放入 `crates/`
  - 未来 Web 预留顶层目录：`web/`（本计划不实现，只占位避免冲突）
- BLE 通信层与协议/编码层分离：
  - transport 负责：扫描/连接/写入/（可选）notify
  - protocol/encoder 负责：把“要打印的内容”变成设备可执行的 vendor messages
- BLE transport 默认选择：`btleplug`（Tokio runtime）
- 为避免实现依赖 `refs/`：需要把关键协议常量（服务/特征 UUID、默认 DPI/宽度、已知结束走纸行为约束等）沉淀为提交内文档与代码常量。
  - 已验证的一条关键约束：避免发送“重复的结束/走纸指令”，否则可能导致标签对齐多走纸（跳纸）

### 已知设备常量（P2；用于实现与调试）

- DPI: 203
- Print width (dots): 384
- BLE GATT:
  - Service UUID: `49535343-fe7d-4ae5-8fa9-9fafd205e455`
  - Write characteristic UUID: `49535343-8841-43f4-a8d4-ecbe34729bb3`
  - Notify characteristic UUID: `49535343-1e4d-4bd9-ba61-23c647249616`

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：
  - BLE 在 macOS 的权限/系统行为：CLI 需要终端具备蓝牙权限；在不同 macOS 版本上行为可能有差异。
  - 协议缺少公开 spec，需要以现有可打印行为为准做对照测试。
- 假设：None（关键取舍已确认）

## 变更记录（Change log）

- 2026-02-04: 创建计划并冻结为 `待实现`（macOS-only；refs 不入库；pure Rust；CLI/Rust API 契约冻结）。

## 参考（References）

- `skills/style-playbook/references/projects/televy-backup.md`（Rust workspace）
- `skills/style-playbook/references/projects/display-ambient-light-desktop.md`（Rust + Web 混合结构示例）
- `skills/style-playbook/references/projects/isolappurr-usb-hub.md`（device mono-repo + `web/`/`tools/` 共存）
