//! Vault management module.

mod config;
mod manager;

pub use config::Config;
pub use manager::{ForkInfo, VaultInfo, VaultManager};
