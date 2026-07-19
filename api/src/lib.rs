// Forbid `.unwrap()` in production code. Test modules (`#[cfg(test)]`) and the
// integration/bench crates are exempt, where panicking on bad setup is fine.
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod assess;
pub mod audit;
pub mod auth;
pub mod bank;
pub mod config;
pub mod domain;
pub mod engine;
pub mod http;
pub mod learning;
pub mod ratelimit;
pub mod settings;
pub mod templates;
