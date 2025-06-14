//! High level API to communicate with discovery Turms service.
#![forbid(unsafe_code)]
#![deny(
    dead_code,
    unused_imports,
    unused_mut,
    missing_docs,
    missing_debug_implementations
)]

pub mod jwt;
pub mod models;
pub mod websocket;
#[macro_use]
mod macros;
