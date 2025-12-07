//! RDPE Editor - Visual editor for particle simulations
//!
//! This crate provides:
//! - A visual editor for designing particle simulations
//! - Serializable configuration types
//! - A runner binary to execute saved configurations

pub mod config;
pub mod ui;

pub use config::*;
