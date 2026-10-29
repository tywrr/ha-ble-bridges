use anyhow::Result;
use btleplug::api::{Central, Peripheral};
use clap::{Parser, Subcommand};
use std::time::Duration;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use switchbot_bridge::bot::{send_bot_command, BotCommand};
use switchbot_bridge::discovery::{find_target_bot, scan_bots};
use switchbot_bridge::server::run_server;

#[derive(Parser)]
#[command(name = "switchbot-bridge")]
#[command(version)]
#[command(about = "A lightweight BLE bridge and CLI for SwitchBot Bots", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for nearby SwitchBot devices and display names, RSSI, and IDs
    Scan {
        /// Scan duration in seconds
        #[arg(short, long, default_value_t = 5)]
        duration: u64,

        /// List all discovered BLE peripherals, not just SwitchBot devices
        #[arg(short, long)]
        all: bool,
    },
    /// Trigger a "Press" actuation on a SwitchBot Bot
    Press {
        /// Optional name or ID substring to match (e.g. the last digits of the Bot's name)
        #[arg(short, long)]
        device: Option<String>,

        /// Discovery timeout in seconds
        #[arg(short, long, default_value_t = 6)]
        timeout: u64,
    },
    /// Trigger a "Turn On" actuation on a SwitchBot Bot (Switch mode)
    On {
        /// Optional name or ID substring to match
        #[arg(short, long)]
        device: Option<String>,

        /// Discovery timeout in seconds
        #[arg(short, long, default_value_t = 6)]
        timeout: u64,
    },
    /// Trigger a "Turn Off" actuation on a SwitchBot Bot (Switch mode)
    Off {
        /// Optional name or ID substring to match
        #[arg(short, long)]
        device: Option<String>,

        /// Discovery timeout in seconds
        #[arg(short, long, default_value_t = 6)]
        timeout: u64,
    },
    /// Start the HTTP REST server for Home Assistant integration
    Serve {
        /// Host address to bind to
        #[arg(short = 'H', long, default_value = "0.0.0.0")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "switchbot_bridge=info,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { duration, all } => {
            if all {
                info!("Scanning for ALL BLE devices for {duration}s...");
                let central = switchbot_bridge::discovery::get_central().await?;
                central
                    .start_scan(btleplug::api::ScanFilter::default())
                    .await?;
                tokio::time::sleep(Duration::from_secs(duration)).await;
                let peripherals = central.peripherals().await?;
                println!("\nFound {} BLE peripheral(s):", peripherals.len());
                for (i, p) in peripherals.iter().enumerate() {
                    let props = p.properties().await.ok().flatten();
                    let name = props
                        .as_ref()
                        .and_then(|pr| pr.local_name.clone())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    let rssi = props
                        .as_ref()
                        .and_then(|pr| pr.rssi)
                        .map(|r| format!("{r} dBm"))
                        .unwrap_or_else(|| "N/A".to_string());
                    let services = props
                        .as_ref()
                        .map(|pr| pr.services.clone())
                        .unwrap_or_default();
                    println!(
                        "  [{}] Name: {:<20} | RSSI: {:<8} | ID: {} | Services: {:?}",
                        i + 1,
                        name,
                        rssi,
                        p.id(),
                        services
                    );
                }
                let _ = central.stop_scan().await;
            } else {
                info!("Scanning for SwitchBot devices for {duration}s...");
                let bots = scan_bots(Duration::from_secs(duration)).await?;
                if bots.is_empty() {
                    println!("\nNo SwitchBot devices found nearby. Use '--all' to inspect all discovered BLE peripherals.");
                } else {
                    println!("\nFound {} SwitchBot device(s):", bots.len());
                    for (i, bot) in bots.iter().enumerate() {
                        let rssi_str = bot
                            .rssi
                            .map(|r| format!("{r} dBm"))
                            .unwrap_or_else(|| "N/A".to_string());
                        println!(
                            "  [{}] Name: {:<18} | RSSI: {:<8} | ID: {}",
                            i + 1,
                            bot.name,
                            rssi_str,
                            bot.id
                        );
                    }
                }
            }
            println!();
        }
        Commands::Press { device, timeout } => {
            let peripheral =
                find_target_bot(device.as_deref(), Duration::from_secs(timeout)).await?;
            send_bot_command(&peripheral, BotCommand::Press).await?;
            println!("Successfully triggered Press.");
        }
        Commands::On { device, timeout } => {
            let peripheral =
                find_target_bot(device.as_deref(), Duration::from_secs(timeout)).await?;
            send_bot_command(&peripheral, BotCommand::On).await?;
            println!("Successfully triggered Turn On.");
        }
        Commands::Off { device, timeout } => {
            let peripheral =
                find_target_bot(device.as_deref(), Duration::from_secs(timeout)).await?;
            send_bot_command(&peripheral, BotCommand::Off).await?;
            println!("Successfully triggered Turn Off.");
        }
        Commands::Serve { host, port } => {
            run_server(&host, port).await?;
        }
    }

    Ok(())
}
