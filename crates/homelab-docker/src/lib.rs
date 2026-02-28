pub mod builder;
pub mod client;
pub mod containers;
pub mod labels;
pub mod logs;
pub mod network;

/// Docker network name used for all PaaS-managed containers and services.
pub const HOMELAB_NETWORK: &str = "homelab";
