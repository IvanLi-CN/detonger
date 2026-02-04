use crate::{Error, PrintOptions, PrinterCaps, Result};

use super::split::split_vendor_messages;

const SYNC: u8 = 0x1F;
const DZPKG_CRC_CONST: u8 = 0x88;

// Header commands (DzPackage).
const CMD_PAGE_START: u8 = 0x20;
const CMD_PAGE_WIDTH: u8 = 0x27;
const CMD_GAP_TYPE: u8 = 0x42;
const CMD_DARKNESS: u8 = 0x43;
const CMD_GAP_LEN: u8 = 0x45;

// Footer commands (DzPackage, optional).
const CMD_PAGE_END: u8 = 0x28;
const CMD_PAGE_PRINT: u8 = 0x21;

// Bitmap stream commands (not DzPackage, no trailing 0x88).
const CMD_BITMAP_PRINT: u8 = 0x2B;

/// Optional job-finalization frames.
///
/// IMPORTANT: Default is to NOT send them due to observed double-advance issues.
#[derive(Debug, Clone, Copy)]
pub struct FinalizeMode {
    pub include_page_end: bool,
    pub include_page_print: bool,
}

impl Default for FinalizeMode {
    fn default() -> Self {
        Self {
            include_page_end: false,
            include_page_print: false,
        }
    }
}

/// Encode a PNG into vendor messages (ready to be sent as BLE writes).
pub fn encode_png_job_messages(
    png: &[u8],
    caps: &PrinterCaps,
    opts: &PrintOptions,
) -> Result<Vec<Vec<u8>>> {
    let rows = rasterize_png_to_rows(png, caps, opts)?;
    encode_bitmap_job_messages(&rows, caps, 1, FinalizeMode::default())
}

/// Generate a low-power width/alignment test pattern and encode it into vendor messages.
pub fn encode_width_test_job_messages(
    caps: &PrinterCaps,
    opts: &PrintOptions,
) -> Result<Vec<Vec<u8>>> {
    let rows = generate_width_test_rows(caps, opts)?;
    encode_bitmap_job_messages(&rows, caps, 1, FinalizeMode::default())
}

/// Build a single-job byte stream, then split it into vendor messages.
///
/// The output is ready for "1 vendor message per GATT write" sending.
pub fn encode_bitmap_job_messages(
    rows: &[Vec<u8>],
    caps: &PrinterCaps,
    page_key: u16,
    finalize: FinalizeMode,
) -> Result<Vec<Vec<u8>>> {
    let payload = encode_bitmap_job_payload(rows, caps, page_key, finalize)?;
    split_vendor_messages(&payload)
}

/// Build a single-job byte stream (concatenated frames + bitmap commands).
///
/// Most callers should use [`encode_bitmap_job_messages`] instead.
pub fn encode_bitmap_job_payload(
    rows: &[Vec<u8>],
    caps: &PrinterCaps,
    page_key: u16,
    finalize: FinalizeMode,
) -> Result<Vec<u8>> {
    let byte_width = byte_width(caps.print_width_dots)?;

    for (idx, row) in rows.iter().enumerate() {
        if row.len() != byte_width {
            return Err(Error::InvalidArgument(format!(
                "row[{idx}] byte width mismatch: got={} want={byte_width}",
                row.len()
            )));
        }
    }

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&encode_header_payload(caps, page_key)?);

    for row in rows {
        out.extend_from_slice(&encode_bitmap_row(row)?);
    }

    // The observed lpapi-ble stream appends a raw 0x0c after the bitmap stream.
    out.push(0x0c);

    if finalize.include_page_end {
        out.extend_from_slice(&encode_dzpkg(CMD_PAGE_END, &[])?);
    }
    if finalize.include_page_print {
        out.extend_from_slice(&encode_dzpkg(CMD_PAGE_PRINT, &[])?);
    }

    Ok(out)
}

fn encode_header_payload(caps: &PrinterCaps, page_key: u16) -> Result<Vec<u8>> {
    let byte_width = byte_width(caps.print_width_dots)?;
    let byte_width_u8: u8 = byte_width
        .try_into()
        .map_err(|_| Error::InvalidArgument("print_width_dots is too large".into()))?;

    let mut out = Vec::new();

    // CMD_PAGE_START: pageKey as u16 BE (pageKey=1 => 00 01)
    out.extend_from_slice(&encode_dzpkg(CMD_PAGE_START, &page_key.to_be_bytes())?);

    // CMD_PAGE_WIDTH: byte width (dots/8)
    out.extend_from_slice(&encode_dzpkg(CMD_PAGE_WIDTH, &[byte_width_u8])?);

    // CMD_GAP_TYPE: 02
    out.extend_from_slice(&encode_dzpkg(CMD_GAP_TYPE, &[0x02])?);

    // CMD_GAP_LEN: ff ff
    out.extend_from_slice(&encode_dzpkg(CMD_GAP_LEN, &[0xff, 0xff])?);

    // CMD_DARKNESS: 08
    out.extend_from_slice(&encode_dzpkg(CMD_DARKNESS, &[0x08])?);

    Ok(out)
}

fn encode_dzpkg(cmd: u8, data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(2 + 2 + data.len() + 1);
    out.push(SYNC);
    out.push(cmd);
    encode_ebv_len(data.len(), &mut out)?;
    out.extend_from_slice(data);
    out.push(DZPKG_CRC_CONST);
    Ok(out)
}

fn encode_ebv_len(len: usize, out: &mut Vec<u8>) -> Result<()> {
    // Match replay_ble.py EBV decoding exactly:
    // - EBV1: single byte when len0 < 0xC0 (0..=0xBF)
    // - EBV2: two bytes otherwise, where data_len = ((len0 & 0x3F) << 8) | next
    if len <= 0xBF {
        out.push(len as u8);
        return Ok(());
    }
    if len > 0x3FFF {
        return Err(Error::InvalidArgument(format!(
            "DzPackage len too large for EBV2: {len}"
        )));
    }
    let hi = ((len >> 8) & 0x3F) as u8;
    let lo = (len & 0xFF) as u8;
    out.push(0xC0 | hi);
    out.push(lo);
    Ok(())
}

fn encode_bitmap_row(row_bytes: &[u8]) -> Result<Vec<u8>> {
    if row_bytes.len() > 0xFF {
        return Err(Error::InvalidArgument(format!(
            "bitmap row too wide for 0x2b 2-byte-len encoding: len={}",
            row_bytes.len()
        )));
    }
    // Use 2-byte big-endian length with a 0 high byte (matches observed captures).
    let len_lo = row_bytes.len() as u8;
    let mut out = Vec::with_capacity(4 + row_bytes.len());
    out.push(SYNC);
    out.push(CMD_BITMAP_PRINT);
    out.push(0x00);
    out.push(len_lo);
    out.extend_from_slice(row_bytes);
    Ok(out)
}

fn byte_width(print_width_dots: u16) -> Result<usize> {
    if print_width_dots % 8 != 0 {
        return Err(Error::InvalidArgument(format!(
            "print_width_dots must be divisible by 8: got={print_width_dots}"
        )));
    }
    Ok((print_width_dots / 8) as usize)
}

fn rasterize_png_to_rows(
    png: &[u8],
    caps: &PrinterCaps,
    opts: &PrintOptions,
) -> Result<Vec<Vec<u8>>> {
    let img = image::load_from_memory(png).map_err(|e| Error::Image(format!("png decode: {e}")))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let width_dots = caps.print_width_dots as i32;
    let byte_width = byte_width(caps.print_width_dots)?;

    let mut rows: Vec<Vec<u8>> = Vec::with_capacity(h as usize);
    for y in 0..h {
        let mut row = vec![0u8; byte_width];
        for x in 0..w {
            let px = rgba.get_pixel(x, y).0;
            let r = px[0] as u32;
            let g = px[1] as u32;
            let b = px[2] as u32;
            let a = px[3] as u32;

            // Approx luminance in 0..=255 (ITU-ish weights).
            let lum = (r * 77 + g * 150 + b * 29) >> 8;
            // Composite onto white when alpha < 255.
            let lum = (lum * a + 255 * (255 - a)) / 255;

            let is_black = lum < (opts.threshold as u32);
            if !is_black {
                continue;
            }

            let dest_x = x as i32 + (opts.x_offset_dots as i32);
            if dest_x < 0 || dest_x >= width_dots {
                continue;
            }
            set_dot_msb_left(&mut row, dest_x as usize);
        }
        rows.push(row);
    }
    Ok(rows)
}

fn generate_width_test_rows(caps: &PrinterCaps, opts: &PrintOptions) -> Result<Vec<Vec<u8>>> {
    let w = caps.print_width_dots as i32;
    let byte_width = byte_width(caps.print_width_dots)?;
    let height: i32 = 64;
    let x_off = opts.x_offset_dots as i32;

    let minor_step: i32 = 8; // ~1mm @203dpi
    let mid_step: i32 = 40; // ~5mm @203dpi
    let major_step: i32 = 80; // ~10mm @203dpi

    let minor_len: i32 = std::cmp::min(8, height - 2);
    let mid_len: i32 = std::cmp::min(12, height - 2);
    let major_len: i32 = std::cmp::min(16, height - 2);

    // Pre-allocate rows and then paint in-place to mirror the legacy "PNG width-test" output:
    // - safe frame (vertical edges + corner segments)
    // - ruler ticks at top & bottom
    let mut rows: Vec<Vec<u8>> = vec![vec![0u8; byte_width]; height as usize];

    // Safe frame: vertical edges (low per-row dot count).
    for y in 0..height {
        let row = &mut rows[y as usize];
        set_dot_msb_left_checked(row, w, 0 + x_off);
        set_dot_msb_left_checked(row, w, (w - 1) + x_off);
    }

    // Corner segments: avoid full-width solid horizontal lines.
    let corner_len: i32 = 16;
    for dx in 0..corner_len {
        let x_l = dx;
        let x_r = w - corner_len + dx;
        set_dot_msb_left_checked(&mut rows[0], w, x_l + x_off);
        set_dot_msb_left_checked(&mut rows[0], w, x_r + x_off);
        set_dot_msb_left_checked(&mut rows[(height - 1) as usize], w, x_l + x_off);
        set_dot_msb_left_checked(&mut rows[(height - 1) as usize], w, x_r + x_off);
    }

    // Ticks at top & bottom. Major ticks are thicker so they are easier to measure by eye.
    for x in 0..w {
        let (len, thick) = if x % major_step == 0 {
            (major_len, 2)
        } else if x % mid_step == 0 {
            (mid_len, 1)
        } else if x % minor_step == 0 {
            (minor_len, 1)
        } else {
            continue;
        };

        for dx in 0..thick {
            // Top tick: y=0..=len
            for y in 0..=len {
                set_dot_msb_left_checked(&mut rows[y as usize], w, x + dx + x_off);
            }
            // Bottom tick: y=(height-1-len)..=(height-1)
            let y0 = (height - 1) - len;
            for y in y0..height {
                set_dot_msb_left_checked(&mut rows[y as usize], w, x + dx + x_off);
            }
        }
    }

    Ok(rows)
}

fn set_dot_msb_left(row_bytes: &mut [u8], x: usize) {
    let byte_idx = x / 8;
    let bit = 7 - (x % 8);
    row_bytes[byte_idx] |= 1u8 << bit;
}

fn set_dot_msb_left_checked(row_bytes: &mut [u8], width_dots: i32, x: i32) {
    if x < 0 || x >= width_dots {
        return;
    }
    set_dot_msb_left(row_bytes, x as usize);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_test_messages_fit_default_ble_chunk() -> Result<()> {
        let caps = PrinterCaps::default();
        let opts = PrintOptions::default();

        let messages = encode_width_test_job_messages(&caps, &opts)?;
        assert!(!messages.is_empty());

        for (idx, msg) in messages.iter().enumerate() {
            assert!(
                msg.len() <= 180,
                "message #{idx} too large for chunk=180: len={}",
                msg.len()
            );
        }

        // Ensure default behavior: do not emit end/print DzPackage frames.
        assert!(
            !messages
                .iter()
                .any(|m| m.starts_with(&[0x1f, CMD_PAGE_END]))
        );
        assert!(
            !messages
                .iter()
                .any(|m| m.starts_with(&[0x1f, CMD_PAGE_PRINT]))
        );
        Ok(())
    }
}
