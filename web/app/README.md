# detonger-web-app

Chrome Web Bluetooth frontend for Detonger P2 printers.

## Requirements

- Chrome desktop (macOS first)
- Secure context (`https://` or `http://localhost`)
- `bun` + `wasm-pack`

## Development

```bash
bun install
bun run dev
```

## Scripts

- `bun run wasm:build`: build `crates/detonger-wasm` to `src/wasm/pkg`
- `bun run lint`: eslint + typecheck
- `bun run test`: vitest unit tests
- `bun run test:smoke`: playwright smoke tests (`?mockBle=1`)
- `bun run build`: production build

## Mock mode

Use querystring `?mockBle=1` to run without real hardware.

