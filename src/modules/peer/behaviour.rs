use super::event::Event;
use libp2p::dcutr;
use libp2p::gossipsub::Gossipsub;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::Kademlia;
use libp2p::mdns::TokioMdns;
use libp2p::ping::Ping;
use libp2p::relay::v2::{client::Client, relay::Relay};
use libp2p::{identify::Identify, NetworkBehaviour};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", event_process = true)]
pub struct PeerBehaviour {
  pub client: Client,
  pub ping: Ping,
  pub identify: Identify,
  pub dcutr: dcutr::behaviour::Behaviour,
  pub kademlia: Kademlia<MemoryStore>,
  pub gossipsub: Gossipsub,
  pub mdns: TokioMdns,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", event_process = true)]
pub struct BootstrapBehaviour {
  pub relay: Relay,
  pub ping: Ping,
  pub identify: Identify,
  pub kademlia: Kademlia<MemoryStore>,
}
