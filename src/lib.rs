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

pub mod bootstrap;
pub mod cache;
pub mod client;
pub mod display;
pub mod error;
pub mod models;
pub mod request;

pub use client::RdapClient;
pub use error::{RdapError, Result};
pub use models::*;
pub use request::{QueryType, RdapRequest};
