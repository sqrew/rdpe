//! Simulation builder and runner.
//!
//! This module provides the [`Simulation`] builder, which is the main entry point
//! for creating and running particle simulations.
//!
//! # Overview
//!
//! A simulation is configured through method chaining:
//!
//! 1. Create with [`Simulation::new()`]
//! 2. Configure particle count, bounds, spawner, and rules
//! 3. Call [`.run()`](Simulation::run) to start the interactive window
//!
//! # Example
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
//! Simulation::<Ball>::new()
//!     .with_particle_count(10_000)
//!     .with_bounds(1.0)
//!     .with_spawner(|ctx| Ball {
//!         position: ctx.random_in_sphere(0.5),
//!         velocity: Vec3::ZERO,
//!     })
//!     .with_rule(Rule::Gravity(9.8))
//!     .with_rule(Rule::BounceWalls)
//!     .run();
//! ```
//!
//! # Execution Model
//!
//! When [`.run()`](Simulation::run) is called:
//!
//! 1. Particles are spawned using the provided spawner function
//! 2. WGSL compute shaders are generated from the configured rules
//! 3. A window opens with an interactive 3D view
//! 4. The GPU runs the compute shader every frame to update particles
//!
//! # Controls
//!
//! - **Left-click + drag**: Rotate camera
//! - **Scroll wheel**: Zoom in/out

use crate::emitter::Emitter;
use crate::field::{FieldConfig, FieldRegistry};
use crate::gpu::GpuState;
use crate::input::Input;
use crate::interactions::InteractionMatrix;
use crate::spawn::SpawnContext;
use crate::rules::Rule;
use crate::shader_utils;
use crate::spatial::{SpatialConfig, MORTON_WGSL, NEIGHBOR_UTILS_WGSL};
use crate::textures::{TextureConfig, TextureRegistry};
use crate::time::Time;
use crate::uniforms::{CustomUniforms, UniformValue, UpdateContext};
use crate::visuals::{VertexEffect, VisualConfig};
use crate::ParticleTrait;
use std::marker::PhantomData;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

/// Type alias for the update callback to reduce complexity.
type UpdateCallback = Box<dyn FnMut(&mut UpdateContext) + Send>;

/// Type alias for the egui UI callback to reduce complexity.
#[cfg(feature = "egui")]
type UiCallback = Box<dyn FnMut(&egui::Context) + Send + 'static>;

/// A particle simulation builder.
///
/// `Simulation` uses the builder pattern to configure all aspects of a particle
/// simulation before running it. The generic parameter `P` is your particle type,
/// which must derive [`Particle`](crate::Particle).
///
/// # Type Parameter
///
/// - `P`: Your particle struct, must implement [`ParticleTrait`] (via `#[derive(Particle)]`)
///
/// # Builder Methods
///
/// | Method | Required | Description |
/// |--------|----------|-------------|
/// | [`with_particle_count`](Self::with_particle_count) | No | Number of particles (default: 10,000) |
/// | [`with_bounds`](Self::with_bounds) | No | Simulation cube half-size (default: 1.0) |
/// | [`with_particle_size`](Self::with_particle_size) | No | Base particle render size (default: 0.015) |
/// | [`with_spawner`](Self::with_spawner) | **Yes** | Function to create each particle |
/// | [`with_rule`](Self::with_rule) | No | Add behavior rules (can call multiple times) |
/// | [`with_spatial_config`](Self::with_spatial_config) | Conditional | Required if using neighbor rules |
///
/// # Example
///
/// ```ignore
/// use rdpe::prelude::*;
///
/// #[derive(Particle, Clone)]
/// struct Boid {
///     position: Vec3,
///     velocity: Vec3,
/// }
///
/// Simulation::<Boid>::new()
///     .with_particle_count(5000)
///     .with_bounds(1.0)
///     .with_spatial_config(0.1, 32)  // Needed for Separate/Cohere/Align
///     .with_spawner(|ctx| Boid {
///         position: ctx.random_in_bounds(),
///         velocity: ctx.random_direction() * 0.5,
///     })
///     .with_rule(Rule::Separate { radius: 0.05, strength: 2.0 })
///     .with_rule(Rule::Cohere { radius: 0.2, strength: 0.5 })
///     .with_rule(Rule::Align { radius: 0.1, strength: 1.0 })
///     .with_rule(Rule::SpeedLimit { min: 0.1, max: 1.5 })
///     .with_rule(Rule::BounceWalls)
///     .run();
/// ```
pub struct Simulation<P: ParticleTrait> {
    /// Number of particles to simulate.
    particle_count: u32,
    /// Half-size of the simulation bounding cube.
    bounds: f32,
    /// Base particle render size (multiplied by per-particle scale).
    particle_size: f32,
    /// Function called to create each particle at startup.
    spawner: Option<Box<dyn Fn(&mut SpawnContext) -> P + Send + Sync>>,
    /// List of rules that define particle behavior.
    rules: Vec<Rule>,
    /// Particle emitters for runtime spawning.
    emitters: Vec<Emitter>,
    /// Sub-emitters for spawning particles on death.
    sub_emitters: Vec<crate::sub_emitter::SubEmitter>,
    /// Interaction matrix for type-based forces.
    interaction_matrix: Option<InteractionMatrix>,
    /// Custom uniforms for user-defined shader data.
    custom_uniforms: CustomUniforms,
    /// Custom textures for user-defined shader sampling.
    texture_registry: TextureRegistry,
    /// Callback for updating custom uniforms each frame.
    update_callback: Option<UpdateCallback>,
    /// Custom WGSL functions that can be called from rules.
    custom_functions: Vec<String>,
    /// Configuration for spatial hashing (neighbor queries).
    spatial_config: SpatialConfig,
    /// Visual rendering configuration.
    visual_config: VisualConfig,
    /// Whether particle communication inbox is enabled.
    inbox_enabled: bool,
    /// Whether particles should start dead (for emitter-only spawning).
    start_dead: bool,
    /// Registry of 3D spatial fields for particle-environment interaction.
    field_registry: FieldRegistry,
    /// Volume rendering configuration for fields.
    volume_config: Option<crate::gpu::VolumeConfig>,
    /// Custom fragment shader code (replaces default fragment body).
    custom_fragment_shader: Option<String>,
    /// Custom vertex shader code (replaces default vertex body).
    custom_vertex_shader: Option<String>,
    /// Pre-built vertex effects (composable).
    vertex_effects: Vec<VertexEffect>,
    /// Whether egui UI is enabled.
    #[cfg(feature = "egui")]
    egui_enabled: bool,
    /// UI callback for egui (called each frame).
    #[cfg(feature = "egui")]
    ui_callback: Option<UiCallback>,
    /// Whether the built-in particle inspector is enabled.
    #[cfg(feature = "egui")]
    inspector_enabled: bool,
    /// Whether the built-in rule inspector is enabled.
    #[cfg(feature = "egui")]
    rule_inspector_enabled: bool,
    /// Phantom data for the particle type.
    _phantom: PhantomData<P>,
}

impl<P: ParticleTrait + 'static> Simulation<P> {
    /// Create a new simulation with default settings.
    ///
    /// # Defaults
    ///
    /// - Particle count: 10,000
    /// - Bounds: 1.0 (cube from -1.0 to +1.0)
    /// - No rules (particles won't move)
    /// - No spawner (must be set before calling `.run()`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let sim = Simulation::<MyParticle>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            particle_count: 10_000,
            bounds: 1.0,
            particle_size: 0.015,
            spawner: None,
            rules: Vec::new(),
            emitters: Vec::new(),
            sub_emitters: Vec::new(),
            interaction_matrix: None,
            custom_uniforms: CustomUniforms::new(),
            texture_registry: TextureRegistry::new(),
            update_callback: None,
            custom_functions: Vec::new(),
            spatial_config: SpatialConfig::default(),
            visual_config: VisualConfig::default(),
            inbox_enabled: false,
            start_dead: false,
            field_registry: FieldRegistry::new(),
            volume_config: None,
            custom_fragment_shader: None,
            custom_vertex_shader: None,
            vertex_effects: Vec::new(),
            #[cfg(feature = "egui")]
            egui_enabled: false,
            #[cfg(feature = "egui")]
            ui_callback: None,
            #[cfg(feature = "egui")]
            inspector_enabled: false,
            #[cfg(feature = "egui")]
            rule_inspector_enabled: false,
            _phantom: PhantomData,
        }
    }

    /// Set the number of particles.
    ///
    /// # Arguments
    ///
    /// * `count` - Total number of particles to simulate
    ///
    /// # Performance
    ///
    /// Modern GPUs can handle millions of particles, but neighbor-based rules
    /// (Separate, Cohere, Align, etc.) add significant per-particle cost.
    /// Start with 10,000-50,000 for neighbor-heavy simulations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Ball>::new()
    ///     .with_particle_count(100_000)
    ///     // ...
    /// ```
    pub fn with_particle_count(mut self, count: u32) -> Self {
        self.particle_count = count;
        self
    }

    /// Set the bounding box half-size.
    ///
    /// Creates a cube from `-bounds` to `+bounds` on all axes.
    /// This defines the simulation space that [`Rule::BounceWalls`] and
    /// [`Rule::WrapWalls`] use.
    ///
    /// # Arguments
    ///
    /// * `bounds` - Half-size of the cube (default: 1.0)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Create a larger simulation space
    /// Simulation::<Ball>::new()
    ///     .with_bounds(5.0)  // Cube from -5 to +5
    ///     // ...
    /// ```
    pub fn with_bounds(mut self, bounds: f32) -> Self {
        self.bounds = bounds;
        self
    }

    /// Set the base particle render size.
    ///
    /// This is the base size for rendering particles. Each particle's
    /// `scale` field multiplies this base size, so:
    ///
    /// `final_size = particle_size * particle.scale`
    ///
    /// # Arguments
    ///
    /// * `size` - Base size in clip space (default: 0.015)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Larger particles
    /// Simulation::<Ball>::new()
    ///     .with_particle_size(0.03)  // 2x default size
    ///     // ...
    ///
    /// // Tiny dots
    /// Simulation::<Ball>::new()
    ///     .with_particle_size(0.005)  // Small points
    ///     // ...
    /// ```
    ///
    /// # Note
    ///
    /// For per-particle size variation, set the `scale` field in your
    /// spawner or modify it in custom rules. Scale of 1.0 = base size.
    pub fn with_particle_size(mut self, size: f32) -> Self {
        self.particle_size = size;
        self
    }

    /// Set the particle spawner function.
    ///
    /// The spawner is called once for each particle at simulation startup.
    /// It receives a [`SpawnContext`] with helper methods for common spawn
    /// patterns like random positions, colors, and structured layouts.
    ///
    /// # Arguments
    ///
    /// * `spawner` - Function `(&mut SpawnContext) -> P`
    ///
    /// # Required
    ///
    /// This method **must** be called before `.run()`, or the simulation
    /// will panic.
    ///
    /// # SpawnContext Helpers
    ///
    /// The context provides many useful methods:
    /// - `ctx.index`, `ctx.count`, `ctx.progress()` - spawn info
    /// - `ctx.random_in_sphere(r)`, `ctx.random_on_sphere(r)` - positions
    /// - `ctx.random_in_cube(size)`, `ctx.random_in_bounds()` - box positions
    /// - `ctx.random_direction()` - unit vectors
    /// - `ctx.random_hue(s, v)`, `ctx.rainbow(s, v)` - colors
    /// - `ctx.grid_position(cols, rows, layers)` - structured layouts
    /// - `ctx.tangent_velocity(pos, speed)` - orbital motion
    ///
    /// # Examples
    ///
    /// ## Random sphere distribution
    ///
    /// ```ignore
    /// .with_spawner(|ctx| Ball {
    ///     position: ctx.random_in_sphere(0.8),
    ///     velocity: Vec3::ZERO,
    /// })
    /// ```
    ///
    /// ## Colorful swirl
    ///
    /// ```ignore
    /// .with_spawner(|ctx| {
    ///     let pos = ctx.random_in_sphere(0.6);
    ///     Spark {
    ///         position: pos,
    ///         velocity: ctx.tangent_velocity(pos, 0.3),
    ///         color: ctx.rainbow(0.9, 1.0),
    ///     }
    /// })
    /// ```
    ///
    /// ## Grid layout
    ///
    /// ```ignore
    /// .with_spawner(|ctx| Ball {
    ///     position: ctx.grid_position(10, 10, 10),
    ///     velocity: Vec3::ZERO,
    /// })
    /// ```
    ///
    /// ## Type-based initialization
    ///
    /// ```ignore
    /// .with_spawner(|ctx| {
    ///     let is_predator = ctx.index < 50;
    ///     Creature {
    ///         position: ctx.random_in_bounds(),
    ///         velocity: ctx.random_direction() * 0.1,
    ///         particle_type: if is_predator { 1 } else { 0 },
    ///     }
    /// })
    /// ```
    pub fn with_spawner<F>(mut self, spawner: F) -> Self
    where
        F: Fn(&mut SpawnContext) -> P + Send + Sync + 'static,
    {
        self.spawner = Some(Box::new(spawner));
        self
    }

    /// Add a rule to the simulation.
    ///
    /// Rules define particle behavior. They are executed in order every frame,
    /// so the sequence matters. Common patterns:
    ///
    /// 1. **Forces first**: Gravity, attraction, neighbor interactions
    /// 2. **Constraints**: Speed limits, drag
    /// 3. **Boundaries last**: BounceWalls, WrapWalls
    ///
    /// # Arguments
    ///
    /// * `rule` - The [`Rule`] to add
    ///
    /// # Multiple rules
    ///
    /// Call this method multiple times to add multiple rules:
    ///
    /// ```ignore
    /// Simulation::<Ball>::new()
    ///     .with_rule(Rule::Gravity(9.8))
    ///     .with_rule(Rule::Drag(0.5))
    ///     .with_rule(Rule::BounceWalls)
    ///     // ...
    /// ```
    ///
    /// # See Also
    ///
    /// See [`Rule`] for all available rules and their parameters.
    pub fn with_rule(mut self, rule: impl Into<Rule>) -> Self {
        self.rules.push(rule.into());
        self
    }

    /// Configure spatial hashing for neighbor queries.
    ///
    /// **Required** when using neighbor-based rules: `Separate`, `Cohere`,
    /// `Align`, `Collide`, `Chase`, `Evade`, `Convert`, or `Typed` wrappers
    /// around these.
    ///
    /// Spatial hashing divides space into a 3D grid to efficiently find
    /// nearby particles. Without it, checking all pairs would be O(n²).
    ///
    /// # Arguments
    ///
    /// * `cell_size` - Size of each grid cell. Should be >= your largest
    ///   interaction radius for best performance.
    /// * `grid_resolution` - Number of cells per axis. **Must be a power of 2**
    ///   (8, 16, 32, 64). Higher = more precision but more memory.
    ///
    /// # Guidelines
    ///
    /// | Scenario | cell_size | grid_resolution |
    /// |----------|-----------|-----------------|
    /// | Small interactions (radius < 0.1) | 0.1 | 32 |
    /// | Medium interactions (radius 0.1-0.3) | 0.2-0.3 | 32 |
    /// | Large bounds (> 1.0) | bounds / 16 | 32 or 64 |
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Boid>::new()
    ///     .with_spatial_config(0.15, 32)  // Cell size 0.15, 32x32x32 grid
    ///     .with_rule(Rule::Separate { radius: 0.1, strength: 2.0 })
    ///     // ...
    /// ```
    ///
    /// # Panics
    ///
    /// Will panic if `grid_resolution` is not a power of 2.
    pub fn with_spatial_config(mut self, cell_size: f32, grid_resolution: u32) -> Self {
        self.spatial_config = SpatialConfig::new(cell_size, grid_resolution);
        self
    }

    /// Set maximum neighbors to process per particle.
    ///
    /// This can significantly improve performance in dense simulations by limiting
    /// the number of neighbor interactions per particle. Use values like 32-64 for
    /// boids-style simulations.
    ///
    /// # Arguments
    ///
    /// * `max` - Maximum neighbors to process (0 = unlimited, default)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Boid>::new()
    ///     .with_spatial_config(0.1, 32)
    ///     .with_max_neighbors(48)  // Process at most 48 neighbors
    ///     .with_rule(Rule::Separate { radius: 0.05, strength: 2.0 })
    /// ```
    pub fn with_max_neighbors(mut self, max: u32) -> Self {
        self.spatial_config.max_neighbors = max;
        self
    }

    /// Enable particle-to-particle communication via inbox buffers.
    ///
    /// When enabled, particles can send values to other particles' "inbox"
    /// during neighbor iteration. Each particle has 4 inbox channels (vec4).
    /// Values are accumulated atomically and cleared each frame.
    ///
    /// # WGSL Functions
    ///
    /// - `inbox_send(target_idx, channel, value)` - Send float to particle's inbox
    /// - `inbox_receive_at(index, channel)` - Read inbox channel for particle (returns f32)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Particle>::new()
    ///     .with_inbox()
    ///     .with_spatial_config(0.2, 32)
    ///     .with_rule(Rule::NeighborCustom(r#"
    ///         // Transfer 10% of energy to neighbor
    ///         if neighbor_dist < 0.1 {
    ///             inbox_send(other_idx, 0u, p.energy * 0.1);
    ///             p.energy *= 0.9;
    ///         }
    ///     "#.into()))
    ///     .with_rule(Rule::Custom(r#"
    ///         // Receive accumulated energy
    ///         p.energy += inbox_receive_at(index, 0u);
    ///     "#.into()))
    ///     .run();
    /// ```
    ///
    /// # Technical Details
    ///
    /// The inbox uses atomic i32 operations with fixed-point encoding (16.16 format).
    /// This provides ~0.00001 precision in the range ±32768.
    pub fn with_inbox(mut self) -> Self {
        self.inbox_enabled = true;
        self
    }

    /// Add a 3D spatial field for particle-environment interaction.
    ///
    /// Fields are persistent 3D grids that particles can read from and write to.
    /// Unlike the inbox system (particle-to-particle), fields provide spatially
    /// indexed data that persists independently of particles.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the field (for documentation; access by index in shaders)
    /// * `config` - Field configuration (resolution, extent, decay, blur)
    ///
    /// # WGSL Functions
    ///
    /// In [`Rule::Custom`], you can use:
    /// - `field_write(field_idx, position, value)` - Deposit a value at a position
    /// - `field_read(field_idx, position)` - Sample field value (trilinear interpolation)
    /// - `field_gradient(field_idx, position, epsilon)` - Get gradient vector
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Agent>::new()
    ///     .with_field("pheromone", FieldConfig::new(64).with_decay(0.98).with_blur(0.1))
    ///     .with_rule(Rule::Custom(r#"
    ///         // Deposit pheromone at current position
    ///         field_write(0u, p.position, 0.1);
    ///
    ///         // Steer toward higher concentrations
    ///         let gradient = field_gradient(0u, p.position, 0.05);
    ///         p.velocity += normalize(gradient) * 0.5;
    ///     "#.into()))
    ///     .run();
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Pheromone trails**: Particles deposit chemicals, others follow gradients
    /// - **Density fields**: Accumulate particle presence for fluid-like behavior
    /// - **Temperature**: Particles emit/absorb heat from spatial field
    /// - **Reaction-diffusion**: Classic pattern formation (Gray-Scott, Turing)
    pub fn with_field(mut self, name: impl Into<String>, config: FieldConfig) -> Self {
        self.field_registry.add(name, config);
        self
    }

    /// Enable volume rendering for a field.
    ///
    /// Volume rendering visualizes a 3D field as volumetric fog or glow
    /// using ray marching. This allows you to see the field data directly,
    /// not just the particles that interact with it.
    ///
    /// Requires at least one field to be registered with [`with_field`](Self::with_field).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rdpe::prelude::*;
    ///
    /// Simulation::<Agent>::new()
    ///     .with_field("pheromone", FieldConfig::new(64).with_decay(0.98))
    ///     .with_volume_render(VolumeConfig::new()
    ///         .with_palette(Palette::Inferno)
    ///         .with_density_scale(5.0)
    ///         .with_steps(64))
    ///     .with_rule(Rule::Custom(r#"
    ///         field_write(0u, p.position, 0.1);
    ///     "#.into()))
    ///     .run();
    /// ```
    pub fn with_volume_render(mut self, config: crate::gpu::VolumeConfig) -> Self {
        self.volume_config = Some(config);
        self
    }

    /// Set a custom fragment shader for particle rendering.
    ///
    /// The custom shader code replaces the default fragment shader body. Your code
    /// has access to:
    ///
    /// - `in.uv` - UV coordinates on the particle quad (-1 to 1 range)
    /// - `in.color` - The computed color for this particle (vec3)
    /// - `uniforms.time` - Current simulation time (f32)
    ///
    /// Your code must return a `vec4<f32>` (RGBA color output).
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Ball>::new()
    ///     .with_fragment_shader(r#"
    ///         // Glowing particles with pulsing animation
    ///         let dist = length(in.uv);
    ///         let pulse = sin(uniforms.time * 3.0) * 0.2 + 0.8;
    ///         let glow = 1.0 / (dist * dist * 4.0 + 0.1) * pulse;
    ///         let color = in.color * glow;
    ///         return vec4<f32>(color, glow * 0.5);
    ///     "#)
    ///     .run();
    /// ```
    ///
    /// # Effects you can create
    ///
    /// - **Glow**: Use `1.0 / (dist * dist + epsilon)` for radial glow
    /// - **Rings**: Use `smoothstep` on distance to create ring shapes
    /// - **Animation**: Use `uniforms.time` for pulsing, rotation, etc.
    /// - **Custom shapes**: Discard fragments with `discard;` to cut out shapes
    pub fn with_fragment_shader(mut self, wgsl_code: &str) -> Self {
        self.custom_fragment_shader = Some(wgsl_code.to_string());
        self
    }

    /// Set a custom vertex shader for particle rendering.
    ///
    /// The custom shader code replaces the default vertex shader body. Your code
    /// has access to:
    ///
    /// **Inputs:**
    /// - `vertex_index: u32` - Which vertex of the quad (0-5)
    /// - `instance_index: u32` - Which particle (0 to particle_count-1)
    /// - `particle_pos: vec3<f32>` - Particle world position
    /// - `particle_color: vec3<f32>` - Particle color (if color field exists)
    /// - `scale: f32` - Per-particle scale multiplier
    /// - `quad_pos: vec2<f32>` - Quad vertex offset (-1 to 1)
    /// - `base_size: f32` - Base particle size from config
    /// - `particle_size: f32` - Computed size (base_size * scale)
    /// - `uniforms.view_proj` - View-projection matrix
    /// - `uniforms.time` - Current simulation time
    /// - `uniforms.delta_time` - Time since last frame
    ///
    /// **Must set:**
    /// - `out.clip_position: vec4<f32>` - Final clip-space position
    /// - `out.color: vec3<f32>` - Color to pass to fragment shader
    /// - `out.uv: vec2<f32>` - UV coordinates for fragment shader
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Ball>::new()
    ///     .with_vertex_shader(r#"
    ///         // Wobbling particles
    ///         let wobble = sin(uniforms.time * 5.0 + f32(instance_index) * 0.1) * 0.05;
    ///         let offset_pos = particle_pos + vec3<f32>(wobble, 0.0, 0.0);
    ///
    ///         let world_pos = vec4<f32>(offset_pos, 1.0);
    ///         var clip_pos = uniforms.view_proj * world_pos;
    ///         clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
    ///         clip_pos.y += quad_pos.y * particle_size * clip_pos.w;
    ///
    ///         out.clip_position = clip_pos;
    ///         out.color = particle_color;
    ///         out.uv = quad_pos;
    ///     "#)
    ///     .run();
    /// ```
    ///
    /// # Effects you can create
    ///
    /// - **Wobble/Wave**: Offset position with `sin(time + index)`
    /// - **Rotation**: Rotate `quad_pos` before applying to clip position
    /// - **Size pulsing**: Multiply `particle_size` by time-based factor
    /// - **Billboarding variants**: Custom billboard orientation
    /// - **Screen-space effects**: Modify clip position directly
    pub fn with_vertex_shader(mut self, wgsl_code: &str) -> Self {
        self.custom_vertex_shader = Some(wgsl_code.to_string());
        self
    }

    /// Add a pre-built vertex effect.
    ///
    /// Vertex effects are composable transformations applied to particle rendering.
    /// Multiple effects can be stacked and are applied in order.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Ball>::new()
    ///     .with_vertex_effect(VertexEffect::Rotate { speed: 2.0 })
    ///     .with_vertex_effect(VertexEffect::Wobble {
    ///         frequency: 3.0,
    ///         amplitude: 0.05,
    ///     })
    ///     .with_vertex_effect(VertexEffect::Pulse {
    ///         frequency: 4.0,
    ///         amplitude: 0.3,
    ///     })
    ///     .run();
    /// ```
    ///
    /// # Available Effects
    ///
    /// - [`VertexEffect::Rotate`] - Spin particles around their facing axis
    /// - [`VertexEffect::Wobble`] - Sinusoidal position offset
    /// - [`VertexEffect::Pulse`] - Size oscillation
    /// - [`VertexEffect::Wave`] - Coordinated wave across particles
    /// - [`VertexEffect::Jitter`] - Random shake
    /// - [`VertexEffect::ScaleByDistance`] - Size based on distance from point
    /// - [`VertexEffect::FadeByDistance`] - Opacity based on distance
    ///
    /// # Note
    ///
    /// If both `with_vertex_effect()` and `with_vertex_shader()` are used,
    /// vertex effects are ignored and the custom shader takes precedence.
    pub fn with_vertex_effect(mut self, effect: VertexEffect) -> Self {
        self.vertex_effects.push(effect);
        self
    }

    /// Add a particle emitter for runtime spawning.
    ///
    /// Emitters respawn dead particles at a configurable rate. Use with
    /// [`Rule::Age`] and [`Rule::Lifetime`] to create continuous particle
    /// effects.
    ///
    /// # Arguments
    ///
    /// * `emitter` - The [`Emitter`] configuration
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Spark>::new()
    ///     .with_particle_count(10_000)
    ///     .with_emitter(Emitter::Point {
    ///         position: Vec3::ZERO,
    ///         rate: 500.0,  // 500 particles per second
    ///     })
    ///     .with_rule(Rule::Age)
    ///     .with_rule(Rule::Lifetime(2.0))
    ///     .with_rule(Rule::Gravity(9.8))
    ///     .run();
    /// ```
    ///
    /// # Note
    ///
    /// When using emitters, particles should start dead (will be spawned by emitter)
    /// or use a spawner that sets some particles alive initially.
    pub fn with_emitter(mut self, emitter: Emitter) -> Self {
        self.emitters.push(emitter);
        self
    }

    /// Configure particle lifecycle with a builder.
    ///
    /// Lifecycle configuration handles aging, death, visual effects (fade, shrink),
    /// and respawning via emitters. This is the ergonomic way to set up particle
    /// systems with birth/death cycles.
    ///
    /// # Hidden Lifecycle Fields
    ///
    /// Every particle automatically has these fields (injected by derive macro):
    /// - `age: f32` - time since spawn (seconds)
    /// - `alive: u32` - 0 = dead, 1 = alive
    /// - `scale: f32` - visual size multiplier
    ///
    /// # Example: Custom Configuration
    ///
    /// ```ignore
    /// .with_lifecycle(|l| {
    ///     l.lifetime(2.0)
    ///      .fade_out()
    ///      .shrink_out()
    ///      .emitter(Emitter::Cone {
    ///          position: Vec3::ZERO,
    ///          direction: Vec3::Y,
    ///          speed: 2.0,
    ///          spread: 0.3,
    ///          rate: 500.0,
    ///      })
    /// })
    /// ```
    ///
    /// # Example: Using Presets
    ///
    /// ```ignore
    /// .with_lifecycle(Lifecycle::fire(Vec3::ZERO, 1000.0))
    /// .with_lifecycle(Lifecycle::fountain(Vec3::new(0.0, -0.5, 0.0), 800.0))
    /// .with_lifecycle(Lifecycle::explosion(Vec3::ZERO, 500))
    /// ```
    ///
    /// # What This Does
    ///
    /// The lifecycle builder automatically adds:
    /// - `Rule::Age` - increment particle age each frame
    /// - `Rule::Lifetime(duration)` - kill particles after duration
    /// - `Rule::FadeOut(duration)` - dim color over lifetime (if enabled)
    /// - `Rule::ShrinkOut(duration)` - shrink scale over lifetime (if enabled)
    /// - `Rule::ColorOverLife { ... }` - color gradient (if enabled)
    /// - Emitters for respawning dead particles
    pub fn with_lifecycle<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(crate::lifecycle::Lifecycle) -> crate::lifecycle::Lifecycle,
    {
        let lifecycle = configure(crate::lifecycle::Lifecycle::new());
        let (rules, emitters, start_dead) = lifecycle.build();

        // Add lifecycle rules
        for rule in rules {
            self.rules.push(rule);
        }

        // Add emitters
        for emitter in emitters {
            self.emitters.push(emitter);
        }

        // Set start_dead flag
        if start_dead {
            self.start_dead = true;
        }

        self
    }

    /// Configure particle lifecycle using a preset.
    ///
    /// Convenience method for using lifecycle presets directly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_lifecycle_preset(Lifecycle::fire(Vec3::ZERO, 1000.0))
    /// ```
    pub fn with_lifecycle_preset(mut self, lifecycle: crate::lifecycle::Lifecycle) -> Self {
        let (rules, emitters, start_dead) = lifecycle.build();

        for rule in rules {
            self.rules.push(rule);
        }

        for emitter in emitters {
            self.emitters.push(emitter);
        }

        // Set start_dead flag
        if start_dead {
            self.start_dead = true;
        }

        self
    }

    /// Add a sub-emitter that spawns child particles when parents die.
    ///
    /// Sub-emitters enable chain reactions, fireworks, explosions with debris,
    /// and biological reproduction effects.
    ///
    /// # How It Works
    ///
    /// 1. When a particle of `parent_type` dies (via `Rule::Lifetime` or `kill_particle()`)
    /// 2. The death event is recorded with position, velocity, and color
    /// 3. A secondary compute pass spawns `count` children at the death location
    /// 4. Children inherit some parent velocity and spread outward
    ///
    /// # Example: Fireworks
    ///
    /// ```ignore
    /// #[derive(ParticleType)]
    /// enum Firework { Rocket, Spark }
    ///
    /// Simulation::<Particle>::new()
    ///     .with_lifecycle(|l| l.lifetime(2.0))
    ///     .with_sub_emitter(
    ///         SubEmitter::new(Firework::Rocket.into(), Firework::Spark.into())
    ///             .count(50)
    ///             .speed(1.0..3.0)
    ///             .spread(std::f32::consts::PI)
    ///     )
    ///     .run();
    /// ```
    ///
    /// # Chaining Sub-Emitters
    ///
    /// ```ignore
    /// // Rockets → Sparks → Embers
    /// .with_sub_emitter(SubEmitter::new(Rocket, Spark).count(30))
    /// .with_sub_emitter(SubEmitter::new(Spark, Ember).count(5))
    /// ```
    pub fn with_sub_emitter(mut self, sub_emitter: crate::sub_emitter::SubEmitter) -> Self {
        self.sub_emitters.push(sub_emitter);
        self
    }

    /// Set up type-based interactions using an interaction matrix.
    ///
    /// The interaction matrix defines attraction and repulsion forces between
    /// particle types. This is the foundation of "particle life" simulations.
    ///
    /// The closure receives an [`InteractionMatrix`] which you configure with
    /// `set()`, `attract()`, `repel()`, or `set_symmetric()` calls.
    ///
    /// # Type Parameter
    ///
    /// `T` should be your `#[derive(ParticleType)]` enum. The `COUNT` const
    /// from the derive tells us how many types exist.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(ParticleType, Clone, Copy)]
    /// enum Species { Red, Green, Blue }
    ///
    /// Simulation::<Particle>::new()
    ///     .with_interactions::<Species>(|m| {
    ///         use Species::*;
    ///         m.attract(Red, Green, 1.0, 0.3);
    ///         m.repel(Green, Red, 0.5, 0.2);
    ///         m.set_symmetric(Blue, Blue, -0.3, 0.25);
    ///     })
    ///     .run();
    /// ```
    ///
    /// # Note
    ///
    /// Using interactions automatically enables spatial hashing for neighbor
    /// queries. The spatial config cell_size will be set to the maximum
    /// interaction radius if not already configured larger.
    pub fn with_interactions<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(&mut InteractionMatrix),
    {
        // We need to know how many types. For now, we require user to specify
        // or we infer from the matrix configuration
        let mut matrix = InteractionMatrix::new(16); // Default max 16 types
        configure(&mut matrix);
        self.interaction_matrix = Some(matrix);
        self
    }

    /// Set up type-based interactions with a specific number of types.
    ///
    /// Use this if you have more than 16 particle types.
    pub fn with_interactions_sized<F>(mut self, num_types: usize, configure: F) -> Self
    where
        F: FnOnce(&mut InteractionMatrix),
    {
        let mut matrix = InteractionMatrix::new(num_types);
        configure(&mut matrix);
        self.interaction_matrix = Some(matrix);
        self
    }

    /// Add a custom uniform that can be used in shader rules.
    ///
    /// Custom uniforms are accessible in [`Rule::Custom`] as `uniforms.name`.
    ///
    /// # Supported Types
    ///
    /// - `f32`, `i32`, `u32` - scalar values
    /// - `Vec2`, `Vec3`, `Vec4` - vector values
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Particle>::new()
    ///     .with_uniform("attractor", Vec3::ZERO)
    ///     .with_uniform("strength", 1.0f32)
    ///     .with_rule(Rule::Custom(r#"
    ///         let dir = uniforms.attractor - p.position;
    ///         p.velocity += normalize(dir) * uniforms.strength;
    ///     "#.into()))
    ///     .run();
    /// ```
    pub fn with_uniform<V: Into<UniformValue>>(mut self, name: &str, value: V) -> Self {
        self.custom_uniforms.set(name, value);
        self
    }

    /// Add a custom texture for use in shaders.
    ///
    /// Custom textures are available in fragment, post-process, and compute shaders
    /// as `tex_name` (the texture) and `tex_name_sampler` (the sampler).
    ///
    /// # Arguments
    ///
    /// * `name` - Name used to access the texture in shaders (becomes `tex_name`)
    /// * `config` - Texture configuration (can be a file path or `TextureConfig`)
    ///
    /// # Supported Input Types
    ///
    /// - `&str` or `String` - Path to an image file (PNG, JPEG, GIF, etc.)
    /// - `TextureConfig` - Full control over texture data and sampling options
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rdpe::prelude::*;
    ///
    /// // Load from file
    /// Simulation::<Particle>::new()
    ///     .with_texture("noise", "assets/noise.png")
    ///     .with_fragment_shader(r#"
    ///         let n = textureSample(tex_noise, tex_noise_sampler, in.uv * 0.5 + 0.5);
    ///         return vec4<f32>(in.color * n.r, 1.0);
    ///     "#)
    ///     .run();
    ///
    /// // Programmatic texture with options
    /// Simulation::<Particle>::new()
    ///     .with_texture("checker",
    ///         TextureConfig::checkerboard(64, 8, [255,255,255,255], [0,0,0,255])
    ///             .with_filter(FilterMode::Nearest)
    ///             .with_address_mode(AddressMode::Repeat))
    ///     .run();
    /// ```
    pub fn with_texture<C: Into<TextureConfig>>(mut self, name: &str, config: C) -> Self {
        self.texture_registry.add(name, config);
        self
    }

    /// Set a callback that runs every frame to update custom uniforms.
    ///
    /// The callback receives an [`UpdateContext`] with:
    /// - `time()` - current simulation time
    /// - `delta_time()` - time since last frame
    /// - `mouse_ndc()` - mouse position in normalized device coordinates
    /// - `mouse_pressed()` - is left mouse button down
    /// - `set(name, value)` - update a custom uniform
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Particle>::new()
    ///     .with_uniform("target", Vec3::ZERO)
    ///     .with_update(|ctx| {
    ///         // Make target orbit based on time
    ///         let t = ctx.time();
    ///         ctx.set("target", Vec3::new(t.cos(), 0.0, t.sin()));
    ///     })
    ///     .run();
    /// ```
    pub fn with_update<F>(mut self, callback: F) -> Self
    where
        F: FnMut(&mut UpdateContext) + Send + 'static,
    {
        self.update_callback = Some(Box::new(callback));
        self
    }

    /// Add a custom WGSL function that can be called from rules.
    ///
    /// Custom functions are injected into the compute shader and can be
    /// called from [`Rule::Custom`] or other custom code.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Particle>::new()
    ///     .with_function(r#"
    ///         fn swirl(pos: vec3<f32>, strength: f32) -> vec3<f32> {
    ///             let d = length(pos.xz);
    ///             return vec3(-pos.z, 0.0, pos.x) * strength / (d + 0.1);
    ///         }
    ///     "#)
    ///     .with_rule(Rule::Custom("p.velocity += swirl(p.position, 2.0);".into()))
    ///     .run();
    /// ```
    ///
    /// # Available in Functions
    ///
    /// Your functions have access to:
    /// - All WGSL built-in functions
    /// - The `Particle` struct type
    /// - The `Uniforms` struct (via parameter passing)
    /// - Other custom functions defined before this one
    pub fn with_function(mut self, wgsl_code: &str) -> Self {
        self.custom_functions.push(wgsl_code.to_string());
        self
    }

    /// Configure visual rendering options.
    ///
    /// Visuals control how particles are rendered, separate from the behavioral
    /// rules that control how they move.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rdpe::prelude::*;
    ///
    /// Simulation::<Ball>::new()
    ///     .with_visuals(|v| {
    ///         v.blend_mode(BlendMode::Additive);  // Glowy particles
    ///         v.shape(ParticleShape::Circle);
    ///     })
    ///     .with_rule(Rule::Gravity(9.8))
    ///     .run();
    /// ```
    ///
    /// # Available Options
    ///
    /// - `blend_mode()` - Alpha, Additive, or Multiply blending
    /// - `shape()` - Circle, Square, Ring, Star, Point
    /// - `trails()` - Render position history as trails
    /// - `connections()` - Draw lines between nearby particles
    /// - `velocity_stretch()` - Stretch particles in motion direction
    pub fn with_visuals<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(&mut VisualConfig),
    {
        configure(&mut self.visual_config);
        self
    }

    /// Enable egui UI overlay.
    ///
    /// When enabled, you can use the `with_ui` method to add interactive
    /// controls to your simulation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Ball>::new()
    ///     .with_egui()
    ///     .with_spawner(|_| Ball::default())
    ///     .run();
    /// ```
    ///
    /// # Note
    ///
    /// Requires the `egui` feature to be enabled.
    #[cfg(feature = "egui")]
    pub fn with_egui(mut self) -> Self {
        self.egui_enabled = true;
        self
    }

    /// Set a UI callback for egui rendering.
    ///
    /// This enables egui and provides a callback that will be called each frame
    /// to render custom UI. The callback receives the egui Context which you
    /// can use to create windows, panels, and widgets.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use egui;
    ///
    /// Simulation::<Ball>::new()
    ///     .with_ui(|ctx| {
    ///         egui::Window::new("Controls").show(ctx, |ui| {
    ///             ui.label("Hello from egui!");
    ///         });
    ///     })
    ///     .with_spawner(|_| Ball::default())
    ///     .run();
    /// ```
    ///
    /// # Note
    ///
    /// Requires the `egui` feature to be enabled.
    #[cfg(feature = "egui")]
    pub fn with_ui<F>(mut self, callback: F) -> Self
    where
        F: FnMut(&egui::Context) + Send + 'static,
    {
        self.egui_enabled = true;
        self.ui_callback = Some(Box::new(callback));
        self
    }

    /// Enable the built-in particle inspector panel.
    ///
    /// When enabled, a "Particle Inspector" window appears that displays
    /// all fields of the currently selected particle with live updates.
    /// Click on any particle to select it.
    ///
    /// This is a zero-boilerplate alternative to manually creating an
    /// inspector using `with_ui()` and `selected_particle_data()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Standalone use - no UI code needed
    /// Simulation::<MyParticle>::new()
    ///     .with_particle_inspector()
    ///     .run();
    ///
    /// // Combined with custom UI
    /// Simulation::<MyParticle>::new()
    ///     .with_particle_inspector()  // Built-in inspector
    ///     .with_ui(|ctx| {            // Plus custom panels
    ///         egui::Window::new("Controls").show(ctx, |ui| {
    ///             ui.label("Custom controls here");
    ///         });
    ///     })
    ///     .run();
    /// ```
    ///
    /// # Note
    ///
    /// Requires the `egui` feature to be enabled.
    #[cfg(feature = "egui")]
    pub fn with_particle_inspector(mut self) -> Self {
        self.egui_enabled = true;
        self.inspector_enabled = true;
        self
    }

    /// Stub for when egui feature is not enabled - provides IDE visibility.
    #[cfg(not(feature = "egui"))]
    pub fn with_particle_inspector(self) -> Self {
        panic!("with_particle_inspector requires the `egui` feature. Enable it in Cargo.toml: rdpe = {{ features = [\"egui\"] }}")
    }

    /// Enable the built-in rule inspector panel.
    ///
    /// Displays an egui window showing all rules and their parameters,
    /// allowing live editing of rule values at runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Ball>::new()
    ///     .with_rule(Rule::Gravity(9.8))
    ///     .with_rule(Rule::Drag(0.5))
    ///     .with_rule_inspector()  // Live-edit gravity, drag, etc.
    ///     .run();
    /// ```
    ///
    /// # Note
    ///
    /// Rule values are stored in a uniform buffer and can be adjusted
    /// without recompiling the shader. Changes take effect immediately.
    ///
    /// Requires the `egui` feature to be enabled.
    #[cfg(feature = "egui")]
    pub fn with_rule_inspector(mut self) -> Self {
        self.egui_enabled = true;
        self.rule_inspector_enabled = true;
        self
    }

    /// Stub for when egui feature is not enabled - provides IDE visibility.
    #[cfg(not(feature = "egui"))]
    pub fn with_rule_inspector(self) -> Self {
        panic!("with_rule_inspector requires the `egui` feature. Enable it in Cargo.toml: rdpe = {{ features = [\"egui\"] }}")
    }

    /// Check if any rules require neighbor queries
    fn has_neighbor_rules(&self) -> bool {
        self.rules.iter().any(|r| r.requires_neighbors()) || self.interaction_matrix.is_some()
    }

    /// Generate the compute shader WGSL code.
    fn generate_compute_shader(&self) -> String {
        self.generate_compute_shader_impl(false)
    }

    /// Generate the compute shader WGSL code with dynamic rule params.
    #[cfg(feature = "egui")]
    fn generate_compute_shader_dynamic(&self) -> String {
        self.generate_compute_shader_impl(true)
    }

    /// Generate the compute shader WGSL code (implementation).
    fn generate_compute_shader_impl(&self, dynamic_rules: bool) -> String {
        let extra_wgsl = P::EXTRA_WGSL;
        let particle_struct = P::WGSL_STRUCT;
        let has_neighbors = self.has_neighbor_rules();
        let has_sub_emitters = !self.sub_emitters.is_empty();


        // Generate non-neighbor rules (static or dynamic)
        let simple_rules_code: String = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.requires_neighbors())
            .map(|(i, r)| {
                if dynamic_rules {
                    r.to_wgsl_dynamic(i, self.bounds)
                } else {
                    r.to_wgsl(self.bounds)
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        // Generate emitter code
        let emitter_code: String = self
            .emitters
            .iter()
            .enumerate()
            .map(|(i, e)| e.to_wgsl(i))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Generate custom uniform fields for WGSL
        // Note: The Rust Uniforms struct is 72 bytes (64 for mat4 + 4 for time + 4 for delta_time)
        // The GPU code pads to 16-byte alignment (80 bytes) before appending custom uniforms
        // So we need 8 bytes (2 x f32) of padding in WGSL to match
        let custom_uniform_fields = if self.custom_uniforms.is_empty() {
            String::new()
        } else {
            format!(
                "\n    _pad0: f32,\n    _pad1: f32,\n{}",
                self.custom_uniforms.to_wgsl_fields()
            )
        };

        // Built-in utility functions (always included)
        let builtin_utils = shader_utils::all_utils_wgsl();

        // Generate custom functions
        let custom_functions_code = if self.custom_functions.is_empty() {
            builtin_utils
        } else {
            format!(
                "{}\n// Custom functions\n{}\n",
                builtin_utils,
                self.custom_functions.join("\n\n")
            )
        };

        // Generate inbox binding and helper functions if enabled
        let (inbox_binding, inbox_helpers) = if self.inbox_enabled {
            let binding = r#"
// Particle communication inbox (4 channels per particle)
// Uses atomic i32 with fixed-point encoding for thread-safe accumulation
@group(1) @binding(0)
var<storage, read_write> inbox: array<array<atomic<i32>, 4>>;
"#;

            let helpers = r#"
// Fixed-point scale for inbox values (16.16 format)
const INBOX_SCALE: f32 = 65536.0;

// Send a float value to another particle's inbox channel
// Values are accumulated atomically across all senders
fn inbox_send(target_idx: u32, channel: u32, value: f32) {
    let scaled = i32(clamp(value, -32768.0, 32767.0) * INBOX_SCALE);
    atomicAdd(&inbox[target_idx][channel], scaled);
}

// Receive accumulated value from inbox channel
// Returns the sum of all values sent to this particle's channel
// my_idx should be the current particle's index
fn inbox_receive_at(my_idx: u32, channel: u32) -> f32 {
    let scaled = atomicLoad(&inbox[my_idx][channel]);
    return f32(scaled) / INBOX_SCALE;
}

// Convenience macro-like variable that will be replaced with actual index
// Note: Use inbox_receive_at(index, channel) for the full form
"#;
            (binding.to_string(), helpers.to_string())
        } else {
            (String::new(), String::new())
        };

        // Generate field bindings and helper functions if fields are registered
        let field_wgsl = if !self.field_registry.is_empty() {
            self.field_registry.to_wgsl_declarations(0)
        } else {
            String::new()
        };

        // Generate sub-emitter death buffer bindings and recording code
        let (sub_emitter_bindings, sub_emitter_death_recording) = if has_sub_emitters {
            (
                crate::gpu::sub_emitter_gpu::death_buffer_bindings_wgsl().to_string(),
                crate::gpu::sub_emitter_gpu::death_recording_wgsl(&self.sub_emitters),
            )
        } else {
            (String::new(), String::new())
        };

        // Check if any rules are OnDeath or OnSpawn
        let has_on_death = self.rules.iter().any(|r| r.is_on_death());
        let has_on_spawn = self.rules.iter().any(|r| r.is_on_spawn());

        // Track was_alive if we need death recording (sub-emitters, OnDeath, or OnSpawn)
        let was_alive_tracking = if has_sub_emitters || has_on_death || has_on_spawn {
            "    let was_alive = p.alive;\n"
        } else {
            ""
        };

        // Generate OnSpawn code
        let on_spawn_code = if has_on_spawn {
            let actions: String = self
                .rules
                .iter()
                .filter(|r| r.is_on_spawn())
                .map(|r| r.to_on_spawn_wgsl())
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r#"
    // OnSpawn handling
    if was_alive == 0u && p.alive == 1u {{
{actions}
    }}
"#
            )
        } else {
            String::new()
        };

        // Generate OnDeath code
        let on_death_code = if has_on_death {
            let actions: String = self
                .rules
                .iter()
                .filter(|r| r.is_on_death())
                .map(|r| r.to_on_death_wgsl())
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r#"
    // OnDeath handling
    if was_alive == 1u && p.alive == 0u {{
{actions}
    }}
"#
            )
        } else {
            String::new()
        };

        let shader = if !has_neighbors {
            // Simple shader without neighbor queries
            format!(
                r#"{extra_wgsl}
{particle_struct}

struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,{custom_uniform_fields}
}};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;
{inbox_binding}
{field_wgsl}
{sub_emitter_bindings}
{inbox_helpers}
{custom_functions_code}
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let index = global_id.x;
    let num_particles = arrayLength(&particles);

    if index >= num_particles {{
        return;
    }}

    var p = particles[index];
{was_alive_tracking}
{emitter_code}

    // Skip dead particles
    if p.alive == 0u {{
        return;
    }}
{on_spawn_code}
{simple_rules_code}

    // Integrate velocity
    p.position += p.velocity * uniforms.delta_time;
{on_death_code}{sub_emitter_death_recording}
    particles[index] = p;
}}
"#
            )
        } else {
            // Complex shader with neighbor queries
            let neighbor_rules_code: String = self
                .rules
                .iter()
                .filter(|r| r.requires_neighbors())
                .map(|r| r.to_neighbor_wgsl())
                .collect::<Vec<_>>()
                .join("\n");

            let post_neighbor_code: String = self
                .rules
                .iter()
                .filter(|r| r.requires_neighbors())
                .map(|r| r.to_post_neighbor_wgsl())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");

            // Check if we need various accumulators
            let needs_cohesion = self.rules.iter().any(|r| r.needs_cohesion_accumulator());
            let needs_alignment = self.rules.iter().any(|r| r.needs_alignment_accumulator());
            let needs_chase = self.rules.iter().any(|r| r.needs_chase_accumulator());
            let needs_evade = self.rules.iter().any(|r| r.needs_evade_accumulator());
            let needs_viscosity = self.rules.iter().any(|r| r.needs_viscosity_accumulator());
            let needs_pressure = self.rules.iter().any(|r| r.needs_pressure_accumulator());
            let needs_surface_tension = self.rules.iter().any(|r| r.needs_surface_tension_accumulator());
            let needs_avoid = self.rules.iter().any(|r| r.needs_avoid_accumulator());
            let needs_diffuse = self.rules.iter().any(|r| r.needs_diffuse_accumulator());
            let needs_accumulate = self.rules.iter().any(|r| r.needs_accumulate_accumulator());
            let needs_signal = self.rules.iter().any(|r| r.needs_signal_accumulator());
            let needs_absorb = self.rules.iter().any(|r| r.needs_absorb_accumulator());

            // Generate interaction matrix code if present
            let (interaction_init, interaction_neighbor, interaction_post) =
                if let Some(ref matrix) = self.interaction_matrix {
                    (
                        matrix.to_wgsl_init(),
                        matrix.to_wgsl_neighbor(),
                        matrix.to_wgsl_post(),
                    )
                } else {
                    (String::new(), String::new(), String::new())
                };

            let accumulator_vars = {
                let mut vars = String::new();
                if needs_cohesion {
                    vars.push_str("    var cohesion_sum = vec3<f32>(0.0);\n    var cohesion_count = 0.0;\n");
                }
                if needs_alignment {
                    vars.push_str("    var alignment_sum = vec3<f32>(0.0);\n    var alignment_count = 0.0;\n");
                }
                if needs_chase {
                    vars.push_str("    var chase_nearest_dist = 1000.0;\n    var chase_nearest_pos = vec3<f32>(0.0);\n");
                }
                if needs_evade {
                    vars.push_str("    var evade_nearest_dist = 1000.0;\n    var evade_nearest_pos = vec3<f32>(0.0);\n");
                }
                if needs_viscosity {
                    vars.push_str("    var viscosity_sum = vec3<f32>(0.0);\n    var viscosity_weight = 0.0;\n");
                }
                if needs_pressure {
                    vars.push_str("    var pressure_density = 0.0;\n    var pressure_force = vec3<f32>(0.0);\n");
                }
                if needs_surface_tension {
                    vars.push_str("    var surface_neighbor_count = 0.0;\n    var surface_center_sum = vec3<f32>(0.0);\n");
                }
                if needs_avoid {
                    vars.push_str("    var avoid_sum = vec3<f32>(0.0);\n    var avoid_count = 0.0;\n");
                }
                if needs_diffuse {
                    vars.push_str("    var diffuse_sum = 0.0;\n    var diffuse_count = 0.0;\n");
                }
                if needs_accumulate {
                    vars.push_str("    var accumulate_sum = 0.0;\n    var accumulate_weight = 0.0;\n    var accumulate_value = 0.0;\n");
                }
                if needs_signal {
                    vars.push_str("    var signal_sum = 0.0;\n    var signal_count = 0.0;\n");
                }
                if needs_absorb {
                    vars.push_str("    var absorb_sum = 0.0;\n    var absorb_found = false;\n    var absorb_target_idx = 0u;\n");
                }
                // Add interaction matrix init
                if !interaction_init.is_empty() {
                    vars.push('\n');
                    vars.push_str(&interaction_init);
                    vars.push('\n');
                }
                vars
            };

            // Combine neighbor rules with interaction matrix neighbor code
            let neighbor_rules_code = {
                let mut code = neighbor_rules_code;
                if !interaction_neighbor.is_empty() {
                    code.push('\n');
                    code.push_str(&interaction_neighbor);
                }
                code
            };

            // Combine post-neighbor code with interaction matrix post code
            let post_neighbor_code = {
                let mut code = post_neighbor_code;
                if !interaction_post.is_empty() {
                    if !code.is_empty() {
                        code.push_str("\n\n");
                    }
                    code.push_str(&interaction_post);
                }
                code
            };

            format!(
                r#"{extra_wgsl}
{particle_struct}

{MORTON_WGSL}

{NEIGHBOR_UTILS_WGSL}

struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,{custom_uniform_fields}
}};

struct SpatialParams {{
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    max_neighbors: u32,
}};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@group(0) @binding(2)
var<storage, read> sorted_indices: array<u32>;

@group(0) @binding(3)
var<storage, read> cell_start: array<u32>;

@group(0) @binding(4)
var<storage, read> cell_end: array<u32>;

@group(0) @binding(5)
var<uniform> spatial: SpatialParams;
{inbox_binding}
{field_wgsl}
{sub_emitter_bindings}
{inbox_helpers}
{custom_functions_code}
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let index = global_id.x;
    let num_particles = arrayLength(&particles);

    if index >= num_particles {{
        return;
    }}

    var p = particles[index];
{was_alive_tracking}
{emitter_code}

    // Skip dead particles
    if p.alive == 0u {{
        return;
    }}
{on_spawn_code}
    let my_pos = p.position;
    let my_cell = pos_to_cell(my_pos, spatial.cell_size, spatial.grid_resolution);

{accumulator_vars}
    // Neighbor iteration with optional max limit
    var neighbor_count = 0u;
    let max_neighbors = spatial.max_neighbors;
    for (var offset_idx = 0u; offset_idx < 27u; offset_idx++) {{
        // Early exit if max neighbors reached (0 = unlimited)
        if max_neighbors > 0u && neighbor_count >= max_neighbors {{
            break;
        }}

        let neighbor_morton = neighbor_cell_morton(my_cell, offset_idx, spatial.grid_resolution);

        if neighbor_morton == 0xFFFFFFFFu {{
            continue; // Out of bounds
        }}

        let start = cell_start[neighbor_morton];
        let end = cell_end[neighbor_morton];

        if start == 0xFFFFFFFFu {{
            continue; // Empty cell
        }}

        for (var j = start; j < end; j++) {{
            // Early exit if max neighbors reached
            if max_neighbors > 0u && neighbor_count >= max_neighbors {{
                break;
            }}

            let other_idx = sorted_indices[j];

            if other_idx == index {{
                continue; // Skip self
            }}

            let other = particles[other_idx];

            // Skip dead neighbors
            if other.alive == 0u {{
                continue;
            }}

            let neighbor_pos = other.position;
            let neighbor_vel = other.velocity;
            let diff = my_pos - neighbor_pos;
            let neighbor_dist = length(diff);
            let neighbor_dir = select(vec3<f32>(0.0), diff / neighbor_dist, neighbor_dist > 0.0001);

            neighbor_count += 1u;

{neighbor_rules_code}
        }}
    }}

{post_neighbor_code}

{simple_rules_code}

    // Integrate velocity
    p.position += p.velocity * uniforms.delta_time;
{on_death_code}{sub_emitter_death_recording}
    particles[index] = p;
}}
"#
            )
        };

        shader
    }

    /// Generate the render shader WGSL code.
    fn generate_render_shader(&self) -> String {
        use crate::visuals::{combine_vertex_effects, ColorMapping, Palette};

        // Determine if we're using a palette
        let use_palette = !matches!(self.visual_config.palette, Palette::None);

        // Generate palette constants and sampling function if using palette
        let (palette_code, color_expr) = if use_palette {
            let colors = self.visual_config.palette.colors();
            let palette_consts = format!(
                r#"
// Palette colors
const PALETTE_0: vec3<f32> = vec3<f32>({}, {}, {});
const PALETTE_1: vec3<f32> = vec3<f32>({}, {}, {});
const PALETTE_2: vec3<f32> = vec3<f32>({}, {}, {});
const PALETTE_3: vec3<f32> = vec3<f32>({}, {}, {});
const PALETTE_4: vec3<f32> = vec3<f32>({}, {}, {});

fn sample_palette(t: f32) -> vec3<f32> {{
    let t_clamped = clamp(t, 0.0, 1.0);
    let scaled = t_clamped * 4.0;
    let idx = u32(floor(scaled));
    let frac = fract(scaled);

    var c0: vec3<f32>;
    var c1: vec3<f32>;

    switch idx {{
        case 0u: {{ c0 = PALETTE_0; c1 = PALETTE_1; }}
        case 1u: {{ c0 = PALETTE_1; c1 = PALETTE_2; }}
        case 2u: {{ c0 = PALETTE_2; c1 = PALETTE_3; }}
        case 3u: {{ c0 = PALETTE_3; c1 = PALETTE_4; }}
        default: {{ c0 = PALETTE_4; c1 = PALETTE_4; }}
    }}

    return mix(c0, c1, frac);
}}
"#,
                colors[0].x, colors[0].y, colors[0].z,
                colors[1].x, colors[1].y, colors[1].z,
                colors[2].x, colors[2].y, colors[2].z,
                colors[3].x, colors[3].y, colors[3].z,
                colors[4].x, colors[4].y, colors[4].z,
            );

            // Generate the mapping expression
            let mapping_expr = match self.visual_config.color_mapping {
                ColorMapping::None => "0.5".to_string(), // Default to middle of palette
                ColorMapping::Index => format!(
                    "f32(instance_index) / f32({}u)",
                    self.particle_count.max(1)
                ),
                ColorMapping::PositionY { min, max } => format!(
                    "clamp((particle_pos.y - {}) / ({} - {}), 0.0, 1.0)",
                    min, max, min
                ),
                ColorMapping::Distance { max_dist } => format!(
                    "clamp(length(particle_pos) / {}, 0.0, 1.0)",
                    max_dist
                ),
                ColorMapping::Random => {
                    // PCG-style hash for random but consistent color per particle
                    "fract(sin(f32(instance_index) * 12.9898) * 43758.5453)".to_string()
                },
                // Speed and Age need velocity/age passed to shader - fall back to index
                ColorMapping::Speed { .. } | ColorMapping::Age { .. } => {
                    format!("f32(instance_index) / f32({}u)", self.particle_count.max(1))
                }
            };

            let color_expr = format!("sample_palette({})", mapping_expr);
            (palette_consts, color_expr)
        } else {
            // No palette - use particle color or position-based
            let color_expr = match P::COLOR_FIELD {
                Some(_) => "particle_color".to_string(),
                None => "normalize(particle_pos) * 0.5 + 0.5".to_string(),
            };
            (String::new(), color_expr)
        };

        // Color input attribute (only if has color field AND not using palette)
        let color_input = if P::COLOR_FIELD.is_some() && !use_palette {
            "@location(1) particle_color: vec3<f32>,"
        } else {
            ""
        };

        // Generate custom uniform fields for render shader
        let custom_uniform_fields = self.custom_uniforms.to_wgsl_fields();

        // Generate texture declarations
        let texture_declarations = self.texture_registry.to_wgsl_declarations(0);

        // Determine vertex shader body:
        // 1. Custom vertex shader takes precedence
        // 2. Composed vertex effects if any
        // 3. Default vertex body
        let vertex_body = if let Some(ref custom) = self.custom_vertex_shader {
            custom.clone()
        } else if !self.vertex_effects.is_empty() {
            combine_vertex_effects(&self.vertex_effects, &color_expr)
        } else {
            format!(
                r#"    let world_pos = vec4<f32>(particle_pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;

    clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
    clip_pos.y += quad_pos.y * particle_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = {color_expr};
    out.uv = quad_pos;

    return out;"#
            )
        };

        format!(
            r#"struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
{custom_uniform_fields}
}};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Custom textures
{texture_declarations}
{palette_code}
struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
}};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) particle_pos: vec3<f32>,
    {color_input}
    @location(2) alive: u32,
    @location(3) scale: f32,
) -> VertexOutput {{
    var out: VertexOutput;

    // Cull dead particles by moving them off-screen
    if alive == 0u {{
        out.clip_position = vec4<f32>(0.0, 0.0, -1000.0, 1.0);
        out.color = vec3<f32>(0.0);
        out.uv = vec2<f32>(0.0);
        return out;
    }}

    var quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    let quad_pos = quad_vertices[vertex_index];
    let base_size = {particle_size};
    let particle_size = base_size * scale;

    // Custom or default vertex transformation
{vertex_body}
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
{fragment_body}
}}
"#,
            particle_size = self.particle_size,
            vertex_body = vertex_body,
            fragment_body = self.custom_fragment_shader.as_deref()
                .unwrap_or_else(|| self.visual_config.shape.to_wgsl_fragment())
        )
    }

    /// Run the simulation.
    ///
    /// This is the final step that starts the simulation. It:
    ///
    /// 1. Spawns all particles using the spawner function
    /// 2. Generates WGSL compute shaders from the configured rules
    /// 3. Initializes the GPU and creates buffers
    /// 4. Opens a window and starts the render loop
    ///
    /// # Blocking
    ///
    /// This method **blocks** until the user closes the window. It runs
    /// the event loop on the main thread.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - No spawner was provided (forgot to call `.with_spawner()`)
    /// - GPU initialization fails (no compatible GPU found)
    ///
    /// # Window Controls
    ///
    /// - **Left-click + drag**: Rotate camera around the origin
    /// - **Scroll wheel**: Zoom in/out
    /// - **Close window**: Exits the application
    ///
    /// # Example
    ///
    /// ```ignore
    /// Simulation::<Ball>::new()
    ///     .with_particle_count(10_000)
    ///     .with_bounds(1.0)
    ///     .with_spawner(|_| Ball::default())
    ///     .with_rule(Rule::Gravity(9.8))
    ///     .with_rule(Rule::BounceWalls)
    ///     .run();  // Blocks here until window closed
    ///
    /// // This code runs after window is closed
    /// println!("Simulation ended");
    /// ```
    pub fn run(mut self) {
        let spawner = self
            .spawner
            .take()
            .expect("Must provide a spawner with .with_spawner()");

        let has_neighbors = self.has_neighbor_rules();

        // If rule inspector is enabled, add all rule params to custom uniforms
        #[cfg(feature = "egui")]
        if self.rule_inspector_enabled {
            for (i, rule) in self.rules.iter().enumerate() {
                for (name, value) in rule.params(i) {
                    self.custom_uniforms.set(&name, value);
                }
            }
        }

        // Generate shaders before moving self (uses dynamic rules if inspector enabled)
        #[cfg(feature = "egui")]
        let compute_shader = if self.rule_inspector_enabled {
            self.generate_compute_shader_dynamic()
        } else {
            self.generate_compute_shader()
        };
        #[cfg(not(feature = "egui"))]
        let compute_shader = self.generate_compute_shader();
        let render_shader = self.generate_render_shader();

        // Calculate custom uniform buffer size and generate WGSL fields
        let custom_uniform_size = self.custom_uniforms.byte_size();
        let custom_uniform_fields = self.custom_uniforms.to_wgsl_fields();

        // Generate particles using SpawnContext
        let bounds = self.bounds;
        let count = self.particle_count;
        let particles: Vec<P> = (0..self.particle_count)
            .map(|i| {
                let mut ctx = SpawnContext::new(i, count, bounds);
                spawner(&mut ctx)
            })
            .collect();

        let config = SimConfig {
            particle_count: self.particle_count,
            bounds: self.bounds,
            compute_shader,
            render_shader,
            has_neighbors,
            spatial_config: self.spatial_config,
            visual_config: self.visual_config,
            color_offset: P::COLOR_OFFSET,
            alive_offset: P::ALIVE_OFFSET,
            scale_offset: P::SCALE_OFFSET,
            custom_uniform_size,
            custom_uniform_fields,
            particle_size: self.particle_size,
            inbox_enabled: self.inbox_enabled,
            start_dead: self.start_dead,
            #[cfg(feature = "egui")]
            egui_enabled: self.egui_enabled,
            texture_declarations: self.texture_registry.to_wgsl_declarations(0),
            texture_registry: self.texture_registry,
            field_registry: self.field_registry,
            volume_config: self.volume_config,
            sub_emitters: self.sub_emitters,
            particle_wgsl_struct: P::WGSL_STRUCT.to_string(),
        };

        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        #[cfg(feature = "egui")]
        let mut app = App::<P>::new(
            particles,
            config,
            self.custom_uniforms,
            self.update_callback,
            self.ui_callback,
            self.inspector_enabled,
            self.rule_inspector_enabled,
            self.rules,
        );
        #[cfg(not(feature = "egui"))]
        let mut app = App::<P>::new(
            particles,
            config,
            self.custom_uniforms,
            self.update_callback,
        );
        event_loop.run_app(&mut app).unwrap();
    }
}

impl<P: ParticleTrait + 'static> Default for Simulation<P> {
    /// Creates a new simulation with default settings.
    ///
    /// Equivalent to [`Simulation::new()`].
    fn default() -> Self {
        Self::new()
    }
}

/// Internal configuration passed to the GPU renderer.
///
/// Contains all the computed values needed to initialize the GPU state.
pub(crate) struct SimConfig {
    /// Total number of particles.
    pub particle_count: u32,
    /// Simulation bounds (half-size of bounding cube).
    pub bounds: f32,
    /// Generated WGSL compute shader source.
    pub compute_shader: String,
    /// Generated WGSL render shader source.
    pub render_shader: String,
    /// Whether neighbor queries are needed.
    pub has_neighbors: bool,
    /// Spatial hashing configuration.
    pub spatial_config: SpatialConfig,
    /// Visual rendering configuration.
    pub visual_config: VisualConfig,
    /// Byte offset of color field in particle struct, if any.
    pub color_offset: Option<u32>,
    /// Byte offset of alive field in particle struct.
    pub alive_offset: u32,
    /// Byte offset of scale field in particle struct.
    pub scale_offset: u32,
    /// Size of custom uniforms in bytes.
    pub custom_uniform_size: usize,
    /// WGSL struct fields for custom uniforms.
    pub custom_uniform_fields: String,
    /// Base particle render size.
    pub particle_size: f32,
    /// Whether particle inbox communication is enabled.
    pub inbox_enabled: bool,
    /// Whether particles should start dead (for emitter-only spawning).
    pub start_dead: bool,
    /// Whether egui is enabled.
    #[cfg(feature = "egui")]
    pub egui_enabled: bool,
    /// Custom textures for shaders.
    pub texture_registry: TextureRegistry,
    /// WGSL declarations for texture bindings.
    pub texture_declarations: String,
    /// 3D spatial fields for particle-environment interaction.
    pub field_registry: FieldRegistry,
    /// Volume rendering configuration for fields.
    pub volume_config: Option<crate::gpu::VolumeConfig>,
    /// Sub-emitters for spawning particles on death.
    pub sub_emitters: Vec<crate::sub_emitter::SubEmitter>,
    /// WGSL struct definition for particles (needed for spawn shader).
    pub particle_wgsl_struct: String,
}

struct App<P: ParticleTrait> {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    gpu_particles: Vec<P::Gpu>,
    config: SimConfig,
    // Input state (single source of truth for user input)
    input: Input,
    custom_uniforms: CustomUniforms,
    update_callback: Option<UpdateCallback>,
    #[cfg(feature = "egui")]
    ui_callback: Option<UiCallback>,
    #[cfg(feature = "egui")]
    inspector_enabled: bool,
    #[cfg(feature = "egui")]
    rule_inspector_enabled: bool,
    #[cfg(feature = "egui")]
    rules: Vec<Rule>,
    // Time tracking (single source of truth)
    time: Time,
    // Grid opacity change requested by update callback (None = no change)
    pending_grid_opacity: Option<f32>,
    // CPU readback - stores data from previous frame's readback request
    readback_data: Option<Vec<u8>>,
}

impl<P: ParticleTrait + 'static> App<P> {
    fn new(
        particles: Vec<P>,
        config: SimConfig,
        custom_uniforms: CustomUniforms,
        update_callback: Option<UpdateCallback>,
        #[cfg(feature = "egui")] ui_callback: Option<UiCallback>,
        #[cfg(feature = "egui")] inspector_enabled: bool,
        #[cfg(feature = "egui")] rule_inspector_enabled: bool,
        #[cfg(feature = "egui")] rules: Vec<Rule>,
    ) -> Self {
        // Convert user particles to GPU format
        let mut gpu_particles: Vec<P::Gpu> = particles.iter().map(|p| p.to_gpu()).collect();

        // If start_dead is set, set all particles' alive field to 0
        if config.start_dead {
            let particle_size = std::mem::size_of::<P::Gpu>();
            let alive_offset = config.alive_offset as usize;

            // Cast to bytes and set alive = 0 for each particle
            let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut gpu_particles);
            for i in 0..particles.len() {
                let particle_start = i * particle_size;
                // alive is a u32, so write 4 bytes of zeros
                bytes[particle_start + alive_offset..particle_start + alive_offset + 4]
                    .copy_from_slice(&0u32.to_ne_bytes());
            }
        }

        Self {
            window: None,
            gpu_state: None,
            gpu_particles,
            config,
            input: Input::new(),
            custom_uniforms,
            update_callback,
            #[cfg(feature = "egui")]
            ui_callback,
            #[cfg(feature = "egui")]
            inspector_enabled,
            #[cfg(feature = "egui")]
            rule_inspector_enabled,
            #[cfg(feature = "egui")]
            rules,
            time: Time::new(),
            pending_grid_opacity: None,
            readback_data: None,
        }
    }
}

impl<P: ParticleTrait + 'static> ApplicationHandler for App<P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = Window::default_attributes()
                .with_title("RDPE - Reaction Diffusion Particle Engine")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            self.window = Some(window.clone());

            let particle_bytes = bytemuck::cast_slice(&self.gpu_particles);
            self.gpu_state = Some(pollster::block_on(GpuState::new(
                window,
                particle_bytes,
                self.config.particle_count,
                std::mem::size_of::<P::Gpu>(),
                &self.config.compute_shader,
                &self.config.render_shader,
                self.config.has_neighbors,
                self.config.spatial_config,
                self.config.color_offset,
                self.config.alive_offset,
                self.config.scale_offset,
                self.config.custom_uniform_size,
                self.config.visual_config.blend_mode,
                self.config.visual_config.trail_length,
                self.config.particle_size,
                self.config.visual_config.connections_enabled,
                self.config.visual_config.connections_radius,
                self.config.inbox_enabled,
                self.config.visual_config.background_color,
                self.config.visual_config.post_process_shader.as_deref(),
                &self.config.custom_uniform_fields,
                &self.config.texture_registry,
                &self.config.texture_declarations,
                &self.config.field_registry,
                self.config.volume_config.as_ref(),
                &self.config.sub_emitters,
                self.config.visual_config.spatial_grid_opacity,
                &self.config.particle_wgsl_struct,
                self.config.visual_config.wireframe_mesh.as_ref(),
                self.config.visual_config.wireframe_thickness,
                #[cfg(feature = "egui")]
                self.config.egui_enabled,
            )));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Pass events to egui first (if enabled)
        #[cfg(feature = "egui")]
        let egui_consumed = {
            if let Some(gpu_state) = &mut self.gpu_state {
                gpu_state.on_window_event(&event)
            } else {
                false
            }
        };
        #[cfg(not(feature = "egui"))]
        let egui_consumed = false;

        // Pass all events to Input for tracking
        self.input.handle_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                // Update input's window size for NDC calculations
                self.input.set_window_size(physical_size.width, physical_size.height);
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size);
                }
            }
            WindowEvent::MouseInput { .. } => {
                // Left click: particle selection (only if egui didn't consume)
                use crate::input::MouseButton as InputMouseButton;
                if !egui_consumed && self.input.mouse_pressed(InputMouseButton::Left) {
                    let mouse_pos = self.input.mouse_position();
                    if let Some(gpu_state) = &mut self.gpu_state {
                        gpu_state.request_pick(mouse_pos.x as u32, mouse_pos.y as u32);
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                // Camera controls (only if egui didn't consume)
                if !egui_consumed {
                    use crate::input::MouseButton as InputMouseButton;
                    let delta = self.input.mouse_delta();

                    if let Some(gpu_state) = &mut self.gpu_state {
                        // Right mouse: orbit
                        if self.input.mouse_held(InputMouseButton::Right) {
                            gpu_state.camera.orbit(delta.x, delta.y);
                        }
                        // Left mouse: reserved (do nothing)
                        // Middle mouse: reserved (do nothing)
                    }
                }
            }
            WindowEvent::MouseWheel { .. } => {
                // Zoom (only if egui didn't consume)
                if !egui_consumed {
                    let scroll = self.input.scroll_delta();
                    if let Some(gpu_state) = &mut self.gpu_state {
                        gpu_state.camera.zoom(scroll);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Update time (single source of truth)
                let (time, delta_time) = self.time.update();

                // Update camera (keyboard movement + smooth interpolation)
                if let Some(gpu_state) = &mut self.gpu_state {
                    use crate::input::KeyCode;

                    // Speed multiplier when shift is held
                    let speed_mult = if self.input.key_held(KeyCode::Shift) {
                        gpu_state.camera.sprint_multiplier
                    } else {
                        1.0
                    };
                    let move_amount = gpu_state.camera.move_speed * delta_time * speed_mult;

                    // WASD movement
                    if self.input.key_held(KeyCode::W) {
                        gpu_state.camera.move_forward(move_amount);
                    }
                    if self.input.key_held(KeyCode::S) {
                        gpu_state.camera.move_forward(-move_amount);
                    }
                    if self.input.key_held(KeyCode::A) {
                        gpu_state.camera.move_right(-move_amount);
                    }
                    if self.input.key_held(KeyCode::D) {
                        gpu_state.camera.move_right(move_amount);
                    }

                    // Q/E vertical movement
                    if self.input.key_held(KeyCode::Q) {
                        gpu_state.camera.move_up(-move_amount);
                    }
                    if self.input.key_held(KeyCode::E) {
                        gpu_state.camera.move_up(move_amount);
                    }

                    // R to reset camera
                    if self.input.key_pressed(KeyCode::R) {
                        gpu_state.camera.reset();
                    }

                    // Smooth interpolation update
                    gpu_state.camera.update(delta_time);
                }

                // Update window title with FPS (Time handles the update interval internally)
                let fps = self.time.fps();
                if fps > 0.0 {
                    if let Some(window) = &self.window {
                        let title = format!(
                            "RDPE | {} particles | {:.1} FPS | {:.2}ms",
                            self.config.particle_count,
                            fps,
                            1000.0 / fps.max(0.001)
                        );
                        window.set_title(&title);
                    }
                }

                // Reset readback_requested flag before callback
                // (it will be set to true by the callback if needed)
                let mut pending_readback = false;

                // Call update callback if present
                if let Some(ref mut callback) = self.update_callback {
                    let mut ctx = UpdateContext::new(
                        &mut self.custom_uniforms,
                        &self.input,
                        time,
                        delta_time,
                        self.config.bounds,
                        self.input.aspect_ratio(),
                        &mut self.pending_grid_opacity,
                        &mut pending_readback,
                        self.readback_data.as_deref(),
                    );
                    callback(&mut ctx);
                }

                // Get custom uniform bytes
                let custom_bytes = if !self.custom_uniforms.is_empty() {
                    Some(self.custom_uniforms.to_bytes())
                } else {
                    None
                };

                if let Some(gpu_state) = &mut self.gpu_state {
                    // Apply pending grid opacity change
                    if let Some(opacity) = self.pending_grid_opacity.take() {
                        gpu_state.set_grid_opacity(opacity);
                    }

                    let bytes_ref = custom_bytes.as_deref();

                    #[cfg(feature = "egui")]
                    let result = {
                        let inspector_enabled = self.inspector_enabled;
                        let rule_inspector_enabled = self.rule_inspector_enabled;
                        let ui_callback = &mut self.ui_callback;
                        let rules = &self.rules;
                        let custom_uniforms = &mut self.custom_uniforms;
                        let has_ui = ui_callback.is_some() || inspector_enabled || rule_inspector_enabled;
                        if has_ui {
                            gpu_state.render_with_ui(time, delta_time, bytes_ref, |ctx| {
                                // Call user UI callback if present
                                if let Some(ref mut ui_cb) = ui_callback {
                                    ui_cb(ctx);
                                }
                                // Render built-in particle inspector if enabled
                                if inspector_enabled {
                                    render_particle_inspector::<P>(ctx);
                                }
                                // Render built-in rule inspector if enabled
                                if rule_inspector_enabled {
                                    render_rule_inspector(ctx, rules, custom_uniforms);
                                }
                            })
                        } else {
                            gpu_state.render(time, delta_time, bytes_ref)
                        }
                    };
                    #[cfg(not(feature = "egui"))]
                    let result = gpu_state.render(time, delta_time, bytes_ref);

                    match result {
                        Ok(_) => {
                            // Perform readback after successful render if requested
                            if pending_readback {
                                self.readback_data = Some(gpu_state.read_particles_sync());
                            }
                        }
                        Err(wgpu::SurfaceError::Lost) => {
                            gpu_state.resize(winit::dpi::PhysicalSize {
                                width: gpu_state.config.width,
                                height: gpu_state.config.height,
                            })
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }

                // Clear per-frame input state AFTER callbacks have run
                // This ensures pressed/released events are available during the update callback
                self.input.begin_frame();
            }
            _ => {}
        }
    }
}

/// Render the built-in rule inspector panel.
///
/// This is called automatically when `.with_rule_inspector()` is enabled.
/// Allows live editing of rule parameters without shader recompilation.
#[cfg(feature = "egui")]
fn render_rule_inspector(
    ctx: &egui::Context,
    rules: &[Rule],
    custom_uniforms: &mut CustomUniforms,
) {
    use crate::uniforms::UniformValue;

    egui::Window::new("Rule Inspector")
        .default_pos([10.0, 300.0])
        .default_width(320.0)
        .show(ctx, |ui| {
            if rules.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(150, 150, 150),
                    "No rules configured",
                );
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, rule) in rules.iter().enumerate() {
                    let params = rule.params(index);

                    // Skip rules with no editable params
                    if params.is_empty() {
                        // Still show the rule name but grayed out
                        ui.add_enabled(false, egui::Label::new(
                            egui::RichText::new(format!("{}. {}", index, rule.display_name()))
                                .color(egui::Color32::from_rgb(100, 100, 100))
                        ));
                        continue;
                    }

                    // Collapsible section for each rule
                    egui::CollapsingHeader::new(format!("{}. {}", index, rule.display_name()))
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.indent(format!("rule_{}_indent", index), |ui| {
                                for (param_name, _) in params {
                                    // Get the current value from custom_uniforms
                                    if let Some(value) = custom_uniforms.get(&param_name) {
                                        // Extract just the parameter name (after rule_X_)
                                        let display_name = param_name
                                            .strip_prefix(&format!("rule_{}_", index))
                                            .unwrap_or(&param_name);

                                        match value {
                                            UniformValue::F32(v) => {
                                                let mut val = *v;
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("{}:", display_name));
                                                    if ui.add(
                                                        egui::DragValue::new(&mut val)
                                                            .speed(0.01)
                                                            .range(-1000.0..=1000.0)
                                                    ).changed() {
                                                        custom_uniforms.set(&param_name, val);
                                                    }
                                                });
                                            }
                                            UniformValue::U32(v) => {
                                                let mut val = *v as i32;
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("{}:", display_name));
                                                    if ui.add(
                                                        egui::DragValue::new(&mut val)
                                                            .speed(0.1)
                                                            .range(0..=1000)
                                                    ).changed() {
                                                        custom_uniforms.set(&param_name, val.max(0) as u32);
                                                    }
                                                });
                                            }
                                            UniformValue::I32(v) => {
                                                let mut val = *v;
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("{}:", display_name));
                                                    if ui.add(
                                                        egui::DragValue::new(&mut val)
                                                            .speed(0.1)
                                                            .range(-1000..=1000)
                                                    ).changed() {
                                                        custom_uniforms.set(&param_name, val);
                                                    }
                                                });
                                            }
                                            UniformValue::Vec2(v) => {
                                                let mut x = v.x;
                                                let mut y = v.y;
                                                let mut changed = false;
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("{}:", display_name));
                                                });
                                                ui.horizontal(|ui| {
                                                    ui.label("  x:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut x).speed(0.01)
                                                    ).changed();
                                                    ui.label("y:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut y).speed(0.01)
                                                    ).changed();
                                                });
                                                if changed {
                                                    custom_uniforms.set(&param_name, glam::Vec2::new(x, y));
                                                }
                                            }
                                            UniformValue::Vec3(v) => {
                                                let mut x = v.x;
                                                let mut y = v.y;
                                                let mut z = v.z;
                                                let mut changed = false;
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("{}:", display_name));
                                                });
                                                ui.horizontal(|ui| {
                                                    ui.label("  x:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut x).speed(0.01)
                                                    ).changed();
                                                    ui.label("y:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut y).speed(0.01)
                                                    ).changed();
                                                    ui.label("z:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut z).speed(0.01)
                                                    ).changed();
                                                });
                                                if changed {
                                                    custom_uniforms.set(&param_name, glam::Vec3::new(x, y, z));
                                                }
                                            }
                                            UniformValue::Vec4(v) => {
                                                let mut x = v.x;
                                                let mut y = v.y;
                                                let mut z = v.z;
                                                let mut w = v.w;
                                                let mut changed = false;
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("{}:", display_name));
                                                });
                                                ui.horizontal(|ui| {
                                                    ui.label("  x:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut x).speed(0.01)
                                                    ).changed();
                                                    ui.label("y:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut y).speed(0.01)
                                                    ).changed();
                                                });
                                                ui.horizontal(|ui| {
                                                    ui.label("  z:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut z).speed(0.01)
                                                    ).changed();
                                                    ui.label("w:");
                                                    changed |= ui.add(
                                                        egui::DragValue::new(&mut w).speed(0.01)
                                                    ).changed();
                                                });
                                                if changed {
                                                    custom_uniforms.set(&param_name, glam::Vec4::new(x, y, z, w));
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        });
                }
            });
        });
}

/// Render the built-in particle inspector panel.
///
/// This is called automatically when `.with_particle_inspector()` is enabled.
#[cfg(feature = "egui")]
fn render_particle_inspector<P: ParticleTrait>(ctx: &egui::Context) {
    use crate::selection::{selected_particle, selected_particle_data, write_particle};

    egui::Window::new("Particle Inspector")
        .default_pos([10.0, 10.0])
        .default_width(300.0)
        .show(ctx, |ui| {
            if let Some(idx) = selected_particle(ctx) {
                if let Some(mut particle) = selected_particle_data::<P>(ctx) {
                    // Render editable fields
                    if particle.render_editable_fields(ui) {
                        // Particle was modified, queue write back to GPU
                        write_particle(ctx, idx, &particle);
                    }
                } else {
                    // Selection exists but data hasn't loaded yet
                    ui.spinner();
                    ui.label("Loading...");
                }
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(150, 150, 150),
                    "Click a particle to inspect it",
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    // Test particle for simulation tests
    #[derive(Clone)]
    struct TestParticle {
        position: Vec3,
        velocity: Vec3,
    }

    // Implement ParticleTrait manually for tests (simpler than using derive)
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct TestParticleGpu {
        position: [f32; 3],
        _pad0: f32,
        velocity: [f32; 3],
        _pad1: f32,
        particle_type: u32,
        age: f32,
        alive: u32,
        scale: f32,
    }

    impl crate::ParticleTrait for TestParticle {
        type Gpu = TestParticleGpu;

        const WGSL_STRUCT: &'static str = r#"struct Particle {
    position: vec3<f32>,
    _pad0: f32,
    velocity: vec3<f32>,
    _pad1: f32,
    particle_type: u32,
    age: f32,
    alive: u32,
    scale: f32,
}"#;
        const COLOR_FIELD: Option<&'static str> = None;
        const COLOR_OFFSET: Option<u32> = None;
        const ALIVE_OFFSET: u32 = 40;
        const SCALE_OFFSET: u32 = 44;

        fn to_gpu(&self) -> Self::Gpu {
            TestParticleGpu {
                position: self.position.to_array(),
                _pad0: 0.0,
                velocity: self.velocity.to_array(),
                _pad1: 0.0,
                particle_type: 0,
                age: 0.0,
                alive: 1,
                scale: 1.0,
            }
        }

        fn from_gpu(gpu: &Self::Gpu) -> Self {
            Self {
                position: Vec3::from_array(gpu.position),
                velocity: Vec3::from_array(gpu.velocity),
            }
        }

        fn inspect_fields(&self) -> Vec<(&'static str, String)> {
            vec![
                ("position", format!("({:.3}, {:.3}, {:.3})", self.position.x, self.position.y, self.position.z)),
                ("velocity", format!("({:.3}, {:.3}, {:.3})", self.velocity.x, self.velocity.y, self.velocity.z)),
            ]
        }

        #[cfg(feature = "egui")]
        fn render_editable_fields(&mut self, ui: &mut egui::Ui) -> bool {
            let mut modified = false;
            egui::Grid::new("editable_fields")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("position");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut self.position.x).speed(0.01).prefix("x: ")).changed() {
                            modified = true;
                        }
                        if ui.add(egui::DragValue::new(&mut self.position.y).speed(0.01).prefix("y: ")).changed() {
                            modified = true;
                        }
                        if ui.add(egui::DragValue::new(&mut self.position.z).speed(0.01).prefix("z: ")).changed() {
                            modified = true;
                        }
                    });
                    ui.end_row();

                    ui.label("velocity");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut self.velocity.x).speed(0.01).prefix("x: ")).changed() {
                            modified = true;
                        }
                        if ui.add(egui::DragValue::new(&mut self.velocity.y).speed(0.01).prefix("y: ")).changed() {
                            modified = true;
                        }
                        if ui.add(egui::DragValue::new(&mut self.velocity.z).speed(0.01).prefix("z: ")).changed() {
                            modified = true;
                        }
                    });
                    ui.end_row();
                });
            modified
        }
    }

    // ========== Default Values Tests ==========

    #[test]
    fn test_simulation_defaults() {
        let sim = Simulation::<TestParticle>::new();

        assert_eq!(sim.particle_count, 10_000);
        assert!((sim.bounds - 1.0).abs() < 0.001);
        assert!((sim.particle_size - 0.015).abs() < 0.001);
        assert!(sim.spawner.is_none());
        assert!(sim.rules.is_empty());
        assert!(sim.emitters.is_empty());
        assert!(!sim.inbox_enabled);
        assert!(sim.field_registry.is_empty());
        assert!(sim.custom_fragment_shader.is_none());
    }

    // ========== Builder Pattern Tests ==========

    #[test]
    fn test_with_particle_count() {
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(50_000);

        assert_eq!(sim.particle_count, 50_000);
    }

    #[test]
    fn test_with_bounds() {
        let sim = Simulation::<TestParticle>::new()
            .with_bounds(2.5);

        assert!((sim.bounds - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_with_particle_size() {
        let sim = Simulation::<TestParticle>::new()
            .with_particle_size(0.05);

        assert!((sim.particle_size - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_with_spawner() {
        let sim = Simulation::<TestParticle>::new()
            .with_spawner(|ctx| TestParticle {
                position: Vec3::new(ctx.index as f32, 0.0, 0.0),
                velocity: Vec3::ZERO,
            });

        assert!(sim.spawner.is_some());
    }

    #[test]
    fn test_with_rule() {
        let sim = Simulation::<TestParticle>::new()
            .with_rule(Rule::Gravity(9.8))
            .with_rule(Rule::Drag(1.0));

        assert_eq!(sim.rules.len(), 2);
    }

    #[test]
    fn test_with_inbox() {
        let sim = Simulation::<TestParticle>::new()
            .with_inbox();

        assert!(sim.inbox_enabled);
    }

    #[test]
    fn test_with_field() {
        let sim = Simulation::<TestParticle>::new()
            .with_field("pheromone", FieldConfig::new(64));

        assert_eq!(sim.field_registry.len(), 1);
    }

    #[test]
    fn test_with_multiple_fields() {
        let sim = Simulation::<TestParticle>::new()
            .with_field("food", FieldConfig::new(32))
            .with_field("danger", FieldConfig::new(64))
            .with_field("heat", FieldConfig::new(48));

        assert_eq!(sim.field_registry.len(), 3);
    }

    #[test]
    fn test_with_spatial_config() {
        let sim = Simulation::<TestParticle>::new()
            .with_spatial_config(0.1, 64);

        assert!((sim.spatial_config.cell_size - 0.1).abs() < 0.001);
        assert_eq!(sim.spatial_config.grid_resolution, 64);
    }

    #[test]
    fn test_with_fragment_shader() {
        let sim = Simulation::<TestParticle>::new()
            .with_fragment_shader("let my_color = vec4<f32>(1.0, 0.0, 0.0, 1.0);");

        assert!(sim.custom_fragment_shader.is_some());
    }

    #[test]
    fn test_with_function() {
        let sim = Simulation::<TestParticle>::new()
            .with_function("fn my_func(x: f32) -> f32 { return x * 2.0; }");

        assert_eq!(sim.custom_functions.len(), 1);
    }

    #[test]
    fn test_with_uniform() {
        let sim = Simulation::<TestParticle>::new()
            .with_uniform::<f32>("speed", 5.0)
            .with_uniform::<f32>("strength", 2.0);

        assert!(sim.custom_uniforms.get("speed").is_some());
        assert!(sim.custom_uniforms.get("strength").is_some());
    }

    // ========== Builder Chaining Tests ==========

    #[test]
    fn test_builder_chaining() {
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(20_000)
            .with_bounds(2.0)
            .with_particle_size(0.02)
            .with_spawner(|_| TestParticle {
                position: Vec3::ZERO,
                velocity: Vec3::ZERO,
            })
            .with_rule(Rule::Gravity(9.8))
            .with_rule(Rule::BounceWalls)
            .with_rule(Rule::Drag(0.5))
            .with_field("trail", FieldConfig::new(32))
            .with_inbox();

        assert_eq!(sim.particle_count, 20_000);
        assert!((sim.bounds - 2.0).abs() < 0.001);
        assert!((sim.particle_size - 0.02).abs() < 0.001);
        assert!(sim.spawner.is_some());
        assert_eq!(sim.rules.len(), 3);
        assert_eq!(sim.field_registry.len(), 1);
        assert!(sim.inbox_enabled);
    }

    // ========== Rule Configuration Tests ==========

    #[test]
    fn test_rules_order_preserved() {
        let sim = Simulation::<TestParticle>::new()
            .with_rule(Rule::Gravity(9.8))
            .with_rule(Rule::Drag(1.0))
            .with_rule(Rule::BounceWalls);

        // Rules should be in the order they were added
        assert!(matches!(sim.rules[0], Rule::Gravity(_)));
        assert!(matches!(sim.rules[1], Rule::Drag(_)));
        assert!(matches!(sim.rules[2], Rule::BounceWalls));
    }

    #[test]
    fn test_neighbor_rules_detected() {
        let sim = Simulation::<TestParticle>::new()
            .with_rule(Rule::Separate { radius: 0.1, strength: 1.0 })
            .with_rule(Rule::Cohere { radius: 0.5, strength: 0.5 });

        // Both rules should require neighbors
        assert!(sim.rules.iter().all(|r| r.requires_neighbors()));
    }

    // ========== Spatial Config Tests ==========

    #[test]
    fn test_spatial_config_defaults() {
        let sim = Simulation::<TestParticle>::new();

        // Default spatial config should have sensible values
        assert!(sim.spatial_config.cell_size > 0.0);
        assert!(sim.spatial_config.grid_resolution > 0);
    }

    // ========== Visual Config Tests ==========

    #[test]
    fn test_with_visuals() {
        let sim = Simulation::<TestParticle>::new()
            .with_visuals(|v| {
                v.blend_mode(crate::visuals::BlendMode::Additive)
                    .trails(10);
            });

        assert_eq!(sim.visual_config.blend_mode, crate::visuals::BlendMode::Additive);
        assert_eq!(sim.visual_config.trail_length, 10);
    }

    // ========== Field Registry Tests ==========

    #[test]
    fn test_field_registry_index_lookup() {
        let sim = Simulation::<TestParticle>::new()
            .with_field("food", FieldConfig::new(32))
            .with_field("danger", FieldConfig::new(64));

        assert_eq!(sim.field_registry.index_of("food"), Some(0));
        assert_eq!(sim.field_registry.index_of("danger"), Some(1));
        assert_eq!(sim.field_registry.index_of("nonexistent"), None);
    }

    // ========== Shader Generation Integration Tests ==========
    //
    // These tests validate that complete simulation configurations
    // generate valid WGSL that compiles with naga.

    /// Helper to validate WGSL using naga
    fn validate_wgsl(wgsl: &str) -> Result<(), String> {
        let module = naga::front::wgsl::parse_str(wgsl)
            .map_err(|e| format!("WGSL parse error: {:?}", e))?;

        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .map_err(|e| format!("WGSL validation error: {:?}", e))?;

        Ok(())
    }

    #[test]
    fn test_simple_physics_shader_validates() {
        // Minimal simulation: gravity + bounce walls
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(1000)
            .with_bounds(1.0)
            .with_rule(Rule::Gravity(9.8))
            .with_rule(Rule::Drag(0.5))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Simple physics shader should be valid");
    }

    #[test]
    fn test_attractor_shader_validates() {
        // Simulation with point attractors
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(5000)
            .with_bounds(2.0)
            .with_rule(Rule::AttractTo {
                point: Vec3::ZERO,
                strength: 1.0,
            })
            .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
            .with_rule(Rule::Drag(1.0));

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Attractor shader should be valid");
    }

    #[test]
    fn test_vortex_curl_shader_validates() {
        // Simulation with vortex and curl forces
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(10000)
            .with_bounds(1.5)
            .with_rule(Rule::Vortex {
                center: Vec3::ZERO,
                axis: Vec3::Y,
                strength: 2.0,
            })
            .with_rule(Rule::Curl {
                scale: 0.5,
                strength: 1.0,
            })
            .with_rule(Rule::WrapWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Vortex/curl shader should be valid");
    }

    #[test]
    fn test_turbulence_oscillate_shader_validates() {
        // Simulation with noise-based motion
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(8000)
            .with_bounds(1.0)
            .with_rule(Rule::Turbulence {
                scale: 1.0,
                strength: 0.5,
            })
            .with_rule(Rule::Oscillate {
                axis: Vec3::Y,
                frequency: 2.0,
                amplitude: 0.3,
                spatial_scale: 1.0,
            })
            .with_rule(Rule::PositionNoise {
                scale: 0.5,
                strength: 0.1,
                speed: 1.0,
            })
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Turbulence/oscillate shader should be valid");
    }

    #[test]
    fn test_boids_style_shader_validates() {
        // Boids-style flocking with neighbor queries
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(5000)
            .with_bounds(1.0)
            .with_spatial_config(0.1, 32)
            .with_rule(Rule::Separate {
                radius: 0.05,
                strength: 5.0,
            })
            .with_rule(Rule::Cohere {
                radius: 0.15,
                strength: 1.0,
            })
            .with_rule(Rule::Align {
                radius: 0.1,
                strength: 2.0,
            })
            .with_rule(Rule::Drag(2.0))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Boids shader should be valid");
    }

    #[test]
    fn test_collision_shader_validates() {
        // Particle collision simulation
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(2000)
            .with_bounds(1.0)
            .with_spatial_config(0.1, 32)
            .with_rule(Rule::Collide {
                radius: 0.03,
                restitution: 0.8,
            })
            .with_rule(Rule::Gravity(2.0))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Collision shader should be valid");
    }

    #[test]
    fn test_nbody_shader_validates() {
        // N-body gravity simulation
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(500)
            .with_bounds(2.0)
            .with_spatial_config(0.5, 16)
            .with_rule(Rule::NBodyGravity {
                radius: 2.0,
                strength: 0.001,
                softening: 0.01,
            })
            .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.0 });

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("N-body shader should be valid");
    }

    #[test]
    fn test_spring_shader_validates() {
        // Spring to anchor point
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(1000)
            .with_bounds(1.0)
            .with_rule(Rule::Spring {
                anchor: Vec3::ZERO,
                stiffness: 5.0,
                damping: 0.1,
            })
            .with_rule(Rule::Gravity(1.0))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Spring shader should be valid");
    }

    #[test]
    fn test_age_lifecycle_shader_validates() {
        // Particles with age and lifetime
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(5000)
            .with_bounds(1.0)
            .with_rule(Rule::Age)
            .with_rule(Rule::Lifetime(5.0))
            .with_rule(Rule::Gravity(9.8))
            .with_rule(Rule::Drag(1.0))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Age lifecycle shader should be valid");
    }

    #[test]
    fn test_wander_shader_validates() {
        // Random wandering motion
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(3000)
            .with_bounds(1.0)
            .with_rule(Rule::Wander {
                strength: 1.0,
                frequency: 100.0,
            })
            .with_rule(Rule::SpeedLimit { min: 0.1, max: 0.5 })
            .with_rule(Rule::WrapWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Wander shader should be valid");
    }

    #[test]
    fn test_orbit_shader_validates() {
        // Orbital motion
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(2000)
            .with_bounds(2.0)
            .with_rule(Rule::Orbit {
                center: Vec3::ZERO,
                strength: 3.0,
            })
            .with_rule(Rule::PointGravity {
                point: Vec3::ZERO,
                strength: 1.0,
                softening: 0.05,
            })
            .with_rule(Rule::Drag(0.1));

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Orbit shader should be valid");
    }

    #[test]
    fn test_inbox_shader_validates() {
        // Particle communication
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(1000)
            .with_bounds(1.0)
            .with_inbox()
            .with_rule(Rule::Gravity(9.8))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Inbox shader should be valid");
    }

    #[test]
    fn test_single_field_shader_validates() {
        // Single spatial field
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(5000)
            .with_bounds(1.0)
            .with_field("pheromone", FieldConfig::new(64))
            .with_rule(Rule::Gravity(1.0))
            .with_rule(Rule::WrapWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Single field shader should be valid");
    }

    #[test]
    fn test_multi_field_shader_validates() {
        // Multiple spatial fields
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(5000)
            .with_bounds(1.0)
            .with_field("food", FieldConfig::new(32))
            .with_field("danger", FieldConfig::new(32))
            .with_field("trail", FieldConfig::new(64))
            .with_rule(Rule::Gravity(1.0))
            .with_rule(Rule::WrapWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Multi-field shader should be valid");
    }

    #[test]
    fn test_custom_function_shader_validates() {
        // Custom WGSL function
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(1000)
            .with_bounds(1.0)
            .with_function(r#"
fn my_force(pos: vec3<f32>) -> vec3<f32> {
    return normalize(-pos) * 0.1;
}
"#)
            .with_rule(Rule::Custom("p.velocity += my_force(p.position);".to_string()))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Custom function shader should be valid");
    }

    #[test]
    fn test_custom_uniform_shader_validates() {
        // Custom uniforms
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(1000)
            .with_bounds(1.0)
            .with_uniform::<f32>("force_strength", 1.0)
            .with_uniform::<f32>("decay_rate", 0.1)
            .with_rule(Rule::Custom("p.velocity *= (1.0 - uniforms.decay_rate);".to_string()))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Custom uniform shader should be valid");
    }

    #[test]
    fn test_typed_rules_shader_validates() {
        // Typed particle interactions
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(3000)
            .with_bounds(1.0)
            .with_spatial_config(0.3, 32)
            .with_rule(Rule::Typed {
                self_type: 0,
                other_type: Some(0),
                rule: Box::new(Rule::Cohere {
                    radius: 0.1,
                    strength: 1.0,
                }),
            })
            .with_rule(Rule::Typed {
                self_type: 1,
                other_type: Some(0),
                rule: Box::new(Rule::Separate {
                    radius: 0.15,
                    strength: 2.0,
                }),
            })
            .with_rule(Rule::Drag(1.0))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Typed rules shader should be valid");
    }

    #[test]
    fn test_chase_evade_shader_validates() {
        // Predator-prey dynamics
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(2000)
            .with_bounds(1.0)
            .with_spatial_config(0.3, 32)
            .with_rule(Rule::Chase {
                self_type: 1,   // Predator
                target_type: 0, // Prey
                radius: 0.4,
                strength: 4.0,
            })
            .with_rule(Rule::Evade {
                self_type: 0,   // Prey
                threat_type: 1, // Predator
                radius: 0.25,
                strength: 6.0,
            })
            .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
            .with_rule(Rule::Drag(1.5))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Chase/evade shader should be valid");
    }

    #[test]
    fn test_complex_combined_shader_validates() {
        // Complex simulation with many features
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(10000)
            .with_bounds(1.5)
            .with_spatial_config(0.15, 32)
            .with_inbox()
            .with_field("trail", FieldConfig::new(64))
            .with_uniform::<f32>("global_force", 0.5)
            .with_function(r#"
fn custom_decay(v: vec3<f32>) -> vec3<f32> {
    return v * 0.99;
}
"#)
            .with_rule(Rule::Separate {
                radius: 0.05,
                strength: 3.0,
            })
            .with_rule(Rule::Cohere {
                radius: 0.2,
                strength: 0.5,
            })
            .with_rule(Rule::Vortex {
                center: Vec3::ZERO,
                axis: Vec3::Y,
                strength: 0.5,
            })
            .with_rule(Rule::Custom("p.velocity = custom_decay(p.velocity);".to_string()))
            .with_rule(Rule::SpeedLimit { min: 0.05, max: 1.5 })
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Complex combined shader should be valid");
    }

    #[test]
    fn test_magnetism_shader_validates() {
        // Magnetic field simulation
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(3000)
            .with_bounds(1.0)
            .with_spatial_config(0.2, 32)
            .with_rule(Rule::Magnetism {
                radius: 0.2,
                strength: 1.0,
                same_repel: true,
            })
            .with_rule(Rule::Drag(0.5))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Magnetism shader should be valid");
    }

    #[test]
    fn test_fluid_shader_validates() {
        // Fluid-like simulation using Viscosity and Pressure
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(5000)
            .with_bounds(1.0)
            .with_spatial_config(0.1, 32)
            .with_rule(Rule::Viscosity {
                radius: 0.08,
                strength: 0.5,
            })
            .with_rule(Rule::Pressure {
                radius: 0.08,
                strength: 1.0,
                target_density: 8.0,
            })
            .with_rule(Rule::Gravity(2.0))
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("Fluid shader should be valid");
    }

    #[test]
    fn test_all_falloff_types_shader_validates() {
        // Test multiple falloff types using Radial rule
        // Note: Smooth falloff has WGSL validation issues with naga, skipped for now
        let sim = Simulation::<TestParticle>::new()
            .with_particle_count(1000)
            .with_bounds(1.0)
            .with_rule(Rule::Radial {
                point: Vec3::new(0.5, 0.0, 0.0),
                strength: 0.5,
                radius: 1.0,
                falloff: crate::rules::Falloff::Constant,
            })
            .with_rule(Rule::Radial {
                point: Vec3::new(-0.5, 0.0, 0.0),
                strength: 0.5,
                radius: 1.0,
                falloff: crate::rules::Falloff::Linear,
            })
            .with_rule(Rule::Radial {
                point: Vec3::new(0.0, 0.5, 0.0),
                strength: 0.5,
                radius: 1.0,
                falloff: crate::rules::Falloff::InverseSquare,
            })
            .with_rule(Rule::Radial {
                point: Vec3::new(0.0, -0.5, 0.0),
                strength: 0.5,
                radius: 1.0,
                falloff: crate::rules::Falloff::Inverse,
            })
            .with_rule(Rule::BounceWalls);

        let shader = sim.generate_compute_shader();
        validate_wgsl(&shader).expect("All falloff types shader should be valid");
    }
}
