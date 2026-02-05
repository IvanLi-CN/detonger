# Runbook: Manual validation (DeTong / Detonger P2)

This runbook is designed to validate printing with minimal paper usage.

## Preconditions

- Printer is powered on and has label paper loaded.
- Your terminal app has macOS Bluetooth permission (see `docs/ble.md`).

## Steps

1) Scan and copy device id

```bash
cargo run -q -p detonger -- scan --timeout-s 6
```

2) Preview the width-test PNG (no printer involved)

```bash
cargo run -q -p detonger -- preview width-test --out /tmp/detonger-width-test.png
```

3) Print the width-test pattern once

```bash
cargo run -q -p detonger -- print width-test --device <device-id>
```

Expected:

- The output matches the preview (frame + top/bottom ruler ticks).
- After printing, the printer does NOT advance an extra whole label.

4) Adjust horizontal alignment with `--x-offset` (dots)

Print at most once per offset change:

```bash
cargo run -q -p detonger -- print width-test --device <device-id> --x-offset -8
```

Expected:

- `x-offset` changes shift the pattern horizontally in a predictable way.

## Debugging

Enable debug logs:

```bash
DETONGER_DEBUG=1 cargo run -q -p detonger -- print width-test --device <device-id>
```

