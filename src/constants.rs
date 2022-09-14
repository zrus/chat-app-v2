// LOGGER CONSTANTS
pub const LOG_DIR: &str = "logs";
pub const LOG_PATTERN: &str = "[{d(%d/%m/%Y %H:%M:%S%.6f %Z)}] {h({l})} {m}{n}";
pub const LOG_DEBUG_PATTERN: &str =
  "[{d(%d/%m/%Y %H:%M:%S%.6f %Z)}] from {f}:{L}{n}{h({l})} {m}{n}";

// BOOTSTRAP CONSTANTS
// *TODO: move to config file
pub const BOODSTRAP_ADDRESS: &str =
  "/ip4/3.19.56.240/tcp/4003/p2p/12D3KooWDfVV2caaXhXPsZti1wyZPtBj7kckpQ62oSCS3vxJuzyY";
pub const BOOT_NODES: &[&str] = &["12D3KooWDfVV2caaXhXPsZti1wyZPtBj7kckpQ62oSCS3vxJuzyY"];
