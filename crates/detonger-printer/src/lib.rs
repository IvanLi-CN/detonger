//! detonger-printer
//!
//! This crate is the Rust library layer for talking to DeTong / Detonger label printers.
//! The public surface is intentionally small and stable so it can be reused by the CLI
//! and future apps.

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("timeout")]
    Timeout,

    #[error("printer not available: {0}")]
    PrinterNotAvailable(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("ble error: {0}")]
    Ble(String),

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image decode error: {0}")]
    Image(String),

    #[error("unimplemented: {0}")]
    Unimplemented(&'static str),
}

/// A BLE device id that the caller can copy-paste (macOS: CoreBluetooth UUID string).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub id: DeviceId,
    pub name: Option<String>,
    pub rssi: Option<i16>,
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
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            threshold: 150,
            x_offset_dots: 0,
        }
    }
}

pub struct PrinterConnection {
    _priv: (),
}

pub async fn scan(_timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
    Err(Error::Unimplemented("scan"))
}

pub async fn connect(_device: &DeviceId) -> Result<PrinterConnection> {
    Err(Error::Unimplemented("connect"))
}

impl PrinterConnection {
    pub async fn print_png(&mut self, _png: &[u8], _opts: &PrintOptions) -> Result<()> {
        Err(Error::Unimplemented("print_png"))
    }

    pub async fn print_width_test(&mut self, _caps: &PrinterCaps, _opts: &PrintOptions) -> Result<()> {
        Err(Error::Unimplemented("print_width_test"))
    }
}

