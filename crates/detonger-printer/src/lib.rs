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
        let messages = protocol::encode_png_job_messages_with_finalize(
            png,
            &caps,
            opts,
            protocol::FinalizeMode::default(),
        )?;
        self.write_vendor_messages(&messages).await
    }

    pub async fn print_png_in_chunks(
        &mut self,
        png: &[u8],
        opts: &PrintOptions,
        max_rows_per_chunk: usize,
    ) -> Result<()> {
        let caps = PrinterCaps::default();
        let jobs = protocol::encode_png_job_messages_in_chunks(
            png,
            &caps,
            opts,
            max_rows_per_chunk,
            protocol::FinalizeMode::default(),
        )?;
        self.write_vendor_message_jobs(&jobs).await
    }

    pub async fn print_job_payload(&mut self, payload: &[u8]) -> Result<()> {
        self.write_raw_payload(payload, 180).await
    }

    pub async fn print_vendor_messages(&mut self, messages: &[Vec<u8>]) -> Result<()> {
        self.write_vendor_messages(messages).await
    }

    pub async fn print_width_test(
        &mut self,
        caps: &PrinterCaps,
        opts: &PrintOptions,
    ) -> Result<()> {
        let messages = protocol::encode_width_test_job_messages_with_finalize(
            caps,
            opts,
            protocol::FinalizeMode::default(),
        )?;
        self.write_vendor_messages(&messages).await
    }

    async fn write_vendor_message_jobs(&mut self, jobs: &[Vec<Vec<u8>>]) -> Result<()> {
        let debug = std::env::var_os("DETONGER_DEBUG").is_some();

        if debug {
            eprintln!("[detonger] write-jobs: start (jobs={})", jobs.len());
        }

        for (index, job) in jobs.iter().enumerate() {
            if debug {
                eprintln!(
                    "[detonger] write-jobs: job {}/{} (messages={})",
                    index + 1,
                    jobs.len(),
                    job.len()
                );
            }

            self.write_vendor_messages(job).await?;
        }

        if debug {
            eprintln!("[detonger] write-jobs: done");
        }

        Ok(())
    }

    async fn write_vendor_messages(&mut self, messages: &[Vec<u8>]) -> Result<()> {
        // The P2 can drop the BLE link on larger jobs if messages are paced too aggressively.
        let delay_ms = std::env::var("DETONGER_WRITE_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(5);
        let delay = Duration::from_millis(delay_ms);
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

        for (index, msg) in messages.iter().enumerate() {
            if debug && (index == 0 || (index + 1) % 16 == 0 || index + 1 == messages.len()) {
                eprintln!(
                    "[detonger] write: progress {}/{} (len={})",
                    index + 1,
                    messages.len(),
                    msg.len()
                );
            }

            tokio::time::timeout(write_timeout, self.inner.write(msg))
                .await
                .map_err(|_| Error::Timeout)
                .and_then(|result| {
                    result.map_err(|err| {
                        Error::Ble(format!(
                            "write message {}/{} failed (len={}): {}",
                            index + 1,
                            messages.len(),
                            msg.len(),
                            err
                        ))
                    })
                })?;

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

    async fn write_raw_payload(&mut self, payload: &[u8], chunk_size: usize) -> Result<()> {
        let delay_ms = std::env::var("DETONGER_WRITE_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(5);
        let delay = Duration::from_millis(delay_ms);
        let write_timeout = Duration::from_secs(5);
        let wait_after = Duration::from_secs(2);

        let debug = std::env::var_os("DETONGER_DEBUG").is_some();
        if debug {
            eprintln!(
                "[detonger] raw-write: start (bytes={}, chunk_size={}, delay={:?}, write_timeout={:?})",
                payload.len(),
                chunk_size,
                delay,
                write_timeout
            );
        }

        let total_chunks = payload.len().div_ceil(chunk_size);
        for (index, chunk) in payload.chunks(chunk_size).enumerate() {
            if debug && (index == 0 || (index + 1) % 16 == 0 || index + 1 == total_chunks) {
                eprintln!(
                    "[detonger] raw-write: progress {}/{} (len={})",
                    index + 1,
                    total_chunks,
                    chunk.len()
                );
            }

            tokio::time::timeout(write_timeout, self.inner.write(chunk))
                .await
                .map_err(|_| Error::Timeout)
                .and_then(|result| {
                    result.map_err(|err| {
                        Error::Ble(format!(
                            "raw write chunk {}/{} failed (len={}): {}",
                            index + 1,
                            total_chunks,
                            chunk.len(),
                            err
                        ))
                    })
                })?;

            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
        }

        if !wait_after.is_zero() {
            tokio::time::sleep(wait_after).await;
        }

        if debug {
            eprintln!("[detonger] raw-write: done");
        }

        Ok(())
    }
}
