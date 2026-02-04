use crate::{Error, Result};

const SYNC: u8 = 0x1F;
const DZPKG_CRC_CONST: u8 = 0x88;

const CMD_BITMAP_PRINT: u8 = 0x2B;
const CMD_BITMAP_REPEAT: u8 = 0x2E;

const BITMAP_CMDS_1LEN: [u8; 5] = [0x29, 0x2C, 0x2D, 0x3C, 0x3D];

fn try_dzpkg_end(payload: &[u8], i: usize) -> Option<usize> {
    let n = payload.len();
    if i + 4 > n || payload[i] != SYNC {
        return None;
    }
    let len0 = payload[i + 2];
    let (data_len, len_len) = if len0 >= 0xC0 {
        if i + 5 > n {
            return None;
        }
        let data_len = (((len0 & 0x3F) as usize) << 8) | (payload[i + 3] as usize);
        (data_len, 2usize)
    } else {
        (len0 as usize, 1usize)
    };

    let data_start = i + 2 + len_len;
    let end = data_start + data_len + 1; // trailing crc
    if end > n {
        return None;
    }
    if payload[end - 1] != DZPKG_CRC_CONST {
        return None;
    }
    Some(end)
}

/// Split an lpapi-ble job blob into write-friendly messages.
///
/// This matches `refs/legacy-prototype/scripts/macos/replay_ble.py::_split_vendor_messages`.
/// In particular:
/// - DzPackage frames end with a trailing `0x88` "crc" byte
/// - Bitmap row payloads (e.g. `0x2b`) do *not* have the trailing `0x88`
/// - Raw bytes (not starting with `0x1f`) are appended to the previous message when possible
pub fn split_vendor_messages(payload: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut i = 0usize;
    let n = payload.len();

    while i < n {
        if payload[i] != SYNC {
            // Raw segment until the next sync byte.
            let mut j = i + 1;
            while j < n && payload[j] != SYNC {
                j += 1;
            }
            let seg = &payload[i..j];
            if let Some(last) = out.last_mut() {
                last.extend_from_slice(seg);
            } else {
                out.push(seg.to_vec());
            }
            i = j;
            continue;
        }

        // Try DzPackage first.
        if let Some(end) = try_dzpkg_end(payload, i) {
            out.push(payload[i..end].to_vec());
            i = end;
            continue;
        }

        if i + 2 >= n {
            return Err(Error::Protocol(format!(
                "Truncated command header at offset={i}"
            )));
        }
        let cmd = payload[i + 1];

        let end = if cmd == CMD_BITMAP_REPEAT {
            i + 3
        } else if cmd == CMD_BITMAP_PRINT {
            if i + 4 > n {
                return Err(Error::Protocol(format!(
                    "Truncated 0x2b header at offset={i}"
                )));
            }
            // Prefer the 2-byte big-endian length when the high byte is 0
            // (common for bitmap-row data where len is 20/48 bytes).
            if payload[i + 2] == 0x00 {
                let data_len = ((payload[i + 2] as usize) << 8) | (payload[i + 3] as usize);
                i + 4 + data_len
            } else {
                let data_len = payload[i + 2] as usize;
                i + 3 + data_len
            }
        } else if BITMAP_CMDS_1LEN.contains(&cmd) {
            let data_len = payload[i + 2] as usize;
            i + 3 + data_len
        } else {
            // Unknown non-DzPackage command: keep a minimal prefix so we don't loop forever.
            i + 2
        };

        if end > n {
            return Err(Error::Protocol(format!(
                "Truncated cmd=0x{cmd:02x} at offset={i} need={end} have={n}"
            )));
        }
        out.push(payload[i..end].to_vec());
        i = end;
    }

    if out.is_empty() {
        return Err(Error::Protocol("No messages found in payload".into()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_hex(input: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = 0u8;
        let mut have_half = false;

        for ch in input.bytes() {
            let v = match ch {
                b'0'..=b'9' => ch - b'0',
                b'a'..=b'f' => ch - b'a' + 10,
                b'A'..=b'F' => ch - b'A' + 10,
                _ => continue,
            };
            if have_half {
                out.push((buf << 4) | v);
                have_half = false;
            } else {
                buf = v;
                have_half = true;
            }
        }
        assert!(!have_half, "odd number of hex digits");
        out
    }

    #[test]
    fn split_vendor_messages_matches_known_capture() -> Result<()> {
        // Derived from:
        // refs/legacy-prototype/captures/job_png_1770185605888_160x64.json (packet[0], base64-decoded).
        const PAYLOAD_HEX: &str = "\
            1f2002000188\
            1f27011488\
            1f42010288\
            1f4502ffff88\
            1f43010888\
            1f2b0014ffffffffffffffffffffffffffffffffffffffff\
            1f2b0014c0808080808080808080c0808080808080808081\
            1f2e06\
            1f2b0014c0000000008000000000c0000000008000000001\
            1f2e02\
            1f2b0014c0000000000000000000c0000000000000000001\
            1f2e02\
            1f2b00148000000000000000000000000000000000000001\
            1f2b001480000000000003c00003fc003ffc000000000001\
            1f2e02\
            1f2b00148000000000003fc0003c0003c003c00000000001\
            1f2e02\
            1f2b001480000000000003c003c00003c03fc00000000001\
            1f2e02\
            1f2b001480000000000003c003fffc03c3c3c00000000001\
            1f2e02\
            1f2b001480000000000003c003c003c3fc03c00000000001\
            1f2e02\
            1f2b001480000000000003c003c003c3c003c00000000001\
            1f2e02\
            1f2b00148000000000003ffc003ffc003ffc000000000001\
            1f2e02\
            1f2b00148000000000000000000000000000000000000001\
            1f2b0014c0000000000000000000c0000000000000000001\
            1f2e02\
            1f2b0014c0000000008000000000c0000000008000000001\
            1f2e02\
            1f2b0014c0808080808080808080c0808080808080808081\
            1f2e06\
            1f2b0014ffffffffffffffffffffffffffffffffffffffff\
            0c";

        let payload = decode_hex(PAYLOAD_HEX);
        let msgs = split_vendor_messages(&payload)?;

        assert_eq!(msgs.len(), 35);
        assert_eq!(msgs[0], vec![0x1f, 0x20, 0x02, 0x00, 0x01, 0x88]);
        assert_eq!(msgs[1], vec![0x1f, 0x27, 0x01, 0x14, 0x88]);
        assert_eq!(msgs[2], vec![0x1f, 0x42, 0x01, 0x02, 0x88]);
        assert_eq!(msgs[3], vec![0x1f, 0x45, 0x02, 0xff, 0xff, 0x88]);
        assert_eq!(msgs[4], vec![0x1f, 0x43, 0x01, 0x08, 0x88]);

        assert_eq!(&msgs[5][0..4], &[0x1f, 0x2b, 0x00, 0x14]);
        assert_eq!(msgs[5].len(), 24);
        assert!(msgs[5][4..].iter().all(|&b| b == 0xff));

        assert_eq!(msgs[7], vec![0x1f, 0x2e, 0x06]);

        let last = msgs.last().expect("non-empty");
        assert_eq!(&last[0..4], &[0x1f, 0x2b, 0x00, 0x14]);
        assert_eq!(last.len(), 25);
        assert_eq!(*last.last().unwrap(), 0x0c);

        // This particular capture doesn't include a raw-first segment.
        assert!(msgs.iter().all(|m| m.first() == Some(&0x1f)));
        Ok(())
    }

    #[test]
    fn split_vendor_messages_preserves_raw_first_segment() -> Result<()> {
        // Matches Python behavior: if the payload starts with raw bytes, they become the first message.
        let payload = [0x0c, 0x0c, 0x1f, 0x2e, 0x01];
        let msgs = split_vendor_messages(&payload)?;
        assert_eq!(msgs, vec![vec![0x0c, 0x0c], vec![0x1f, 0x2e, 0x01]]);
        Ok(())
    }
}

