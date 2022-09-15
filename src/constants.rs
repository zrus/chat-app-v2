// LOGGER CONSTANTS
pub const LOG_DIR: &str = "logs";
pub const LOG_PATTERN: &str = "[{d(%d/%m/%Y %H:%M:%S%.6f %Z)}] {h({l})} {m}{n}";
pub const LOG_DEBUG_PATTERN: &str =
  "[{d(%d/%m/%Y %H:%M:%S%.6f %Z)}] from {f}:{L}{n}{h({l})} {m}{n}";

// BOOTSTRAP CONSTANTS
// *TODO: move to config file
pub const BOOTSTRAP_ADDRESS: &str = "/ip4/3.19.56.240/tcp";
pub const BOOTNODES: &[&str] = &[
  "12D3KooWERHN2kX14rZBbCkKnLKdDzbQfFjA8NUTvHANSmsqbacA",
  "12D3KooWDfVV2caaXhXPsZti1wyZPtBj7kckpQ62oSCS3vxJuzyY",
  "12D3KooWMDoD3xyLF7g4N3a2krrBhW4gBuJ9TZaJ2vUVA5rmfFXt",
  "12D3KooWACdDu7PiwBBukn58ZSjmMKucbB1KvuYPGStzihqSkJVs",
];
pub const KEY_SEEDS: &[u8] = &[89, 134, 189, 234];
pub const PORTS: &[u16] = &[4003, 4043, 4344, 4443];
