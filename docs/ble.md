# BLE / GATT Notes (DeTong / Detonger P2)

This document is a stable reference for the BLE parameters and macOS setup needed to talk to the
target label printer.

## Scope

- Target device: DeTong / Detonger P2 (macOS-first)
- Transport: BLE GATT write to a vendor service/characteristic
- Web MVP: Chrome desktop Web Bluetooth (secure context required)

## GATT UUIDs

- Service UUID: `49535343-fe7d-4ae5-8fa9-9fafd205e455`
- Write characteristic UUID: `49535343-8841-43f4-a8d4-ecbe34729bb3`
- Notify characteristic UUID: `49535343-1e4d-4bd9-ba61-23c647249616` (reserved; not required for basic printing)

## Device identifier (macOS)

On macOS, `detonger` uses CoreBluetooth peripheral UUID strings as the device id. You can obtain
them by running `detonger scan`.

## macOS Bluetooth permission

The terminal running `detonger` must have Bluetooth permission:

System Settings -> Privacy & Security -> Bluetooth -> enable your terminal app.

## CLI quickstart

Scan:

```bash
cargo run -q -p detonger -- scan --timeout-s 6
```

Generate a PNG preview (no printer needed, no paper used):

```bash
cargo run -q -p detonger -- preview width-test --out /tmp/detonger-width-test.png
```

Print the width-test pattern:

```bash
cargo run -q -p detonger -- print width-test --device <device-id>
```

## Debugging

Set `DETONGER_DEBUG=1` to print connect/discovery/write logs:

```bash
DETONGER_DEBUG=1 cargo run -q -p detonger -- print width-test --device <device-id>
```

Common failure modes:

- `timeout`: the OS or the printer didn't complete scan/connect/discovery within the timeout.
- `not found`: the target service/characteristic wasn't discovered (printer firmware or a transient BLE issue).

## Web Bluetooth notes

- Use Chrome desktop. Safari/Firefox are out of scope for this MVP.
- The page must run in a secure context: `https://` or `http://localhost`.
- `requestDevice()` requires user gesture; browser permission prompts are expected.
