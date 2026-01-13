//! DNS module
//!
//! Contains DNS server implementations and related functionality.

mod cache;
mod message;
pub mod proxy;
mod resolver;
mod rewrite;
pub mod server;

pub use cache::*;
pub use message::*;
pub use proxy::*;
pub use resolver::*;
pub use rewrite::*;
