# Protocol Notes (DeTong / Detonger P2)

This document summarizes the observed job encoding used by the printer. It is intentionally
high-level and practical: enough to implement and debug without depending on `refs/`.

## Transport boundary

- We send a job as a sequence of "vendor messages".
- Each vendor message is written to the GATT write characteristic as a single write.
- The job blob is split into vendor messages by parsing frame boundaries (DzPackage vs bitmap commands).

## Frame types

### 1) DzPackage frames (`0x1f`)

DzPackage frames have this structure:

```text
0x1f <cmd> <len...> <data...> 0x88
```

- `<len...>` uses EBV encoding:
  - 1 byte when `len <= 0xBF`
  - 2 bytes otherwise (`0xC0 | hi6`, then `lo8`)
- trailing `0x88` is a constant "crc" byte (observed in captures)

Common header commands:

- `0x20` page start: payload is `pageKey` as `u16` big-endian (e.g. `00 01`)
- `0x27` page width: payload is byte-width (`print_width_dots / 8`)
- `0x42` gap type: payload `02`
- `0x45` gap len: payload `ff ff`
- `0x43` darkness: payload `08`

### 2) Bitmap stream commands (`0x2b`)

Bitmap row frames do NOT include the trailing `0x88`. Observed form:

```text
0x1f 0x2b 0x00 <len_lo> <row-bytes...>
```

- `<len_lo>` is the row byte-width
- row bytes are MSB-left: dot `x=0` is bit 7 of byte 0

### 3) Raw bytes (e.g. `0x0c`)

Observed streams append a raw `0x0c` after the bitmap rows. In our splitter, raw bytes are appended
to the previous vendor message when possible.

## Job finalization (important)

There are optional DzPackage footer commands:

- `0x28` page end
- `0x21` page print

We default to NOT sending them because they were observed to cause a "double-advance / skip one label"
behavior on some firmwares. If the printer behavior changes in the future, we can make this
configurable behind an explicit option.

