//! # RDAP Client Library
//!
//! A modern, elegant RDAP (Registration Data Access Protocol) client library
//! written in Rust.
//!
//! ## Features
//!
//! - Full RDAP protocol support (RFC 7480-7484)
//! - Automatic bootstrap service discovery
//! - Beautiful colored output
//! - Disk and memory caching
//! - Async/await support
//! - Type-safe JSON parsing
//! - Configurable bootstrap URLs
//! - Custom TLD overrides

pub mod bootstrap;
pub mod cache;
pub mod client;
pub mod config;
pub mod display;
pub mod error;
pub mod ip;
pub mod models;
pub mod net;
pub mod request;
#[cfg(feature = "server")]
pub mod server;

pub use client::RdapClient;
pub use config::Config;
pub use error::{RdapError, Result};
pub use models::*;
pub use net::NetOptions;
pub use request::{QueryType, RdapRequest};
