use std::str::FromStr;

use anyhow::anyhow;

#[derive(Debug)]
pub enum PeerMode {
  Peer,
  Bootstrap,
}

impl FromStr for PeerMode {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "bootstrap" => Ok(Self::Bootstrap),
      "peer" => Ok(Self::Peer),
      _ => Err(anyhow!("Peer mode is invalid.")),
    }
  }
}
