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
//!     .with_spawner(|i, count| Ball {
//!         position: Vec3::new(0.0, 0.5, 0.0),
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
use crate::gpu::GpuState;
use crate::interactions::InteractionMatrix;
use crate::rules::Rule;
use crate::shader_utils;
use crate::spatial::{SpatialConfig, MORTON_WGSL, NEIGHBOR_UTILS_WGSL};
use crate::textures::{TextureConfig, TextureRegistry};
use crate::uniforms::{CustomUniforms, UniformValue, UpdateContext};
use crate::visuals::VisualConfig;
use crate::ParticleTrait;
use std::marker::PhantomData;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

/// Type alias for the update callback to reduce complexity.
type UpdateCallback = Box<dyn FnMut(&mut UpdateContext) + Send>;

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
///     .with_spawner(|i, count| Boid {
///         position: Vec3::new(
///             (i as f32 / count as f32) * 2.0 - 1.0,
///             0.0,
///             0.0,
///         ),
///         velocity: Vec3::ZERO,
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
    spawner: Option<Box<dyn Fn(u32, u32) -> P + Send + Sync>>,
    /// List of rules that define particle behavior.
    rules: Vec<Rule>,
    /// Particle emitters for runtime spawning.
    emitters: Vec<Emitter>,
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
    /// Custom fragment shader code (replaces default fragment body).
    custom_fragment_shader: Option<String>,
    /// Whether egui UI is enabled.
    #[cfg(feature = "egui")]
    egui_enabled: bool,
    /// UI callback for egui (called each frame).
    #[cfg(feature = "egui")]
    ui_callback: Option<Box<dyn FnMut(&egui::Context) + Send + 'static>>,
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
            interaction_matrix: None,
            custom_uniforms: CustomUniforms::new(),
            texture_registry: TextureRegistry::new(),
            update_callback: None,
            custom_functions: Vec::new(),
            spatial_config: SpatialConfig::default(),
            visual_config: VisualConfig::default(),
            inbox_enabled: false,
            custom_fragment_shader: None,
            #[cfg(feature = "egui")]
            egui_enabled: false,
            #[cfg(feature = "egui")]
            ui_callback: None,
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
    /// It receives the particle index and total count, and must return a
    /// fully initialized particle.
    ///
    /// # Arguments
    ///
    /// * `spawner` - Function `(index, total_count) -> P`
    ///
    /// # Required
    ///
    /// This method **must** be called before `.run()`, or the simulation
    /// will panic.
    ///
    /// # Examples
    ///
    /// ## Simple centered spawn
    ///
    /// ```ignore
    /// .with_spawner(|i, count| Ball {
    ///     position: Vec3::ZERO,
    ///     velocity: Vec3::ZERO,
    /// })
    /// ```
    ///
    /// ## Random distribution
    ///
    /// ```ignore
    /// let mut rng = rand::thread_rng();
    /// let particles: Vec<Ball> = (0..count)
    ///     .map(|_| Ball {
    ///         position: Vec3::new(
    ///             rng.gen_range(-1.0..1.0),
    ///             rng.gen_range(-1.0..1.0),
    ///             rng.gen_range(-1.0..1.0),
    ///         ),
    ///         velocity: Vec3::ZERO,
    ///     })
    ///     .collect();
    ///
    /// Simulation::<Ball>::new()
    ///     .with_spawner(move |i, _| particles[i as usize].clone())
    /// ```
    ///
    /// ## Type-based initialization
    ///
    /// ```ignore
    /// .with_spawner(|i, count| {
    ///     let is_predator = i < 50;
    ///     Creature {
    ///         position: random_pos(),
    ///         velocity: Vec3::ZERO,
    ///         particle_type: if is_predator { 1 } else { 0 },
    ///     }
    /// })
    /// ```
    pub fn with_spawner<F>(mut self, spawner: F) -> Self
    where
        F: Fn(u32, u32) -> P + Send + Sync + 'static,
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
    pub fn with_rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
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
    ///     .with_spawner(|_, _| Ball::default())
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
    ///     .with_spawner(|_, _| Ball::default())
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

    /// Check if any rules require neighbor queries
    fn has_neighbor_rules(&self) -> bool {
        self.rules.iter().any(|r| r.requires_neighbors()) || self.interaction_matrix.is_some()
    }

    /// Generate the compute shader WGSL code.
    fn generate_compute_shader(&self) -> String {
        let particle_struct = P::WGSL_STRUCT;
        let has_neighbors = self.has_neighbor_rules();

        // Generate non-neighbor rules
        let simple_rules_code: String = self
            .rules
            .iter()
            .filter(|r| !r.requires_neighbors())
            .map(|r| r.to_wgsl(self.bounds))
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
        let custom_uniform_fields = if self.custom_uniforms.is_empty() {
            String::new()
        } else {
            format!("\n{}", self.custom_uniforms.to_wgsl_fields())
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

        if !has_neighbors {
            // Simple shader without neighbor queries
            format!(
                r#"{particle_struct}

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

{emitter_code}

    // Skip dead particles
    if p.alive == 0u {{
        return;
    }}

{simple_rules_code}

    // Integrate velocity
    p.position += p.velocity * uniforms.delta_time;

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
                r#"{particle_struct}

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
    _pad: u32,
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

{emitter_code}

    // Skip dead particles
    if p.alive == 0u {{
        return;
    }}

    let my_pos = p.position;
    let my_cell = pos_to_cell(my_pos, spatial.cell_size, spatial.grid_resolution);

{accumulator_vars}
    // Neighbor iteration
    for (var offset_idx = 0u; offset_idx < 27u; offset_idx++) {{
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

{neighbor_rules_code}
        }}
    }}

{post_neighbor_code}

{simple_rules_code}

    // Integrate velocity
    p.position += p.velocity * uniforms.delta_time;

    particles[index] = p;
}}
"#
            )
        }
    }

    /// Generate the render shader WGSL code.
    fn generate_render_shader(&self) -> String {
        use crate::visuals::{ColorMapping, Palette};

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

    let world_pos = vec4<f32>(particle_pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;

    clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
    clip_pos.y += quad_pos.y * particle_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = {color_expr};
    out.uv = quad_pos;

    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
{fragment_body}
}}
"#,
            particle_size = self.particle_size,
            fragment_body = self.custom_fragment_shader.as_ref().map(|s| s.as_str()).unwrap_or(r#"    let dist = length(in.uv);
    if dist > 1.0 {
        discard;
    }
    let alpha = 1.0 - smoothstep(0.5, 1.0, dist);
    return vec4<f32>(in.color, alpha);"#)
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
    ///     .with_spawner(|_, _| Ball::default())
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

        // Generate shaders before moving self
        let compute_shader = self.generate_compute_shader();
        let render_shader = self.generate_render_shader();

        // Calculate custom uniform buffer size and generate WGSL fields
        let custom_uniform_size = self.custom_uniforms.byte_size();
        let custom_uniform_fields = self.custom_uniforms.to_wgsl_fields();

        // Generate particles
        let particles: Vec<P> = (0..self.particle_count)
            .map(|i| spawner(i, self.particle_count))
            .collect();

        let config = SimConfig {
            particle_count: self.particle_count,
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
            #[cfg(feature = "egui")]
            egui_enabled: self.egui_enabled,
            texture_declarations: self.texture_registry.to_wgsl_declarations(0),
            texture_registry: self.texture_registry,
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
    /// Whether egui is enabled.
    #[cfg(feature = "egui")]
    pub egui_enabled: bool,
    /// Custom textures for shaders.
    pub texture_registry: TextureRegistry,
    /// WGSL declarations for texture bindings.
    pub texture_declarations: String,
}

struct App<P: ParticleTrait> {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    gpu_particles: Vec<P::Gpu>,
    config: SimConfig,
    mouse_pressed: bool,
    last_mouse_pos: Option<(f64, f64)>,
    current_mouse_ndc: Option<glam::Vec2>,
    custom_uniforms: CustomUniforms,
    update_callback: Option<UpdateCallback>,
    #[cfg(feature = "egui")]
    ui_callback: Option<Box<dyn FnMut(&egui::Context) + Send + 'static>>,
    // FPS tracking
    frame_count: u32,
    fps_update_time: std::time::Instant,
    current_fps: f32,
}

impl<P: ParticleTrait + 'static> App<P> {
    fn new(
        particles: Vec<P>,
        config: SimConfig,
        custom_uniforms: CustomUniforms,
        update_callback: Option<UpdateCallback>,
        #[cfg(feature = "egui")] ui_callback: Option<Box<dyn FnMut(&egui::Context) + Send + 'static>>,
    ) -> Self {
        // Convert user particles to GPU format
        let gpu_particles: Vec<P::Gpu> = particles.iter().map(|p| p.to_gpu()).collect();

        Self {
            window: None,
            gpu_state: None,
            gpu_particles,
            config,
            mouse_pressed: false,
            last_mouse_pos: None,
            current_mouse_ndc: None,
            custom_uniforms,
            update_callback,
            #[cfg(feature = "egui")]
            ui_callback,
            frame_count: 0,
            fps_update_time: std::time::Instant::now(),
            current_fps: 0.0,
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

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // Only process if egui didn't consume the event
                if !egui_consumed && button == MouseButton::Left {
                    self.mouse_pressed = state == ElementState::Pressed;
                    if !self.mouse_pressed {
                        self.last_mouse_pos = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Track mouse position in NDC for custom uniforms (always)
                if let Some(gpu_state) = &self.gpu_state {
                    let w = gpu_state.config.width as f32;
                    let h = gpu_state.config.height as f32;
                    let ndc_x = (position.x as f32 / w) * 2.0 - 1.0;
                    let ndc_y = 1.0 - (position.y as f32 / h) * 2.0; // Flip Y
                    self.current_mouse_ndc = Some(glam::Vec2::new(ndc_x, ndc_y));
                }

                // Camera drag (only if egui didn't consume)
                if !egui_consumed && self.mouse_pressed {
                    if let Some((last_x, last_y)) = self.last_mouse_pos {
                        let dx = position.x - last_x;
                        let dy = position.y - last_y;

                        if let Some(gpu_state) = &mut self.gpu_state {
                            gpu_state.camera.yaw -= dx as f32 * 0.005;
                            gpu_state.camera.pitch += dy as f32 * 0.005;
                            gpu_state.camera.pitch = gpu_state.camera.pitch.clamp(-1.5, 1.5);
                        }
                    }
                    self.last_mouse_pos = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Only process if egui didn't consume the event
                if !egui_consumed {
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.1,
                    };
                    if let Some(gpu_state) = &mut self.gpu_state {
                        gpu_state.camera.distance -= scroll * 0.3;
                        gpu_state.camera.distance = gpu_state.camera.distance.clamp(0.5, 20.0);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // FPS tracking
                self.frame_count += 1;
                let elapsed = self.fps_update_time.elapsed().as_secs_f32();
                if elapsed >= 0.5 {
                    self.current_fps = self.frame_count as f32 / elapsed;
                    self.frame_count = 0;
                    self.fps_update_time = std::time::Instant::now();

                    // Update window title with FPS
                    if let Some(window) = &self.window {
                        let title = format!(
                            "RDPE | {} particles | {:.1} FPS | {:.2}ms",
                            self.config.particle_count,
                            self.current_fps,
                            1000.0 / self.current_fps.max(0.001)
                        );
                        window.set_title(&title);
                    }
                }

                // Get time info once
                let (time, delta_time) = if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.get_time_info()
                } else {
                    (0.0, 0.0)
                };

                // Call update callback if present
                if let Some(ref mut callback) = self.update_callback {
                    let mut ctx = UpdateContext {
                        uniforms: &mut self.custom_uniforms,
                        time,
                        delta_time,
                        mouse_ndc: self.current_mouse_ndc,
                        mouse_pressed: self.mouse_pressed,
                    };
                    callback(&mut ctx);
                }

                // Get custom uniform bytes
                let custom_bytes = if !self.custom_uniforms.is_empty() {
                    Some(self.custom_uniforms.to_bytes())
                } else {
                    None
                };

                if let Some(gpu_state) = &mut self.gpu_state {
                    let bytes_ref = custom_bytes.as_deref();

                    #[cfg(feature = "egui")]
                    let result = {
                        if let Some(ref mut ui_cb) = self.ui_callback {
                            gpu_state.render_with_ui(time, delta_time, bytes_ref, ui_cb)
                        } else {
                            gpu_state.render(time, delta_time, bytes_ref)
                        }
                    };
                    #[cfg(not(feature = "egui"))]
                    let result = gpu_state.render(time, delta_time, bytes_ref);

                    match result {
                        Ok(_) => {}
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
            }
            _ => {}
        }
    }
}
