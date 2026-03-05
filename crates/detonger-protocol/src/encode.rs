use crate::{Error, PaperType, PrintOptions, PrinterCaps, Result};

use super::split::split_vendor_messages;

use std::io::Cursor;

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
#[derive(Debug, Clone, Copy, Default)]
pub struct FinalizeMode {
    pub include_page_end: bool,
    pub include_page_print: bool,
}

/// Encode a PNG into vendor messages (ready to be sent as BLE writes).
pub fn encode_png_job_messages(
    png: &[u8],
    caps: &PrinterCaps,
    opts: &PrintOptions,
) -> Result<Vec<Vec<u8>>> {
    let rows = rasterize_png_to_rows(png, caps, opts)?;
    encode_bitmap_job_messages(&rows, caps, opts, 1, FinalizeMode::default())
}

/// Generate a low-power width/alignment test pattern and encode it into vendor messages.
pub fn encode_width_test_job_messages(
    caps: &PrinterCaps,
    opts: &PrintOptions,
) -> Result<Vec<Vec<u8>>> {
    let rows = generate_width_test_rows(caps, opts)?;
    encode_bitmap_job_messages(&rows, caps, opts, 1, FinalizeMode::default())
}

/// Render the width-test pattern as a PNG (useful to preview what `print width-test` draws).
///
/// `scale` enlarges the output for visibility (e.g. `4` => 4x pixels).
pub fn render_width_test_png(
    caps: &PrinterCaps,
    opts: &PrintOptions,
    scale: u32,
) -> Result<Vec<u8>> {
    if scale == 0 {
        return Err(Error::InvalidArgument("scale must be >= 1".into()));
    }

    let rows = generate_width_test_rows(caps, opts)?;
    render_rows_to_png(&rows, caps.print_width_dots, scale)
}

/// Build a single-job byte stream, then split it into vendor messages.
///
/// The output is ready for "1 vendor message per GATT write" sending.
pub fn encode_bitmap_job_messages(
    rows: &[Vec<u8>],
    caps: &PrinterCaps,
    opts: &PrintOptions,
    page_key: u16,
    finalize: FinalizeMode,
) -> Result<Vec<Vec<u8>>> {
    let payload = encode_bitmap_job_payload(rows, caps, opts, page_key, finalize)?;
    split_vendor_messages(&payload)
}

/// Build a single-job byte stream (concatenated frames + bitmap commands).
///
/// Most callers should use [`encode_bitmap_job_messages`] instead.
pub fn encode_bitmap_job_payload(
    rows: &[Vec<u8>],
    caps: &PrinterCaps,
    opts: &PrintOptions,
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
    out.extend_from_slice(&encode_header_payload(caps, opts, page_key)?);

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

fn encode_header_payload(
    caps: &PrinterCaps,
    opts: &PrintOptions,
    page_key: u16,
) -> Result<Vec<u8>> {
    let byte_width = byte_width(caps.print_width_dots)?;
    let byte_width_u8: u8 = byte_width
        .try_into()
        .map_err(|_| Error::InvalidArgument("print_width_dots is too large".into()))?;

    let mut out = Vec::new();

    // CMD_PAGE_START: pageKey as u16 BE (pageKey=1 => 00 01)
    out.extend_from_slice(&encode_dzpkg(CMD_PAGE_START, &page_key.to_be_bytes())?);

    // CMD_PAGE_WIDTH: byte width (dots/8)
    out.extend_from_slice(&encode_dzpkg(CMD_PAGE_WIDTH, &[byte_width_u8])?);

    let (gap_type, gap_len) = match opts.paper_type {
        PaperType::Continuous => (0x00, [0x00, 0x00]),
        PaperType::Gap => (0x02, [0xff, 0xff]),
    };

    // CMD_GAP_TYPE: paper detection mode.
    out.extend_from_slice(&encode_dzpkg(CMD_GAP_TYPE, &[gap_type])?);

    // CMD_GAP_LEN: keep legacy max value for gap paper; use 0 for continuous mode.
    out.extend_from_slice(&encode_dzpkg(CMD_GAP_LEN, &gap_len)?);

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
    if !print_width_dots.is_multiple_of(8) {
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
        set_dot_msb_left_checked(row, w, x_off);
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

fn render_rows_to_png(rows: &[Vec<u8>], width_dots: u16, scale: u32) -> Result<Vec<u8>> {
    let w: u32 = width_dots.into();
    let h: u32 = rows
        .len()
        .try_into()
        .map_err(|_| Error::InvalidArgument("bitmap too tall".into()))?;

    let byte_width = byte_width(width_dots)?;

    let mut img = image::GrayImage::from_pixel(w, h, image::Luma([255u8]));

    for (y, row) in rows.iter().enumerate() {
        if row.len() != byte_width {
            return Err(Error::InvalidArgument(format!(
                "row byte width mismatch: got={} want={byte_width}",
                row.len()
            )));
        }

        for x in 0..(w as usize) {
            let byte_idx = x / 8;
            let bit = 7 - (x % 8);
            let is_black = (row[byte_idx] & (1u8 << bit)) != 0;
            if is_black {
                img.put_pixel(x as u32, y as u32, image::Luma([0u8]));
            }
        }
    }

    // Nearest-neighbor scale (repeat pixels), to avoid pulling in extra image features.
    let img = if scale == 1 {
        img
    } else {
        let mut scaled = image::GrayImage::from_pixel(w * scale, h * scale, image::Luma([255u8]));
        for y in 0..h {
            for x in 0..w {
                let px = *img.get_pixel(x, y);
                if px.0[0] == 255 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        scaled.put_pixel(x * scale + dx, y * scale + dy, px);
                    }
                }
            }
        }
        scaled
    };

    let mut out = Vec::new();
    image::DynamicImage::ImageLuma8(img)
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| Error::Image(format!("png encode: {e}")))?;
    Ok(out)
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

    #[test]
    fn header_gap_commands_follow_paper_type() -> Result<()> {
        let caps = PrinterCaps::default();
        let row = vec![0u8; byte_width(caps.print_width_dots)?];

        let gap_opts = PrintOptions {
            paper_type: PaperType::Gap,
            ..PrintOptions::default()
        };
        let gap_msgs = encode_bitmap_job_messages(
            std::slice::from_ref(&row),
            &caps,
            &gap_opts,
            1,
            FinalizeMode::default(),
        )?;
        assert_eq!(gap_msgs[2], vec![0x1f, CMD_GAP_TYPE, 0x01, 0x02, 0x88]);
        assert_eq!(gap_msgs[3], vec![0x1f, CMD_GAP_LEN, 0x02, 0xff, 0xff, 0x88]);

        let continuous_opts = PrintOptions {
            paper_type: PaperType::Continuous,
            ..PrintOptions::default()
        };
        let continuous_msgs = encode_bitmap_job_messages(
            std::slice::from_ref(&row),
            &caps,
            &continuous_opts,
            1,
            FinalizeMode::default(),
        )?;
        assert_eq!(
            continuous_msgs[2],
            vec![0x1f, CMD_GAP_TYPE, 0x01, 0x00, 0x88]
        );
        assert_eq!(
            continuous_msgs[3],
            vec![0x1f, CMD_GAP_LEN, 0x02, 0x00, 0x00, 0x88]
        );

        Ok(())
    }

    #[test]
    fn width_test_preview_png_dimensions() -> Result<()> {
        let caps = PrinterCaps::default();
        let opts = PrintOptions::default();

        let png = render_width_test_png(&caps, &opts, 4)?;
        let img =
            image::load_from_memory(&png).map_err(|e| Error::Image(format!("png decode: {e}")))?;

        assert_eq!(img.width(), (caps.print_width_dots as u32) * 4);
        assert_eq!(img.height(), 64 * 4);
        Ok(())
    }
}
