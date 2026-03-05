//! detonger-protocol
//!
//! Protocol/encoding layer for DeTong / Detonger printers.
//! This crate is transport-agnostic and only operates on bytes.

use serde::{Deserialize, Serialize};

pub mod encode;
pub mod split;

pub use encode::{
    FinalizeMode, encode_bitmap_job_messages, encode_bitmap_job_payload, encode_png_job_messages,
    encode_width_test_job_messages, render_width_test_png,
};
pub use split::split_vendor_messages;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("image decode error: {0}")]
    Image(String),

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PrinterCaps {
    pub dpi: u16,
    pub print_width_dots: u16,
}

impl Default for PrinterCaps {
    fn default() -> Self {
        Self {
            dpi: 203,
            print_width_dots: 384,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PrintOptions {
    /// Threshold in `[0,255]`. Lower values make the output lighter.
    pub threshold: u8,
    /// Horizontal offset in printhead dots. Negative shifts left (may crop).
    pub x_offset_dots: i16,
    /// Paper feed mode for gap detection.
    pub paper_type: PaperType,
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            threshold: 150,
            x_offset_dots: 0,
            paper_type: PaperType::Gap,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PaperType {
    Continuous,
    #[default]
    Gap,
}
