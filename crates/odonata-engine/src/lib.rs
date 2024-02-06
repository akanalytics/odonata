// #![cfg_attr(debug_assertions, allow(dead_code))]
#![allow(dead_code)]
#![warn(clippy::all)]
#![warn(clippy::correctness)]
#![warn(clippy::style)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![allow(mixed_script_confusables)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]
#![allow(clippy::bool_assert_comparison)]



#[macro_use]
extern crate tracing;

pub use crate::tracing::{debug, error, event_enabled, info, log::Level, trace, warn};


#[cfg(test)]
extern crate test_log;



pub mod version;
pub mod engine;
pub mod cache;
pub mod comms;
pub mod search;
mod exam;
mod book;
pub mod eval;




