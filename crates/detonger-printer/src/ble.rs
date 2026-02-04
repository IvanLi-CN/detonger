use std::time::{Duration, Instant};

use btleplug::api::{Central as _, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};

use crate::uuid::PRINTER_WRITE_CHARACTERISTIC_UUID;
use crate::{DeviceId, DiscoveredDevice, Error, Result};

/// Opaque BLE connection state held by [`crate::PrinterConnection`].
///
/// Keep btleplug types out of the public API so the crate surface stays stable
/// even if we ever swap BLE backends.
pub(crate) struct BlePrinterConnection {
    pub(crate) peripheral: Peripheral,
    pub(crate) write_characteristic: btleplug::api::Characteristic,
}

pub(crate) async fn scan(timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
    let adapter = default_adapter().await?;

    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(map_btleplug_error)?;

    if !timeout.is_zero() {
        tokio::time::sleep(timeout).await;
    }

    // Best-effort stop. Even if it fails, we can still return discovered devices.
    let _ = adapter.stop_scan().await;

    let peripherals = adapter.peripherals().await.map_err(map_btleplug_error)?;
    let mut out = Vec::with_capacity(peripherals.len());

    for p in peripherals {
        let id = DeviceId(p.id().to_string());

        let props = p.properties().await.map_err(map_btleplug_error)?;
        let (name, rssi) = match props {
            Some(props) => (props.local_name, props.rssi),
            None => (None, None),
        };

        out.push(DiscoveredDevice { id, name, rssi });
    }

    // Deterministic ordering for humans/tests.
    out.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    Ok(out)
}

pub(crate) async fn connect(device: &DeviceId) -> Result<BlePrinterConnection> {
    let adapter = default_adapter().await?;

    // On macOS/CoreBluetooth, peripherals are only known once discovered via scan.
    // Btleplug's CoreBluetooth backend cannot "add" a peripheral from an ID.
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(map_btleplug_error)?;

    let peripheral = find_peripheral_by_id(&adapter, device, Duration::from_secs(5)).await?;

    // Best-effort stop.
    let _ = adapter.stop_scan().await;

    if !peripheral.is_connected().await.map_err(map_btleplug_error)? {
        tokio::time::timeout(Duration::from_secs(15), peripheral.connect())
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(map_btleplug_error)?;
    }

    tokio::time::timeout(Duration::from_secs(15), peripheral.discover_services())
        .await
        .map_err(|_| Error::Timeout)?
        .map_err(map_btleplug_error)?;

    let write_characteristic = peripheral
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == PRINTER_WRITE_CHARACTERISTIC_UUID)
        .ok_or_else(|| {
            Error::NotFound(format!(
                "write characteristic {} not found on device {}",
                PRINTER_WRITE_CHARACTERISTIC_UUID, device.0
            ))
        })?;

    Ok(BlePrinterConnection {
        peripheral,
        write_characteristic,
    })
}

async fn default_adapter() -> Result<Adapter> {
    // Adapter bring-up on macOS waits for a CoreBluetooth state update; wrap it so
    // we don't hang forever if permission is missing or Bluetooth is off.
    let adapters = tokio::time::timeout(Duration::from_secs(10), async {
        let manager = Manager::new().await?;
        manager.adapters().await
    })
    .await
    .map_err(|_| Error::Timeout)?
    .map_err(map_btleplug_error)?;

    adapters
        .into_iter()
        .next()
        .ok_or_else(|| Error::Ble("no bluetooth adapters found".to_string()))
}

async fn find_peripheral_by_id(
    adapter: &Adapter,
    device: &DeviceId,
    timeout: Duration,
) -> Result<Peripheral> {
    let deadline = Instant::now() + timeout;

    loop {
        let peripherals = adapter.peripherals().await.map_err(map_btleplug_error)?;
        if let Some(p) = peripherals
            .into_iter()
            .find(|p| p.id().to_string() == device.0)
        {
            return Ok(p);
        }

        if Instant::now() >= deadline {
            return Err(Error::NotFound(format!("device {} not found", device.0)));
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

fn map_btleplug_error(err: btleplug::Error) -> Error {
    match err {
        btleplug::Error::DeviceNotFound => Error::NotFound("device not found".to_string()),
        btleplug::Error::TimedOut(_) => Error::Timeout,
        btleplug::Error::PermissionDenied => Error::Ble(
            "permission denied (macOS: grant Bluetooth permission to your terminal app in System Settings -> Privacy & Security -> Bluetooth)".to_string(),
        ),
        other => Error::Ble(other.to_string()),
    }
}

