mod constants;
mod modules;

pub use crate::modules::*;
use crate::{
  constants::{BOOTNODES, KEY_SEEDS, PORTS},
  peer::{mode::PeerMode, BootstrapBuilder, PeerBuilder},
};

use anyhow::Result;
use bastion::prelude::*;
use clap::Parser;
use log::debug;
use logger::FileLoggerSettingBuilder;
use modules::traits::peer::TBuilder;
use opts::Opts;

#[tokio::main]
async fn main() -> Result<()> {
  let opts = Opts::parse();

  Bastion::init();
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
    PeerMode::Peer => {
      let builder = match opts.key_seed {
        Some(seed) => PeerBuilder::default().local_key_with_seed(seed),
        None => PeerBuilder::default().local_key(),
      };
      Vec::from([builder.boxed()])
    }
    PeerMode::Bootstrap => {
      let mut builders = Vec::new();

      for idx in 0..opts.number_of_boot_node {
        builders.push(
          BootstrapBuilder::default()
            .local_key_with_seed(KEY_SEEDS[idx])
            .port(PORTS[idx])
            .boxed(),
        )
      }

      builders
    }
  };

  for (idx, builder) in peer_buidler.iter().enumerate() {
    let mut peer = builder.build().await?;
    spawn!(async move {
      peer.run(&BOOTNODES[..idx]).await.expect("peer run failed");
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
  }

  Bastion::block_until_stopped();

  Ok(())
}
