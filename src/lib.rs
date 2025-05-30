#![forbid(unsafe_code)]
/*#![deny(
    dead_code,
    unused_imports,
    unused_mut,
    missing_docs,
    missing_debug_implementations
)]*/
//! Manage communication between Turms and client.

pub mod error;
mod future;
pub mod jwt;
pub mod models;
pub mod websocket;
