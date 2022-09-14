mod constants;
mod modules;

pub use crate::modules::*;
use crate::peer::{mode::PeerMode, BootstrapBuilder, PeerBuilder};

use anyhow::Result;
use bastion::prelude::*;
use clap::Parser;
use log::debug;
use logger::FileLoggerSettingBuilder;
use opts::Opts;

#[tokio::main]
async fn main() -> Result<()> {
  let opts = Opts::parse();
  let bastion_cfg = Config::new().hide_backtraces();

  Bastion::init_with(bastion_cfg);
  Bastion::start();

  let file_logger_builder = if opts.file_logging_enabled {
    Some(
      FileLoggerSettingBuilder::default()
        .level(opts.log_level_file)
        .name(opts.log_name.as_deref())
        .size(opts.log_size)
        .rotation(opts.log_rotation)
        .build(),
    )
  } else {
    None
  };

  logger::setup_logger(opts.log_level_cmd, file_logger_builder.as_ref())?;

  debug!("{file_logger_builder:?}");
  debug!("{opts:?}");

  let peer_buidler = match opts.peer_mode {
    PeerMode::Peer => match opts.key_seed {
      Some(seed) => PeerBuilder::default().local_key_with_seed(seed),
      None => PeerBuilder::default().local_key(),
    },
    PeerMode::Bootstrap => match opts.key_seed {
      Some(seed) => BootstrapBuilder::default().local_key_with_seed(seed),
      None => BootstrapBuilder::default().local_key(),
    },
  };

  let mut peer = peer_buidler.build().await?;
  peer.run().await?;

  Bastion::stop();
  Bastion::block_until_stopped();

  Ok(())
}
