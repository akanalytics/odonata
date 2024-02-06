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

mod evaluation;
mod feature;
mod hardcoded_weights;
mod hce;
// mod material_balance;
mod scoring;
pub mod see;
mod weight;

#[cfg(test)]
extern crate test_log;

extern crate tracing;

pub use self::{
    evaluation::Evaluation,
    feature::{Feature, FeatureCategory},
    hce::Hce,
    scoring::{Scorer, Softcoded, SummationScorer, WeightVec},
    weight::{Weight, WeightOf},
};
