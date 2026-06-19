use std::time::{Duration, Instant};

use bluest::Adapter;
use futures::StreamExt;

const PRINTER_SERVICE_UUID: bluest::Uuid =
    bluest::Uuid::from_u128(0x49535343fe7d4ae58fa99fafd205e455);
const PRINTER_WRITE_CHARACTERISTIC_UUID: bluest::Uuid =
    bluest::Uuid::from_u128(0x49535343884143f4a8d4ecbe34729bb3);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: connect_probe <device-id> [rounds] [mode]")?;
    let rounds = std::env::args()
        .nth(2)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3);
    let mode = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "open-each".to_string());

    let adapter = Adapter::default().await.ok_or("Bluetooth adapter not found")?;
    adapter.wait_available().await?;

    let device_id: bluest::DeviceId = serde_json::from_str(&format!("\"{}\"", id))?;
    let reused_device = if mode == "scan-once" {
        Some(find_device_by_id(&adapter, &id, Duration::from_secs(20)).await?)
    } else {
        None
    };

    for round in 1..=rounds {
        println!("round={round} stage=open");
        let device = match mode.as_str() {
            "open-each" => adapter.open_device(&device_id).await?,
            "scan-each" => find_device_by_id(&adapter, &id, Duration::from_secs(20)).await?,
            "scan-once" => reused_device
                .as_ref()
                .ok_or("scan-once missing device")?
                .clone(),
            other => return Err(format!("unsupported mode: {other}").into()),
        };

        println!("round={round} stage=connect");
        let t0 = Instant::now();
        tokio::time::timeout(Duration::from_secs(20), adapter.connect_device(&device)).await??;
        println!("round={round} connected_after_ms={}", t0.elapsed().as_millis());

        let connected_deadline = Instant::now() + Duration::from_secs(10);
        while !device.is_connected().await {
            if Instant::now() >= connected_deadline {
                return Err(format!("round={round} connect poll timeout").into());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        println!("round={round} stage=discover-service");
        let t0 = Instant::now();
        let services = tokio::time::timeout(
            Duration::from_secs(20),
            device.discover_services_with_uuid(PRINTER_SERVICE_UUID),
        )
        .await??;
        println!(
            "round={round} service_count={} service_after_ms={}",
            services.len(),
            t0.elapsed().as_millis()
        );

        let service = services
            .into_iter()
            .next()
            .ok_or_else(|| format!("round={round} printer service missing"))?;

        println!("round={round} stage=discover-char");
        let t0 = Instant::now();
        let chars = tokio::time::timeout(
            Duration::from_secs(20),
            service.discover_characteristics_with_uuid(PRINTER_WRITE_CHARACTERISTIC_UUID),
        )
        .await??;
        println!(
            "round={round} char_count={} char_after_ms={}",
            chars.len(),
            t0.elapsed().as_millis()
        );

        println!("round={round} stage=disconnect");
        let t0 = Instant::now();
        tokio::time::timeout(Duration::from_secs(10), adapter.disconnect_device(&device)).await??;
        println!("round={round} disconnected_after_ms={}", t0.elapsed().as_millis());

        let disconnected_deadline = Instant::now() + Duration::from_secs(10);
        while device.is_connected().await {
            if Instant::now() >= disconnected_deadline {
                return Err(format!("round={round} disconnect poll timeout").into());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}

async fn find_device_by_id(
    adapter: &Adapter,
    id_str: &str,
    timeout: Duration,
) -> Result<bluest::Device, Box<dyn std::error::Error>> {
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

        if adv.device.id().to_string() == id_str {
            drop(scan);
            return Ok(adv.device);
        }
    }

    drop(scan);
    Err(format!("device {id_str} not found").into())
}
