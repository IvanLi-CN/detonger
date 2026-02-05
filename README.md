# detonger

Rust workspace for talking to **DeTong / Detonger** label printers over BLE (macOS-first).

## Layout

- `crates/detonger-printer`: Rust library (BLE transport + protocol/encoding)
- `crates/detonger-cli`: CLI binary (`detonger`)
- `docs/plan/`: stage-gated plans (docs-first workflow)
- `refs/`: local-only reverse engineering / legacy prototype (gitignored, never committed)
- `web/`: reserved for a future web app (out of scope for current plan)

## macOS Bluetooth permission

On macOS, the terminal running `detonger` must have Bluetooth permission.

System Settings -> Privacy & Security -> Bluetooth -> enable your terminal app.

## Docs

- `docs/ble.md`: BLE/GATT UUIDs + macOS setup
- `docs/protocol.md`: protocol framing notes (header/bitmap/finalize)
- `docs/runbook.md`: manual validation steps (paper-saving)

## CLI

Scan for printers:

```bash
cargo run -q -p detonger -- scan --timeout-s 6
```

Generate a PNG preview (no printer needed):

```bash
cargo run -q -p detonger -- preview width-test --out /tmp/detonger-width-test.png
```

Print the width-test pattern:

```bash
cargo run -q -p detonger -- print width-test --device <device-id>
```

Print a PNG:

```bash
cargo run -q -p detonger -- print png --device <device-id> --png /path/to/file.png
```
