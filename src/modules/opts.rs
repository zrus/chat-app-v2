use clap::Parser;
use log::LevelFilter;

use super::peer::mode::PeerMode;

#[derive(Debug, Parser)]
#[clap(name = "Demo of Actor model + CQRS")]
pub struct Opts {
  /// Disable file logging.
  #[clap(long, short)]
  pub file_logging_enabled: bool,
  /// Log level for command line.
  #[clap(long, default_value = "info")]
  pub log_level_cmd: LevelFilter,
  /// Log level for files.
  #[clap(long, default_value = "info")]
  pub log_level_file: LevelFilter,
  /// Log name for files.
  #[clap(long)]
  pub log_name: Option<String>,
  /// Log file size.
  #[clap(long)]
  pub log_size: Option<u64>,
  /// Log file rotation.
  #[clap(long)]
  pub log_rotation: Option<u32>,
  /// Peer mode.
  #[clap(long, default_value = "peer")]
  pub peer_mode: PeerMode,
  /// Key seed.
  #[clap(long)]
  pub key_seed: Option<u8>,
}
