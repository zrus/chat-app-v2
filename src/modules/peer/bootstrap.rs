use std::io::{Error, ErrorKind};
use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::upgrade::{self, SelectUpgrade};
use libp2p::futures::StreamExt;
use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent, IdentifyInfo};
use libp2p::identity::Keypair;
use libp2p::kad::{store::MemoryStore, Kademlia, KademliaConfig};
use libp2p::mplex::MplexConfig;
use libp2p::multiaddr::Protocol;
use libp2p::noise;
use libp2p::ping::{Ping, PingConfig};
use libp2p::relay::v2::relay::Relay;
use libp2p::swarm::{Swarm, SwarmBuilder, SwarmEvent};
use libp2p::tcp::{GenTcpConfig, TokioTcpTransport};
use libp2p::yamux::{WindowUpdateMode, YamuxConfig};
use libp2p::Multiaddr;
use libp2p::PeerId;
use libp2p::Transport;
use log::{debug, error, info};
use tokio::time::Instant;

use crate::peer::event::Event;
use crate::traits::peer::{TBuilder, TPeer};

use super::super::helper::generate_ed25519;
use super::behaviour::BootstrapBehaviour;

const BOOTSTRAP_INTERVAL: Duration = Duration::from_secs(3 * 60);

pub struct Bootstrap {
  swarm: Swarm<BootstrapBehaviour>,
}

#[async_trait]
impl TPeer for Bootstrap {
  async fn run(&mut self) -> Result<()> {
    self.swarm.listen_on(
      Multiaddr::empty()
        .with("0.0.0.0".parse::<Ipv4Addr>().unwrap().into())
        .with(Protocol::Tcp(4003)),
    )?;

    let sleep = tokio::time::sleep(BOOTSTRAP_INTERVAL);
    tokio::pin!(sleep);

    loop {
      tokio::select! {
        () = &mut sleep => {
          sleep.as_mut().reset(Instant::now() + BOOTSTRAP_INTERVAL);
          let _ = self.swarm.behaviour_mut().kademlia.bootstrap();
        }
        event = self.swarm.select_next_some() => {
          match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {:?}", address);
            }
            SwarmEvent::Behaviour(Event::Identify(event)) => {
                info!("{:?}", event);
                if let IdentifyEvent::Received { peer_id, info: IdentifyInfo { listen_addrs, protocols, .. } } = event {
                  if protocols
                    .iter()
                    .any(|p| p.as_bytes() == libp2p::kad::protocol::DEFAULT_PROTO_NAME)
                  {
                    for addr in listen_addrs {
                      self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                    }
                  }
                };
            }
            SwarmEvent::Behaviour(Event::Ping(_)) => {}
            SwarmEvent::Behaviour(Event::Autonat(e)) => {
              info!("{e:?}");
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                info!("Established connection to {:?} via {:?}", peer_id, endpoint);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                error!("Outgoing connection error to {:?}: {:?}", peer_id, error);
            }
            event => debug!("Other: {event:?}"),
          }
        }
      }
    }
  }
}

#[derive(Default)]
pub struct BootstrapBuilder {
  local_key: Option<Keypair>,
  local_peer_id: Option<PeerId>,
}

impl BootstrapBuilder {
  pub fn local_key(mut self) -> Box<dyn TBuilder> {
    self.local_key = Some(Keypair::generate_ed25519());
    self.local_peer_id = Some(PeerId::from(&self.local_key.as_ref().unwrap().public()));
    Box::new(self)
  }

  pub fn local_key_with_seed(mut self, seed: u8) -> Box<dyn TBuilder> {
    self.local_key = Some(generate_ed25519(seed));
    self.local_peer_id = Some(PeerId::from(&self.local_key.as_ref().unwrap().public()));
    Box::new(self)
  }
}

#[async_trait]
impl TBuilder for BootstrapBuilder {
  async fn build(&self) -> Result<Box<dyn TPeer>> {
    let local_key = self.local_key.as_ref().unwrap();
    let local_peer_id = self.local_peer_id.unwrap();

    info!("Local peer id: {local_peer_id}");

    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
      .into_authentic(&local_key)
      .expect("Signing libp2p-noise static DH keypair failed.");

    let yamux_config = {
      let mut config = YamuxConfig::default();
      config.set_max_buffer_size(16 * 1024 * 1024);
      config.set_receive_window_size(16 * 1024 * 1024);
      config.set_window_update_mode(WindowUpdateMode::on_receive());
      config
    };

    let multiplex_upgrade = SelectUpgrade::new(yamux_config, MplexConfig::new());

    let transport = TokioTcpTransport::new(GenTcpConfig::default().nodelay(true))
      .upgrade(upgrade::Version::V1)
      .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
      .multiplex(multiplex_upgrade)
      .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
      .map_err(|err| Error::new(ErrorKind::Other, err))
      .boxed();
    let mut config = KademliaConfig::default();
    config
      .set_query_timeout(Duration::from_secs(10))
      .set_record_ttl(Some(Duration::from_secs(60)))
      .set_publication_interval(Some(Duration::from_secs(30)))
      .set_provider_record_ttl(Some(Duration::from_secs(20)))
      .set_provider_publication_interval(Some(Duration::from_secs(10)));
    let store = MemoryStore::new(local_peer_id);
    let kademlia = Kademlia::with_config(local_peer_id, store, config);

    let behaviour = BootstrapBehaviour {
      relay: Relay::new(PeerId::from(local_key.public()), Default::default()),
      ping: Ping::new(PingConfig::default().with_keep_alive(true)),
      identify: Identify::new(IdentifyConfig::new(
        "/TODO/0.0.1".to_string(),
        local_key.public(),
      )),
      kademlia,
    };

    let swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
      .executor(Box::new(|fut| {
        tokio::spawn(fut);
      }))
      .build();
    Ok(Box::new(Bootstrap { swarm }))
  }
}
