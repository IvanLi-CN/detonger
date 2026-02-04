# Rust API（Library）

本文件定义 `detonger_printer` crate 的最小对外接口契约（供 CLI 与未来 Web/服务复用）。

## detonger_printer

- 范围（Scope）: internal
- 变更（Change）: New

## Public types（拟定）

### DeviceId

- 表示一个可连接的 BLE 设备标识。
- 兼容性：
  - macOS：预期为 CoreBluetooth 的 UUID 字符串（形如 `XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX`）。

### DiscoveredDevice

- Fields:
  - `id: DeviceId`
  - `name: Option<String>`
  - `rssi: Option<i16>`

### PrinterCaps

- Fields:
  - `dpi: u16`（default: 203）
  - `print_width_dots: u16`（default: 384）

### PrintOptions

- Fields:
  - `threshold: u8`（default: 150）
  - `x_offset_dots: i16`（default: 0）
  - （预留）位图压缩/兼容性扩展：不在本计划范围内；若后续需要，将以新计划增量引入

### Error / Result

- `pub type Result<T> = std::result::Result<T, Error>;`
- `Error` 至少应区分：
  - `InvalidArgument`
  - `Ble`（底层 BLE 错误）
  - `Timeout`
  - `NotFound`
  - `PrinterNotAvailable`（忙/缺纸/盖子开等）
  - `Protocol`（协议解析/编码错误）

## Public functions（拟定）

### Discovery

```rust
pub async fn scan(timeout: std::time::Duration) -> Result<Vec<DiscoveredDevice>>;
```

- 行为：在 `timeout` 内扫描 BLE 设备，返回候选列表；不得 panic。

### Connect

```rust
pub async fn connect(device: &DeviceId) -> Result<PrinterConnection>;
```

- 行为：连接目标设备并准备好写入；若设备不可连接，返回 `NotFound` 或 `Ble`。

### Print PNG

```rust
impl PrinterConnection {
    pub async fn print_png(&mut self, png: &[u8], opts: &PrintOptions) -> Result<()>;
}
```

- 行为：
  - 输入为 PNG 原始字节（由调用方读取文件）；库负责解析 PNG、阈值化并编码为打印机可执行的 vendor messages。
  - 库必须保证“不会因为重复结束指令导致额外走纸/跳纸”的行为（以当前已验证设备为基准）。
  - 对于不可打印状态（缺纸/盖子开等），返回 `PrinterNotAvailable`。

### Print test pattern

```rust
impl PrinterConnection {
    pub async fn print_width_test(&mut self, caps: &PrinterCaps, opts: &PrintOptions) -> Result<()>;
}
```

- 行为：生成并打印一个“低耗材”的宽度/定位测试图（高度可控、避免整行满黑），用于校准宽度与水平偏移。

## Protocol/transport constraints（实现约束）

- BLE/GATT：写入与 notify 的 service/characteristic UUID 必须集中定义（并可在需要时暴露为高级配置），避免散落。
- 发送策略：对目标设备应遵循“按 vendor message 边界写入”（避免把一行位图数据切碎到不同 GATT write）。
