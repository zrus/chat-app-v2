use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;

use crate::constants::{BOOTSTRAP_ADDRESS, PORTS};
use crate::peer::event::Event;
use crate::traits::peer::{TBuilder, TPeer};
use anyhow::Result;
use async_trait::async_trait;
use libp2p::core::upgrade;
use libp2p::dns::DnsConfig;
use libp2p::futures::StreamExt;
use libp2p::gossipsub::{Gossipsub, GossipsubConfig, MessageAuthenticity};
use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent, IdentifyInfo};
use libp2p::identity::Keypair;
use libp2p::kad::{store::MemoryStore, Kademlia, KademliaConfig};
use libp2p::multiaddr::Protocol;
use libp2p::noise;
use libp2p::ping::{Ping, PingConfig};
use libp2p::relay::v2::relay::Relay;
use libp2p::swarm::{Swarm, SwarmBuilder, SwarmEvent};
use libp2p::tcp::{GenTcpConfig, TokioTcpTransport};
use libp2p::Multiaddr;
use libp2p::PeerId;
use libp2p::Transport;
use log::{debug, error, info};
use tokio::time::Instant;

use super::super::helper::generate_ed25519;
use super::behaviour::BootstrapBehaviour;

const BOOTSTRAP_INTERVAL: Duration = Duration::from_secs(3 * 60);

pub struct Bootstrap {
  swarm: Swarm<BootstrapBehaviour>,
  port: u16,
}

#[async_trait]
impl TPeer for Bootstrap {
  async fn run(&mut self, boot_nodes: &[&str]) -> Result<()> {
    let listen_addr = Multiaddr::empty()
      .with(Protocol::from(Ipv4Addr::UNSPECIFIED))
      .with(Protocol::Tcp(self.port));
    self.swarm.listen_on(listen_addr)?;

    info!("{boot_nodes:?}");
    for (idx, peer) in boot_nodes.iter().enumerate() {
      info!("{peer}");
      self.swarm.behaviour_mut().kademlia.add_address(
        &PeerId::from_str(peer)?,
        format!("{BOOTSTRAP_ADDRESS}/{}", PORTS[idx]).parse::<Multiaddr>()?,
      );
    }
    match self.swarm.behaviour_mut().kademlia.bootstrap() {
      Ok(_) => info!("bootstrapped!"),
      Err(_) => info!("no known servers"),
    }

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
            SwarmEvent::Behaviour(Event::Relay(event)) => {
              info!("{:?}", event)
            }
            SwarmEvent::Behaviour(Event::Identify(event)) => {
              info!("{:?}", event);
              if let IdentifyEvent::Received { peer_id, info: IdentifyInfo { ref listen_addrs, ref protocols, .. } } = event {
                if protocols
                  .iter()
                  .any(|p| p.as_bytes() == libp2p::kad::protocol::DEFAULT_PROTO_NAME)
                {
                  for addr in listen_addrs.iter().cloned() {
                    self.swarm
                      .behaviour_mut()
                      .kademlia
                      .add_address(&peer_id, addr);
                  }
                }

                if listen_addrs
                  .iter()
                  .any(|address| address.iter().any(|p| p == Protocol::P2pCircuit))
                {
                  println!("{:?}", event);
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
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
              info!("Connection to {peer_id} closed due to: {cause:?}");
              // self.swarm.behaviour_mut().kademlia.remove_peer(&peer_id);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error } => {
              error!("Outgoing connection error to {:?} due to: {:?}", peer_id, error);
              // if let Some(peer_id) = peer_id {
              //   self.swarm.behaviour_mut().kademlia.remove_peer(&peer_id);
              // }
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
  port: Option<u16>,
}

impl BootstrapBuilder {
  pub fn local_key(mut self) -> Self {
    self.local_key = Some(Keypair::generate_ed25519());
    self.local_peer_id = Some(PeerId::from(&self.local_key.as_ref().unwrap().public()));
    self
  }

  pub fn local_key_with_seed(mut self, seed: u8) -> Self {
    self.local_key = Some(generate_ed25519(seed));
    self.local_peer_id = Some(PeerId::from(&self.local_key.as_ref().unwrap().public()));
    self
  }

  pub fn port(mut self, port: u16) -> Self {
    self.port = Some(port);
    self
  }
}

#[async_trait]
impl TBuilder for BootstrapBuilder {
  fn boxed(self) -> Box<dyn TBuilder> {
    Box::new(self)
  }

  async fn build(&self) -> Result<Box<dyn TPeer>> {
    let local_key = self.local_key.as_ref().unwrap();
    let local_peer_id = self.local_peer_id.unwrap();

    info!("Local peer id: {local_peer_id}");

    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
      .into_authentic(&local_key)
      .expect("Signing libp2p-noise static DH keypair failed.");

    let transport = TokioTcpTransport::new(GenTcpConfig::default().nodelay(true));
    let transport = DnsConfig::system(transport).await?;
    let transport = transport
      .upgrade(upgrade::Version::V1)
      .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
      .multiplex(libp2p::yamux::YamuxConfig::default())
      .boxed();

    let mut config = KademliaConfig::default();
    config
      .set_query_timeout(Duration::from_secs(10))
      .set_connection_idle_timeout(Duration::from_secs(10))
      .set_record_ttl(Some(Duration::from_secs(120)))
      .set_publication_interval(None)
      .set_replication_interval(None)
      .set_provider_record_ttl(Some(Duration::from_secs(120)))
      .set_provider_publication_interval(None);
    let store = MemoryStore::new(local_peer_id);
    let kademlia = Kademlia::with_config(local_peer_id, store, config);

    let gossipsub = Gossipsub::new(
      MessageAuthenticity::Signed(local_key.clone()),
      GossipsubConfig::default(),
    )
    .expect("Valid config");

    let behaviour = BootstrapBehaviour {
      relay: Relay::new(PeerId::from(local_key.public()), Default::default()),
      ping: Ping::new(PingConfig::default().with_keep_alive(true)),
      identify: Identify::new(IdentifyConfig::new(
        "/TODO/0.0.1".to_string(),
        local_key.public(),
      )),
      kademlia,
      gossipsub,
    };

    let swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
      .executor(Box::new(|fut| {
        tokio::spawn(fut);
      }))
      .build();
    Ok(Box::new(Bootstrap {
      swarm,
      port: self.port.unwrap(),
    }))
  }
}
