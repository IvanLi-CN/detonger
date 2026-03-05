# Runbook: Manual validation (DeTong / Detonger P2)

This runbook is designed to validate printing with minimal paper usage.

## Preconditions

- Printer is powered on and has label paper loaded.
- CLI path: terminal app has macOS Bluetooth permission (see `docs/ble.md`).
- Web path: use Chrome desktop and run on `https://` or `http://localhost`.

## A) CLI validation (baseline)

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

## B) Web validation (MVP)

1) Start app

```bash
cd web/app
bun install
bun run dev
```

2) Open app (real device)

```text
http://localhost:5173
```

3) Click `连接打印机`, select Detonger device, wait for `connected` state.

4) Text print check

- Enter 1-2 short lines.
- Click `打印文本`.
- Expected: printer outputs readable text; logs show "文本打印完成".

5) PNG print check

- Upload a small PNG file.
- Click `打印 PNG`.
- Expected: printer outputs image content; logs show "PNG 打印完成".

6) Width-test from web

- Click `打印 width-test`.
- Expected: same alignment semantics as CLI width-test.

## Debugging

CLI debug logs:

```bash
DETONGER_DEBUG=1 cargo run -q -p detonger -- print width-test --device <device-id>
```

Web smoke path (no hardware):

```text
http://localhost:5173/?mockBle=1
```

Run automated smoke:

```bash
cd web/app
bun run test:smoke
```
