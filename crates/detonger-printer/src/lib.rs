//! detonger-printer
//!
//! This crate is the Rust library layer for talking to DeTong / Detonger label printers.
//! The public surface is intentionally small and stable so it can be reused by the CLI
//! and future apps.

use std::time::Duration;

use serde::{Deserialize, Serialize};

mod ble;
mod uuid;
pub use detonger_protocol as protocol;
pub use detonger_protocol::{PrintOptions, PrinterCaps};

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

pub struct PrinterConnection {
    inner: ble::BlePrinterConnection,
}

pub async fn scan(timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
    ble::scan(timeout).await
}

impl From<detonger_protocol::Error> for Error {
    fn from(value: detonger_protocol::Error) -> Self {
        match value {
            detonger_protocol::Error::InvalidArgument(msg) => Self::InvalidArgument(msg),
            detonger_protocol::Error::Protocol(msg) => Self::Protocol(msg),
            detonger_protocol::Error::Image(msg) => Self::Image(msg),
            detonger_protocol::Error::Io(err) => Self::Io(err),
        }
    }
}

pub async fn connect(device: &DeviceId) -> Result<PrinterConnection> {
    let inner = ble::connect(device).await?;
    Ok(PrinterConnection { inner })
}

impl PrinterConnection {
    pub async fn print_png(&mut self, png: &[u8], opts: &PrintOptions) -> Result<()> {
        let caps = PrinterCaps::default();
        let messages = protocol::encode_png_job_messages(png, &caps, opts)?;
        self.write_vendor_messages(&messages).await
    }

    pub async fn print_width_test(
        &mut self,
        caps: &PrinterCaps,
        opts: &PrintOptions,
    ) -> Result<()> {
        let messages = protocol::encode_width_test_job_messages(caps, opts)?;
        self.write_vendor_messages(&messages).await
    }

    async fn write_vendor_messages(&mut self, messages: &[Vec<u8>]) -> Result<()> {
        // Conservative pacing based on the legacy replay script. Some firmwares can
        // drop the connection if messages are sent too fast.
        let delay = Duration::from_millis(5);
        let write_timeout = Duration::from_secs(5);
        let wait_after = Duration::from_secs(2);

        let debug = std::env::var_os("DETONGER_DEBUG").is_some();
        if debug {
            eprintln!(
                "[detonger] write: start (messages={}, delay={:?}, write_timeout={:?})",
                messages.len(),
                delay,
                write_timeout
            );
        }

        for msg in messages {
            tokio::time::timeout(write_timeout, self.inner.write(msg))
                .await
                .map_err(|_| Error::Timeout)??;

            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
        }

        // Give the printer time to process/print before the connection is dropped by process exit.
        if !wait_after.is_zero() {
            tokio::time::sleep(wait_after).await;
        }

        if debug {
            eprintln!("[detonger] write: done");
        }

        Ok(())
    }
}
