use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use bastion::prelude::block_on;
use libp2p::core::{transport::OrTransport, upgrade};
use libp2p::dcutr;
use libp2p::dns::DnsConfig;
use libp2p::futures::StreamExt;
use libp2p::gossipsub::{self, GossipsubEvent, IdentTopic, MessageAuthenticity, ValidationMode};
use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent, IdentifyInfo};
use libp2p::identity::Keypair;
use libp2p::kad::{store::MemoryStore, Kademlia, KademliaConfig};
use libp2p::mdns::{MdnsEvent, TokioMdns};
use libp2p::multiaddr::Protocol;
use libp2p::noise;
use libp2p::ping::{Ping, PingConfig};
use libp2p::relay::v2::client::{self, Client};
use libp2p::swarm::{Swarm, SwarmBuilder, SwarmEvent};
use libp2p::tcp::{GenTcpConfig, TokioTcpTransport};
use libp2p::Multiaddr;
use libp2p::PeerId;
use libp2p::Transport;
use log::{debug, error, info, warn};
use tokio::io::AsyncBufReadExt;

use crate::constants::{BOOTSTRAP_ADDRESS, BOOT_NODES};
use crate::modules::peer::event::Event;
use crate::traits::peer::{TBuilder, TPeer};

use super::super::helper::generate_ed25519;
use super::behaviour::PeerBehaviour;

pub struct Peer {
  swarm: Swarm<PeerBehaviour>,
  topic: IdentTopic,
}

#[async_trait]
impl TPeer for Peer {
  async fn run(&mut self) -> Result<()> {
    self.swarm.listen_on(
      Multiaddr::empty()
        .with("0.0.0.0".parse::<Ipv4Addr>().unwrap().into())
        .with(Protocol::Tcp(0)),
    )?;

    loop {
      tokio::select! {
        event = self.swarm.select_next_some() => {
          match event {
            SwarmEvent::NewListenAddr { address, .. } => {
              info!("Listening on {:?}", address);
            }
            event => info!("{:?}", event),
          }
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
          // Likely listening on all interfaces now, thus continuing by breaking the loop.
          break;
        }
      }
    }

    self
      .swarm
      .dial(format!("{}/p2p/{}", BOOTSTRAP_ADDRESS, BOOT_NODES[0]).parse::<Multiaddr>()?)?;
    let mut learned_observed_addr = false;
    let mut told_relay_observed_addr = false;

    loop {
      match self.swarm.select_next_some().await {
        SwarmEvent::NewListenAddr { .. } => {}
        SwarmEvent::Dialing { .. } => {}
        SwarmEvent::ConnectionEstablished { .. } => {}
        SwarmEvent::Behaviour(Event::Ping(_)) => {}
        SwarmEvent::Behaviour(Event::Identify(IdentifyEvent::Sent { .. })) => {
          info!("Told relay its public address.");
          told_relay_observed_addr = true;
        }
        SwarmEvent::Behaviour(Event::Identify(IdentifyEvent::Received {
          info: IdentifyInfo { observed_addr, .. },
          ..
        })) => {
          info!("Relay told us our public address: {:?}", observed_addr);
          learned_observed_addr = true;
        }
        event => info!("{:?}", event),
      }

      if learned_observed_addr && told_relay_observed_addr {
        break;
      }
    }

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    loop {
      tokio::select! {
        line = stdin.next_line() => {
          let line = line?.expect("stdin closed");
          if let Err(e) = self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topic.clone(), line.as_bytes()) {
              error!("{e:?}");
          }
        }
        event = self.swarm.select_next_some() => {
          match event {
            SwarmEvent::Behaviour(Event::Gossipsub(GossipsubEvent::Message { message, .. })) => {
              info!(
                "Received: '{:?}' from {:?}",
                String::from_utf8_lossy(&message.data),
                message.source
              );
            }
            SwarmEvent::NewListenAddr { address, .. } => {
              info!("Listening on {:?}", address);
            }
            // SwarmEvent::Behaviour(Event::Mdns(event)) => {
            //   debug!("{event:?}");
            //   match event {
            //     MdnsEvent::Discovered(list) => {
            //       for (peer, _) in list {
            //         self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
            //       }
            //     }
            //     MdnsEvent::Expired(list) => {
            //       for (peer, _) in list {
            //         if !self.swarm.behaviour().mdns.has_node(&peer) {
            //           self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
            //         }
            //       }
            //     }
            //   }
            // },
            SwarmEvent::Behaviour(Event::Client(client::Event::ReservationReqAccepted {
                ..
            })) => {
                info!("Relay accepted our reservation request.");
            }
            SwarmEvent::Behaviour(Event::Client(event)) => {
                info!("{:?}", event)
            }
            SwarmEvent::Behaviour(Event::Dcutr(event)) => {
                info!("{:?}", event)
            }
            SwarmEvent::Behaviour(Event::Identify(event)) => {
                info!("Identify: {:?}", event);
                if let IdentifyEvent::Received {
                  peer_id,
                  info:
                    IdentifyInfo {
                      listen_addrs,
                      protocols,
                      ..
                    },
                } = event
                {
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
                }
            }
            SwarmEvent::Behaviour(Event::Ping(e)) => {
              warn!("Ping: {e:?}");
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                info!("Established connection to {:?} via {:?}", peer_id, endpoint);
                self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                debug!("Outgoing connection error to {peer_id:?}: {error:?}");
            }
            event => info!("Other: {event:?}"),
          }
        }
      }
    }
  }
}

#[derive(Default)]
pub struct PeerBuilder {
  local_key: Option<Keypair>,
  local_peer_id: Option<PeerId>,
}

impl PeerBuilder {
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
impl TBuilder for PeerBuilder {
  async fn build(&self) -> Result<Box<dyn TPeer>> {
    let local_key = self.local_key.as_ref().unwrap();
    let local_peer_id = self.local_peer_id.unwrap();

    info!("Local peer id: {local_peer_id}");

    let (relay_transport, client) = Client::new_transport_and_behaviour(local_peer_id);

    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
      .into_authentic(&local_key)
      .expect("Signing libp2p-noise static DH keypair failed.");

    let transport = OrTransport::new(
      relay_transport,
      block_on(DnsConfig::system(TokioTcpTransport::new(
        GenTcpConfig::default().nodelay(true).port_reuse(true),
      )))?,
    )
    .upgrade(upgrade::Version::V1)
    .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
    .multiplex(libp2p::yamux::YamuxConfig::default())
    .boxed();

    // Create a Gossipsub topic
    let topic = gossipsub::IdentTopic::new("chat");

    // Set mDNS
    // let mdns = TokioMdns::new(Default::default()).await?;

    // Set a custom gossipsub
    let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
      .heartbeat_interval(std::time::Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
      .flood_publish(true)
      .support_floodsub()
      .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
      .build()
      .expect("Valid config");

    // Build a gossipsub network behaviour
    let mut gossipsub: gossipsub::Gossipsub = gossipsub::Gossipsub::new(
      MessageAuthenticity::Signed(local_key.clone()),
      gossipsub_config,
    )
    .expect("Correct configuration");

    // Subscribes to our topic
    gossipsub.subscribe(&topic).unwrap();

    let mut config = KademliaConfig::default();
    config
      .set_query_timeout(Duration::from_secs(10))
      .set_connection_idle_timeout(Duration::from_secs(10))
      .set_record_ttl(Some(Duration::from_secs(120)))
      .set_publication_interval(Some(Duration::from_secs(90)))
      .set_provider_record_ttl(Some(Duration::from_secs(60)))
      .set_provider_publication_interval(Some(Duration::from_secs(30)));
    let store = MemoryStore::new(local_peer_id);
    let kademlia = Kademlia::with_config(local_peer_id, store, config);

    let mut behaviour = PeerBehaviour {
      client,
      ping: Ping::new(PingConfig::default().with_keep_alive(true)),
      identify: Identify::new(IdentifyConfig::new(
        "/TODO/0.0.1".to_string(),
        local_key.public(),
      )),
      dcutr: dcutr::behaviour::Behaviour::new(),
      gossipsub,
      // mdns,
      kademlia,
    };

    for peer in BOOT_NODES {
      behaviour.kademlia.add_address(
        &PeerId::from_str(peer)?,
        BOOTSTRAP_ADDRESS.parse::<Multiaddr>()?,
      );
    }

    behaviour.kademlia.bootstrap()?;

    let swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
      .executor(Box::new(|fut| {
        tokio::spawn(fut);
      }))
      .build();
    Ok(Box::new(Peer { swarm, topic }))
  }
}
