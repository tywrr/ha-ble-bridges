use anyhow::{anyhow, Result};
use btleplug::api::{Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};
use uuid::Uuid;

/// Official SwitchBot standard GATT 128-bit UUIDs
pub const SWITCHBOT_SERVICE_UUID: Uuid = Uuid::from_u128(0xcba20d00_224d_11e6_9fb8_0002a5d5c51b);
pub const COMMAND_CHAR_UUID: Uuid = Uuid::from_u128(0xcba20002_224d_11e6_9fb8_0002a5d5c51b);
pub const NOTIFY_CHAR_UUID: Uuid = Uuid::from_u128(0xcba20003_224d_11e6_9fb8_0002a5d5c51b);

pub const CMD_PRESS: [u8; 3] = [0x57, 0x01, 0x00];
pub const CMD_ON: [u8; 3] = [0x57, 0x01, 0x01];
pub const CMD_OFF: [u8; 3] = [0x57, 0x01, 0x02];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotCommand {
    Press,
    On,
    Off,
}

impl BotCommand {
    pub fn as_bytes(&self) -> &'static [u8; 3] {
        match self {
            BotCommand::Press => &CMD_PRESS,
            BotCommand::On => &CMD_ON,
            BotCommand::Off => &CMD_OFF,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BotCommand::Press => "Press",
            BotCommand::On => "Turn On",
            BotCommand::Off => "Turn Off",
        }
    }
}

/// Connects to a SwitchBot Bot peripheral, discovers services, sends the requested command, and disconnects.
pub async fn send_bot_command(peripheral: &Peripheral, cmd: BotCommand) -> Result<()> {
    info!(
        "Connecting to SwitchBot peripheral (ID: {})...",
        peripheral.id()
    );
    peripheral.connect().await?;

    info!("Connected. Discovering GATT services...");
    peripheral.discover_services().await?;

    let chars = peripheral.characteristics();
    let cmd_char = chars
        .iter()
        .find(|c| c.uuid == COMMAND_CHAR_UUID)
        .ok_or_else(|| {
            anyhow!(
                "Command characteristic ({}) not found on device",
                COMMAND_CHAR_UUID
            )
        })?;

    debug!("Found command characteristic: {:?}", cmd_char);

    info!("Sending {} command {:?}...", cmd.name(), cmd.as_bytes());
    // Try WithoutResponse first; fallback to WithResponse if needed
    if cmd_char
        .properties
        .contains(btleplug::api::CharPropFlags::WRITE_WITHOUT_RESPONSE)
    {
        peripheral
            .write(cmd_char, cmd.as_bytes(), WriteType::WithoutResponse)
            .await?;
    } else {
        peripheral
            .write(cmd_char, cmd.as_bytes(), WriteType::WithResponse)
            .await?;
    }

    info!("Command sent successfully. Waiting for actuation (2.0s)...");
    sleep(Duration::from_millis(2000)).await;

    debug!("Disconnecting...");
    let _ = peripheral.disconnect().await;
    info!("Disconnected cleanly.");

    Ok(())
}
