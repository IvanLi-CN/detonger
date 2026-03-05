# detonger

Rust workspace + Web app for talking to **DeTong / Detonger** label printers over BLE.

## Supported / tested printers

- Tested: DeTong / Detonger P2 (德佟印立方 P2)
- Other models: not tested, may not work (protocol and GATT UUIDs may differ)

## Layout

- `crates/detonger-protocol`: protocol/encoding layer (transport agnostic)
- `crates/detonger-printer`: Rust library (BLE transport + printer operations)
- `crates/detonger-cli`: CLI binary (`detonger`)
- `crates/detonger-wasm`: Wasm bindings for protocol encoding
- `docs/specs/`: work-item specs (new canonical path)
- `docs/plan/`: legacy planning docs (read-only compatibility)
- `web/app`: Vite + React + TypeScript Web Bluetooth app
- `refs/`: local-only reverse engineering / legacy prototype (gitignored, never committed)

## macOS Bluetooth permission

On macOS, the terminal running `detonger` must have Bluetooth permission.

System Settings -> Privacy & Security -> Bluetooth -> enable your terminal app.

## Docs

- `docs/ble.md`: BLE/GATT UUIDs + macOS setup
- `docs/protocol.md`: protocol framing notes (header/bitmap/finalize)
- `docs/runbook.md`: manual validation steps (CLI + Web)

## CLI quick start

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

## Web app quick start

Requires Chrome desktop + secure context (`https://` or `http://localhost`).

```bash
cd web/app
bun install
bun run dev
```

Open `http://localhost:5173/?mockBle=1` for a mock-BLE demo without hardware.

### Web checks

```bash
cd web/app
bun run lint
bun run test
bun run test:smoke
bun run build
```

`bun run wasm:build` compiles `crates/detonger-wasm` and regenerates `web/app/src/wasm/pkg`.
