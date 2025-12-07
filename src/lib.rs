//! # RDPE - Reaction Diffusion Particle Engine
//!
//! GPU-accelerated particle simulations with a simple, declarative API.
//!
//! RDPE handles all the GPU complexity (buffers, shaders, spatial hashing) so you can
//! focus on defining particle behavior through composable rules.
//!
//! ## Quick Start
//!
//! ```ignore
//! use rdpe::prelude::*;
//!
//! #[derive(Particle, Clone)]
//! struct Ball {
//!     position: Vec3,
//!     velocity: Vec3,
//! }
//!
//! fn main() {
//!     Simulation::<Ball>::new()
//!         .with_particle_count(10_000)
//!         .with_bounds(1.0)
//!         .with_spawner(|i, _| Ball {
//!             position: Vec3::new(0.0, 0.5, 0.0),
//!             velocity: Vec3::ZERO,
//!         })
//!         .with_rule(Rule::Gravity(9.8))
//!         .with_rule(Rule::BounceWalls)
//!         .run();
//! }
//! ```
//!
//! ## Core Concepts
//!
//! ### Particles
//!
//! Define your particle struct with `#[derive(Particle)]`. Required fields:
//! - `position: Vec3` - particle position in 3D space
//! - `velocity: Vec3` - particle velocity
//!
//! Optional fields:
//! - `#[color] color: Vec3` - custom particle color (RGB, 0.0-1.0)
//! - `particle_type: u32` - for typed interactions (auto-added if not present)
//! - Any other `f32`, `u32`, `i32`, `Vec2`, `Vec3`, `Vec4` fields
//!
//! ### Rules
//!
//! Rules define particle behavior. They execute every frame in order:
//!
//! ```ignore
//! .with_rule(Rule::Gravity(9.8))        // Apply forces
//! .with_rule(Rule::Separate { ... })    // Neighbor interactions
//! .with_rule(Rule::SpeedLimit { ... })  // Constrain velocity
//! .with_rule(Rule::Drag(1.0))           // Apply friction
//! .with_rule(Rule::BounceWalls)         // Boundary conditions
//! ```
//!
//! ### Typed Interactions
//!
//! Use [`ParticleType`] derive for type-safe particle categories:
//!
//! ```ignore
//! #[derive(ParticleType, Clone, Copy, PartialEq)]
//! enum Species {
//!     Prey,
//!     Predator,
//! }
//!
//! // Predators chase prey
//! Rule::Chase {
//!     self_type: Species::Predator.into(),
//!     target_type: Species::Prey.into(),
//!     radius: 0.3,
//!     strength: 2.0,
//! }
//! ```
//!
//! ## Spatial Hashing
//!
//! Neighbor-based rules (Separate, Cohere, Align, etc.) use spatial hashing
//! for efficient neighbor queries. Configure with:
//!
//! ```ignore
//! .with_spatial_config(cell_size, grid_resolution)
//! ```
//!
//! - `cell_size` should be >= your largest interaction radius
//! - `grid_resolution` must be a power of 2 (16, 32, 64)
//!
//! ## Feature Overview
//!
//! | Category | Rules |
//! |----------|-------|
//! | Physics | [`Rule::Gravity`], [`Rule::Drag`], [`Rule::Acceleration`] |
//! | Boundaries | [`Rule::BounceWalls`], [`Rule::WrapWalls`] |
//! | Forces | [`Rule::AttractTo`], [`Rule::RepelFrom`] |
//! | Movement | [`Rule::Wander`], [`Rule::SpeedLimit`] |
//! | Flocking | [`Rule::Separate`], [`Rule::Cohere`], [`Rule::Align`] |
//! | Collision | [`Rule::Collide`] |
//! | Types | [`Rule::Typed`], [`Rule::Convert`], [`Rule::Chase`], [`Rule::Evade`] |
//! | Custom | [`Rule::Custom`] for raw WGSL |

mod emitter;
pub mod field;
mod gpu;
pub mod input;
mod interactions;
pub mod lifecycle;
pub mod rules;
pub mod shader_utils;
mod simulation;
mod spawn;
mod spatial;
pub mod sub_emitter;
pub mod textures;
pub mod time;
mod uniforms;
pub mod visuals;
pub mod selection;

pub use bytemuck;
pub use emitter::Emitter;
pub use field::{FieldConfig, FieldRegistry, FieldType};
pub use glam::{Vec2, Vec3, Vec4};
pub use gpu::VolumeConfig;
pub use interactions::InteractionMatrix;
pub use lifecycle::Lifecycle;
pub use rdpe_derive::{MultiParticle, Particle, ParticleType};
pub use rules::{AgentState, Falloff, Rule, Transition};
pub use simulation::Simulation;
pub use spawn::SpawnContext;
pub use sub_emitter::{SpawnTrigger, SubEmitter};
pub use textures::{AddressMode, FilterMode, TextureConfig, TextureRegistry};
pub use uniforms::{CustomUniforms, UniformValue, UpdateContext};
pub use visuals::{BlendMode, ColorMapping, Palette, ParticleShape, VertexEffect, VisualConfig, WireframeMesh};

/// Trait automatically implemented by `#[derive(Particle)]`.
///
/// This trait bridges your Rust particle struct to GPU-compatible memory layout.
/// The derive macro generates:
/// - A companion `{Name}Gpu` struct with proper alignment/padding
/// - WGSL struct definition for compute shaders
/// - Conversion between Rust and GPU representations
///
/// # Do Not Implement Manually
///
/// This trait should only be derived, never implemented by hand.
/// The derive macro handles complex GPU memory layout requirements.
///
/// # Example
///
/// ```ignore
/// #[derive(Particle, Clone)]
/// struct MyParticle {
///     position: Vec3,           // Required
///     velocity: Vec3,           // Required
///     #[color]
///     color: Vec3,              // Optional: custom color
///     particle_type: u32,       // Optional: for typed rules
///     energy: f32,              // Optional: custom data
/// }
/// ```
pub trait ParticleTrait: Clone + Send + Sync {
    /// GPU-compatible representation with proper memory alignment.
    ///
    /// Generated automatically by the derive macro. Includes padding
    /// fields to satisfy WGSL alignment requirements (vec3 → 16-byte aligned).
    type Gpu: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Send + Sync;

    /// WGSL struct definition for use in compute shaders.
    ///
    /// Generated to match the GPU struct layout exactly.
    const WGSL_STRUCT: &'static str;

    /// Name of the field marked with `#[color]`, if any.
    ///
    /// Used by the renderer to determine which field contains particle color.
    /// If `None`, particles are colored based on position.
    const COLOR_FIELD: Option<&'static str>;

    /// Byte offset of the color field within the GPU struct.
    ///
    /// Used to configure vertex attributes for rendering.
    const COLOR_OFFSET: Option<u32>;

    /// Byte offset of the `alive` field within the GPU struct.
    ///
    /// Used to configure vertex attributes for culling dead particles.
    /// Always present since lifecycle fields are auto-injected.
    const ALIVE_OFFSET: u32;

    /// Byte offset of the `scale` field within the GPU struct.
    ///
    /// Used to configure vertex attributes for particle sizing.
    /// Always present since lifecycle fields are auto-injected.
    const SCALE_OFFSET: u32;

    /// Additional WGSL code prepended to shaders.
    ///
    /// Used by `MultiParticle` enums to inject type constants and helper functions.
    /// Regular particle structs leave this empty.
    ///
    /// Example for a multi-particle enum:
    /// ```wgsl
    /// const BOID: u32 = 0u;
    /// const PREDATOR: u32 = 1u;
    /// fn is_boid(p: Particle) -> bool { return p.particle_type == BOID; }
    /// ```
    const EXTRA_WGSL: &'static str = "";

    /// Convert this particle to its GPU representation.
    ///
    /// Called once per particle during initialization.
    fn to_gpu(&self) -> Self::Gpu;

    /// Convert from GPU representation back to this particle type.
    ///
    /// Used for CPU readback of particle data (e.g., for inspection).
    fn from_gpu(gpu: &Self::Gpu) -> Self;

    /// Get field names and their display values for inspection.
    ///
    /// Returns a vector of (field_name, formatted_value) pairs for all
    /// user-defined fields in the particle struct. Used by the built-in
    /// particle inspector panel.
    ///
    /// This is automatically generated by the derive macro and should not
    /// be implemented manually.
    fn inspect_fields(&self) -> Vec<(&'static str, String)>;

    /// Render editable UI widgets for all particle fields.
    ///
    /// Returns `true` if any field was modified. Used by the built-in
    /// particle inspector to allow live editing of particle values.
    ///
    /// This is automatically generated by the derive macro and should not
    /// be implemented manually.
    #[cfg(feature = "egui")]
    fn render_editable_fields(&mut self, ui: &mut egui::Ui) -> bool;
}

/// Convenient re-exports for common usage.
///
/// # Usage
///
/// ```ignore
/// use rdpe::prelude::*;
/// ```
///
/// This imports:
/// - [`Simulation`] - the simulation builder
/// - [`Rule`] - all available rules
/// - [`Emitter`] - particle emitters
/// - [`Particle`] - derive macro for particle structs
/// - [`ParticleType`] - derive macro for type enums
/// - [`Vec2`], [`Vec3`], [`Vec4`] - glam vector types
/// - [`ParticleTrait`] - the particle trait (rarely needed directly)
pub mod prelude {
    pub use crate::emitter::Emitter;
    pub use crate::field::{FieldConfig, FieldRegistry, FieldType};
    pub use crate::gpu::VolumeConfig;
    pub use crate::input::{Input, KeyCode, MouseButton};
    pub use crate::interactions::InteractionMatrix;
    pub use crate::lifecycle::Lifecycle;
    pub use crate::rules::{AgentState, Falloff, Rule, Transition};
    pub use crate::simulation::Simulation;
    pub use crate::spawn::SpawnContext;
    pub use crate::sub_emitter::{SpawnTrigger, SubEmitter};
    pub use crate::textures::{AddressMode, FilterMode, TextureConfig, TextureRegistry};
    pub use crate::time::Time;
    pub use crate::uniforms::{CustomUniforms, UpdateContext};
    pub use crate::visuals::{BlendMode, ColorMapping, Palette, ParticleShape, VertexEffect, VisualConfig, WireframeMesh};
    pub use crate::ParticleTrait;
    pub use crate::{Vec2, Vec3, Vec4};
    pub use rdpe_derive::{MultiParticle, Particle, ParticleType};
    #[cfg(feature = "egui")]
    pub use crate::selection::{selected_particle, selected_particle_data};
    #[cfg(feature = "egui")]
    pub use egui;
}
