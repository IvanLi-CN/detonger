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

## CLI (planned contract)

See `docs/plan/cdwzw-rust-workspace-reinit/contracts/cli.md`.

