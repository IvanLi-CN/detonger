# 命令行（CLI）

本计划的 CLI 目标是“尽量暴露库能力，同时提供便捷的打印/测试入口”。

## detonger

- 范围（Scope）: external
- 变更（Change）: New

### 用法（Usage）

```text
detonger <command> [options]
```

### 命令（Commands）

#### scan

```text
detonger scan [--timeout-s <sec>] [--format <human|json>]
```

- `--timeout-s`: 搜索超时（default: 5）
- `--format`: 输出格式（default: human）

#### print png

```text
detonger print png --device <id> --png <path> [--x-offset <dots>] [--threshold <0-255>]
```

- `--device`: 设备标识（macOS 上通常是 CoreBluetooth UUID 字符串）
- `--png`: PNG 文件路径
- `--x-offset`: 水平预偏移（dots；负值向左，default: 0）
- `--threshold`: 黑白阈值（default: 150）

说明：

- 本计划首版聚焦“可用且可回归”的打印路径；位图压缩/兼容性扩展若需要，将以新计划增量引入（不在本计划范围内）。

#### print width-test

```text
detonger print width-test --device <id> [--width <dots>] [--height <dots>] [--x-offset <dots>]
```

- 用于校准打印宽度与水平位置；默认宽度取目标打印头宽度。

### 输出（Output）

- Format: `human` 或 `json`
- `json`（示意）：
  - `scan`: `[{ "device": "...", "name": "...", "rssi": -60 }]`
  - `print`: `{ "status": "ok" }` 或 `{ "status": "error", "error": { ... } }`

### 退出码（Exit codes）

- `0`: 成功
- `2`: 参数/用法错误
- `10`: BLE/连接相关错误
- `20`: 打印机不可用（忙/缺纸/盖子开等可归类错误）
- `100`: 未分类错误

### 兼容性与迁移（Compatibility / migration）

- `--device` 形状在不同 OS 可能不同：本计划默认先以 macOS 为最小可用集；若扩展跨平台，需要在契约中追加约束与示例。
