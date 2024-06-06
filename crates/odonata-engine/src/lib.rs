
#[macro_use]
extern crate tracing;

pub use crate::tracing::log::Level;
pub use crate::tracing::{debug, error, event_enabled, info, trace, warn};

#[cfg(test)]
extern crate test_log;

pub mod book;
pub mod cache;
pub mod comms;
pub mod eval;
pub mod search;
