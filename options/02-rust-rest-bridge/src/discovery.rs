use anyhow::{anyhow, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use crate::bot::SWITCHBOT_SERVICE_UUID;

#[derive(Debug, Clone)]
pub struct DiscoveredBot {
    pub id: String,
    pub name: String,
    pub rssi: Option<i16>,
    pub peripheral: Peripheral,
}

pub async fn get_central() -> Result<Adapter> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No Bluetooth adapter found on this system"))?;

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    loop {
        let state = central.adapter_state().await;
        info!("Current adapter state: {:?}", state);
        if let Ok(btleplug::api::CentralState::PoweredOn) = state {
            break;
        }
        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Timed out waiting for Bluetooth adapter to become PoweredOn (last state: {:?})",
                state
            ));
        }
        sleep(Duration::from_millis(500)).await;
    }

    Ok(central)
}

/// Scans for SwitchBot devices for the specified duration (default 3-5 seconds).
pub async fn scan_bots(duration: Duration) -> Result<Vec<DiscoveredBot>> {
    let central = get_central().await?;
    info!("Starting BLE scan ({} seconds)...", duration.as_secs());
    central.start_scan(ScanFilter::default()).await?;

    let interval = Duration::from_millis(500);
    let iterations = (duration.as_millis() / interval.as_millis()).max(1);

    for _ in 0..iterations {
        sleep(interval).await;
    }

    let mut found = Vec::new();
    let peripherals = central.peripherals().await?;
    info!(
        "Total BLE peripherals observed by adapter: {}",
        peripherals.len()
    );

    for p in &peripherals {
        if let Ok(Some(props)) = p.properties().await {
            let name = props.local_name.clone().unwrap_or_default();
            let has_service = props.services.contains(&SWITCHBOT_SERVICE_UUID);
            let is_switchbot = has_service
                || name.to_lowercase().contains("wohand")
                || name.to_lowercase().contains("bot")
                || name.to_lowercase().contains("switchbot");

            tracing::debug!(
                "Peripheral id={}, name='{}', services={:?}, rssi={:?}",
                p.id(),
                name,
                props.services,
                props.rssi
            );

            if is_switchbot {
                found.push(DiscoveredBot {
                    id: p.id().to_string(),
                    name: if name.is_empty() {
                        "Unknown SwitchBot".to_string()
                    } else {
                        name
                    },
                    rssi: props.rssi,
                    peripheral: p.clone(),
                });
            }
        } else {
            tracing::debug!("Peripheral id={} had no properties available", p.id());
        }
    }

    let _ = central.stop_scan().await;
    Ok(found)
}

/// Finds a single SwitchBot Bot matching an optional name or ID filter.
pub async fn find_target_bot(filter: Option<&str>, timeout: Duration) -> Result<Peripheral> {
    let central = get_central().await?;
    info!(
        "Scanning for target SwitchBot Bot (timeout: {}s)...",
        timeout.as_secs()
    );
    central.start_scan(ScanFilter::default()).await?;

    let step = Duration::from_millis(400);
    let max_steps = (timeout.as_millis() / step.as_millis()).max(1);

    for _ in 0..max_steps {
        sleep(step).await;
        let peripherals = central.peripherals().await?;

        for p in peripherals {
            if let Ok(Some(props)) = p.properties().await {
                let name = props.local_name.clone().unwrap_or_default();
                let id_str = p.id().to_string();
                let has_service = props.services.contains(&SWITCHBOT_SERVICE_UUID);
                let is_bot = has_service || name.contains("WoHand") || name.contains("Bot");

                if !is_bot {
                    continue;
                }

                if let Some(query) = filter {
                    let q = query.to_lowercase();
                    if name.to_lowercase().contains(&q) || id_str.to_lowercase().contains(&q) {
                        info!("Matched target SwitchBot: '{}' (ID: {})", name, id_str);
                        let _ = central.stop_scan().await;
                        return Ok(p);
                    }
                } else {
                    info!("Found SwitchBot: '{}' (ID: {})", name, id_str);
                    let _ = central.stop_scan().await;
                    return Ok(p);
                }
            }
        }
    }

    let _ = central.stop_scan().await;
    Err(anyhow!(
        "No matching SwitchBot Bot found within {}s (filter: {:?})",
        timeout.as_secs(),
        filter
    ))
}
