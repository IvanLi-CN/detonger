use std::time::{Duration, Instant};

use bluest::Adapter;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: gatt_dump <device-id> [scan-timeout-s]")?;
    let scan_timeout_s = std::env::args()
        .nth(2)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(8);

    let adapter = Adapter::default().await.ok_or("Bluetooth adapter not found")?;
    adapter.wait_available().await?;

    let device = find_device_by_id(&adapter, &id, Duration::from_secs(scan_timeout_s))
        .await?
        .ok_or_else(|| format!("device {id} not found during scan"))?;

    adapter.connect_device(&device).await?;

    let connected_deadline = Instant::now() + Duration::from_secs(10);
    while !device.is_connected().await {
        if Instant::now() >= connected_deadline {
            return Err("connect poll timeout".into());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let services = device.discover_services().await?;
    for service in services {
        let service_uuid = service.uuid();
        let is_primary = service.is_primary().await.unwrap_or(false);
        println!("service={service_uuid} primary={is_primary}");

        let characteristics = service.discover_characteristics().await?;
        for characteristic in characteristics {
            let characteristic_uuid = characteristic.uuid();
            let props = characteristic.properties().await?;
            let max_write_len = characteristic.max_write_len().ok();
            println!(
                "  characteristic={} props={{broadcast:{} read:{} write_without_response:{} write:{} notify:{} indicate:{} authenticated_signed_writes:{} extended_properties:{} reliable_writes:{} writable_auxiliaries:{}}} max_write_len={:?}",
                characteristic_uuid,
                props.broadcast,
                props.read,
                props.write_without_response,
                props.write,
                props.notify,
                props.indicate,
                props.authenticated_signed_writes,
                props.extended_properties,
                props.reliable_write,
                props.writable_auxiliaries,
                max_write_len
            );
        }
    }

    adapter.disconnect_device(&device).await?;
    Ok(())
}

async fn find_device_by_id(
    adapter: &Adapter,
    id: &str,
    timeout: Duration,
) -> Result<Option<bluest::Device>, Box<dyn std::error::Error>> {
    let mut scan = adapter.scan(&[]).await?;
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let next = tokio::time::timeout(remaining, scan.next()).await;
        let adv = match next {
            Ok(Some(v)) => v,
            Ok(None) => break,
            Err(_) => break,
        };

        if adv.device.id().to_string() == id {
            drop(scan);
            return Ok(Some(adv.device));
        }
    }

    drop(scan);
    Ok(None)
}
