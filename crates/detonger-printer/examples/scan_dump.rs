use std::time::{Duration, Instant};

use bluest::Adapter;
use futures::StreamExt;

fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timeout_s = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(12);
    let adapter = Adapter::default().await.ok_or("Bluetooth adapter not found")?;
    adapter.wait_available().await?;

    let mut scan = adapter.scan(&[]).await?;
    let deadline = Instant::now() + Duration::from_secs(timeout_s);

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let next = tokio::time::timeout(remaining, scan.next()).await;
        let adv = match next {
            Ok(Some(v)) => v,
            Ok(None) => break,
            Err(_) => break,
        };

        let name = if adv.adv_data.local_name.is_some() {
            adv.adv_data.local_name.clone()
        } else {
            adv.device.name_async().await.ok().filter(|value| !value.is_empty())
        };

        let manufacturer = adv.adv_data.manufacturer_data.as_ref().map(|data| {
            format!(
                "company=0x{company:04x} len={len} hex={hex}",
                company = data.company_id,
                len = data.data.len(),
                hex = hex_string(&data.data)
            )
        });

        let service_data = if adv.adv_data.service_data.is_empty() {
            Vec::new()
        } else {
            adv.adv_data
                .service_data
                .iter()
                .map(|(uuid, data)| format!("{uuid}={}", hex_string(data)))
                .collect::<Vec<_>>()
        };

        println!("device={}", adv.device.id());
        println!("  name={:?}", name);
        println!("  rssi={:?}", adv.rssi);
        println!("  connectable={}", adv.adv_data.is_connectable);
        println!("  services={:?}", adv.adv_data.services);
        println!("  manufacturer={:?}", manufacturer);
        println!("  service_data={:?}", service_data);
    }

    Ok(())
}
