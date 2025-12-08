//! RDPE Editor - Visual editor for particle simulations
//!
//! This crate provides:
//! - A visual editor for designing particle simulations
//! - Serializable configuration types
//! - Embedded simulation viewport for live editing
//! - A runner binary to execute saved configurations

pub mod code_export;
pub mod config;
#[cfg(feature = "egui")]
pub mod embedded;
pub mod particle;
pub mod shader_gen;
pub mod shader_validate;
pub mod spawn;
pub mod ui;

pub use code_export::generate_code;
pub use config::*;
pub use particle::MetaParticle;
pub use shader_gen::{generate_compute_shader, generate_render_shader};
pub use spawn::generate_particles;
#[cfg(feature = "egui")]
pub use embedded::{EmbeddedSimulation, SimulationResources, SimulationCallback, ParsedParticle};
