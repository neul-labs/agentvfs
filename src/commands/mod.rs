//! CLI command implementations.

pub mod aliases;
pub mod audit;
pub mod cat;
pub mod checkpoint;
pub mod checkout;
pub mod compact;
pub mod cp;
pub mod diff;
pub mod exec;
pub mod export;
pub mod find;
pub mod gc;
pub mod grep;
pub mod import;
pub mod log;
pub mod ls;
pub mod maintain;
pub mod meta;
pub mod mkdir;
#[cfg(feature = "fuse")]
pub mod mount;
pub mod mv;
pub mod prune;
#[cfg(feature = "fuse")]
pub mod proxy;
pub mod quota;
pub mod revert;
pub mod rm;
pub mod search;
pub mod shell;
pub mod snapshot;
pub mod stats;
pub mod tag;
pub mod tree;
#[cfg(feature = "fuse")]
pub mod unmount;
pub mod untag;
pub mod vault;
pub mod write;

use serde::Serialize;

/// Output formatter for JSON or text output.
pub struct Output {
    json: bool,
}

impl Output {
    pub fn new(json: bool) -> Self {
        Self { json }
    }

    pub fn print<T: Serialize + std::fmt::Display>(&self, value: &T) {
        if self.json {
            match serde_json::to_string_pretty(value) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
        } else {
            println!("{}", value);
        }
    }

    pub fn print_json<T: Serialize>(&self, value: &T) {
        if self.json {
            match serde_json::to_string_pretty(value) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
        }
    }

    pub fn print_text(&self, text: &str) {
        if !self.json {
            println!("{}", text);
        }
    }

    pub fn print_error(&self, error: &crate::error::VfsError) {
        if self.json {
            println!("{}", error.to_json());
        } else {
            eprintln!("Error: {}", error);
        }
    }

    pub fn is_json(&self) -> bool {
        self.json
    }
}
