// LOGGER CONSTANTS
pub const LOG_DIR: &str = "logs";
pub const LOG_PATTERN: &str = "[{d(%d/%m/%Y %H:%M:%S%.6f %Z)}] {h({l})} {m}{n}";
pub const LOG_DEBUG_PATTERN: &str =
  "[{d(%d/%m/%Y %H:%M:%S%.6f %Z)}] from {f}:{L}{n}{h({l})} {m}{n}";

// BOOTSTRAP CONSTANTS
// *TODO: move to config file
pub const BOOTSTRAP_ADDRESS: &str = "/ip4/3.19.56.240/tcp/4003";
pub const BOOTNODES: &[&str] = &["12D3KooWDfVV2caaXhXPsZti1wyZPtBj7kckpQ62oSCS3vxJuzyY"];

// DEFAULT BOOTNODES
pub const PUBLIC_BOOTNODES: &[&str] = &[
  "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
  "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
  "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
  "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
];
pub const PUBLIC_BOOT_ADDR: &str = "/dnsaddr/bootstrap.libp2p.io";
