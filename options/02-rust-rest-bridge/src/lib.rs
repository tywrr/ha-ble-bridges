pub mod bot;
pub mod discovery;
pub mod server;

pub use bot::{
    send_bot_command, BotCommand, COMMAND_CHAR_UUID, NOTIFY_CHAR_UUID, SWITCHBOT_SERVICE_UUID,
};
pub use discovery::{find_target_bot, scan_bots, DiscoveredBot};
pub use server::{router, run_server};
