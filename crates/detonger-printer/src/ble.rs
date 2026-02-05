use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use bluest::{Adapter, CharacteristicProperties, Uuid as BluestUuid};
use futures::StreamExt as _;

use crate::uuid::{PRINTER_SERVICE_UUID, PRINTER_WRITE_CHARACTERISTIC_UUID};
use crate::{DeviceId, DiscoveredDevice, Error, Result};

const ADAPTER_TIMEOUT: Duration = Duration::from_secs(10);
const SCAN_LOOKUP_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const DISCOVER_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECTED_POLL_TIMEOUT: Duration = Duration::from_secs(10);

/// Opaque BLE connection state held by [`crate::PrinterConnection`].
pub(crate) struct BlePrinterConnection {
    // On macOS, the adapter owns the connection lifecycle. Keep it alive for the duration of the
    // printer connection so the link doesn't get torn down early.
    #[allow(dead_code)]
    adapter: Adapter,
    #[allow(dead_code)]
    device: bluest::Device,
    write_characteristic: bluest::Characteristic,
    write_without_response: bool,
}

impl BlePrinterConnection {
    pub(crate) async fn write(&self, data: &[u8]) -> Result<()> {
        if self.write_without_response {
            tokio::time::timeout(
                DISCOVER_TIMEOUT,
                self.write_characteristic.write_without_response(data),
            )
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(map_bluest_error)?;
        } else {
            tokio::time::timeout(DISCOVER_TIMEOUT, self.write_characteristic.write(data))
                .await
                .map_err(|_| Error::Timeout)?
                .map_err(map_bluest_error)?;
        }

        Ok(())
    }
}

pub(crate) async fn scan(timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
    let adapter = default_adapter().await?;

    let mut scan = adapter.scan(&[]).await.map_err(map_bluest_error)?;

    let deadline = Instant::now() + timeout;
    let mut seen: HashMap<String, DiscoveredDevice> = HashMap::new();

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());

        let next = tokio::time::timeout(remaining, scan.next()).await;
        let adv = match next {
            Ok(Some(v)) => v,
            Ok(None) => break,
            Err(_) => break,
        };

        let id = adv.device.id().to_string();
        let name = adv.adv_data.local_name;
        let rssi = adv.rssi;

        seen.entry(id.clone())
            .and_modify(|d| {
                if d.name.is_none() {
                    d.name = name.clone();
                }
                d.rssi = rssi.or(d.rssi);
            })
            .or_insert_with(|| DiscoveredDevice {
                id: DeviceId(id),
                name,
                rssi,
            });
    }

    // Dropping the stream stops scanning.
    drop(scan);

    let mut out: Vec<DiscoveredDevice> = seen.into_values().collect();
    out.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    Ok(out)
}

pub(crate) async fn connect(device: &DeviceId) -> Result<BlePrinterConnection> {
    let adapter = default_adapter().await?;

    let debug = std::env::var_os("DETONGER_DEBUG").is_some();
    if debug {
        eprintln!("[detonger] connect: scanning for device id {}...", device.0);
    }

    let dev = find_device_by_id(&adapter, &device.0, SCAN_LOOKUP_TIMEOUT).await?;

    if debug {
        eprintln!("[detonger] connect: connecting...");
    }

    tokio::time::timeout(CONNECT_TIMEOUT, adapter.connect_device(&dev))
        .await
        .map_err(|_| Error::Timeout)?
        .map_err(map_bluest_error)?;

    // On macOS, `connect_device` may return before the device is fully connected/ready for GATT.
    // Poll briefly to avoid immediately timing out on service discovery.
    let poll_deadline = Instant::now() + CONNECTED_POLL_TIMEOUT;
    while !dev.is_connected().await {
        if Instant::now() >= poll_deadline {
            return Err(Error::Timeout);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    if debug {
        eprintln!("[detonger] connect: connected");
    }

    // Discover just the service/characteristic we need.
    if debug {
        eprintln!(
            "[detonger] connect: discovering service {} (timeout={:?})...",
            PRINTER_SERVICE_UUID, DISCOVER_TIMEOUT
        );
    }

    let t0 = Instant::now();
    let services = tokio::time::timeout(
        DISCOVER_TIMEOUT,
        dev.discover_services_with_uuid(PRINTER_SERVICE_UUID),
    )
    .await;
    if debug {
        eprintln!(
            "[detonger] connect: service discovery finished after {:?}",
            t0.elapsed()
        );
    }
    let services = services
        .map_err(|_| Error::Timeout)?
        .map_err(map_bluest_error)?;

    if debug {
        eprintln!(
            "[detonger] connect: service discovered (count={})",
            services.len()
        );
    }

    let service = services.into_iter().next().ok_or_else(|| {
        Error::NotFound(format!(
            "service {PRINTER_SERVICE_UUID} not found on device {}",
            device.0
        ))
    })?;

    if debug {
        eprintln!(
            "[detonger] connect: discovering characteristic {} (timeout={:?})...",
            PRINTER_WRITE_CHARACTERISTIC_UUID, DISCOVER_TIMEOUT
        );
    }

    let t0 = Instant::now();
    let chars = tokio::time::timeout(
        DISCOVER_TIMEOUT,
        service.discover_characteristics_with_uuid(PRINTER_WRITE_CHARACTERISTIC_UUID),
    )
    .await;
    if debug {
        eprintln!(
            "[detonger] connect: characteristic discovery finished after {:?}",
            t0.elapsed()
        );
    }
    let chars = chars
        .map_err(|_| Error::Timeout)?
        .map_err(map_bluest_error)?;

    if debug {
        eprintln!(
            "[detonger] connect: characteristic discovered (count={})",
            chars.len()
        );
    }

    let write_characteristic = chars.into_iter().next().ok_or_else(|| {
        Error::NotFound(format!(
            "write characteristic {PRINTER_WRITE_CHARACTERISTIC_UUID} not found on device {}",
            device.0
        ))
    })?;

    let props = write_characteristic
        .properties()
        .await
        .map_err(map_bluest_error)?;

    let (write_without_response, _props) = pick_write_mode(&props)?;

    if debug {
        eprintln!(
            "[detonger] connect: ready (write_without_response={})",
            write_without_response
        );
    }

    Ok(BlePrinterConnection {
        adapter,
        device: dev,
        write_characteristic,
        write_without_response,
    })
}

async fn default_adapter() -> Result<Adapter> {
    let adapter = Adapter::default()
        .await
        .ok_or_else(|| Error::Ble("no bluetooth adapters found".to_string()))?;

    tokio::time::timeout(ADAPTER_TIMEOUT, adapter.wait_available())
        .await
        .map_err(|_| Error::Timeout)?
        .map_err(map_bluest_error)?;

    Ok(adapter)
}

async fn find_device_by_id(
    adapter: &Adapter,
    id_str: &str,
    timeout: Duration,
) -> Result<bluest::Device> {
    let mut scan = adapter.scan(&[]).await.map_err(map_bluest_error)?;

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let next = tokio::time::timeout(remaining, scan.next()).await;
        let adv = match next {
            Ok(Some(v)) => v,
            Ok(None) => break,
            Err(_) => break,
        };

        if adv.device.id().to_string() == id_str {
            drop(scan);
            return Ok(adv.device);
        }
    }

    drop(scan);
    Err(Error::NotFound(format!("device {id_str} not found")))
}

fn pick_write_mode(props: &CharacteristicProperties) -> Result<(bool, CharacteristicProperties)> {
    if props.write_without_response {
        return Ok((true, *props));
    }
    if props.write {
        return Ok((false, *props));
    }
    Err(Error::NotFound(
        "write characteristic does not support write".to_string(),
    ))
}

fn map_bluest_error(err: bluest::Error) -> Error {
    use bluest::error::ErrorKind as K;
    match err.kind() {
        K::Timeout => Error::Ble(err.to_string()),
        K::NotFound => Error::NotFound(err.to_string()),
        K::InvalidParameter => Error::InvalidArgument(err.to_string()),
        // On macOS, this is commonly the "Bluetooth permission not granted" case.
        K::NotAuthorized => Error::Ble(err.to_string()),
        other => Error::Ble(format!("{other}: {}", err.message())),
    }
}

// Keep a stable UUID type identity in this module (helps avoid confusing imports).
#[allow(dead_code)]
fn _assert_uuid_types_are_compatible(_u: BluestUuid) {}
