//! Particle behavior rules.
//!
//! Rules define how particles behave each frame. They are applied in order
//! and translated into WGSL compute shader code at simulation startup.
//!
//! # Rule Categories
//!
//! - **Basic Physics**: Gravity, Drag, Acceleration, SpeedLimit, Wander
//! - **Boundaries**: BounceWalls, WrapWalls
//! - **Point Forces**: AttractTo, RepelFrom, PointGravity, Spring
//! - **Field Effects**: Vortex, Turbulence, Orbit, Curl
//! - **Wave/Modulation**: Oscillate, PositionNoise
//! - **Flocking** (neighbor): Collide, Separate, Cohere, Align, Avoid
//! - **Fluid** (neighbor): NBodyGravity, Viscosity, Pressure, SurfaceTension
//! - **Electromagnetic** (neighbor): Magnetism
//! - **Type-Based** (neighbor): Typed, Convert, Chase, Evade
//! - **Lifecycle**: Age, Lifetime, FadeOut, ShrinkOut, ColorOverLife
//! - **Visual**: ColorBySpeed, ColorByAge, ScaleBySpeed
//! - **Custom**: Custom (raw WGSL), NeighborCustom (WGSL in neighbor loop)
//!
//! # Neighbor Rules
//!
//! Rules marked "(neighbor)" query nearby particles and require spatial hashing.
//! Configure with:
//!
//! ```ignore
//! .with_spatial_config(cell_size, grid_resolution)
//! ```
//!
//! The `cell_size` should be at least as large as your largest interaction
//! radius. The `grid_resolution` controls memory usage (typical: 32 or 64).

use glam::Vec3;

/// Distance falloff functions for force-based rules.
///
/// Controls how a force's strength changes with distance from the source.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Falloff {
    /// Constant force regardless of distance.
    #[default]
    Constant,

    /// Linear falloff: force decreases linearly to zero at max range.
    Linear,

    /// Inverse falloff: force = 1/distance (with softening).
    Inverse,

    /// Inverse-square falloff: force = 1/distance² (realistic gravity/EM).
    InverseSquare,

    /// Smooth falloff using smoothstep for gradual transitions.
    Smooth,
}

impl Falloff {
    /// Generate WGSL code for this falloff function.
    /// Returns an expression that computes the falloff factor given `dist` and `radius`.
    pub fn to_wgsl_expr(&self) -> &'static str {
        match self {
            Falloff::Constant => "1.0",
            Falloff::Linear => "(1.0 - dist / radius)",
            Falloff::Inverse => "(1.0 / (dist + 0.01))",
            Falloff::InverseSquare => "(1.0 / (dist * dist + 0.0001))",
            Falloff::Smooth => "(1.0 - smoothstep(0.0, radius, dist))",
        }
    }
}

/// A transition between agent states.
///
/// Transitions are checked in order of priority (highest first).
/// The first transition whose condition evaluates to true is taken.
#[derive(Clone, Debug)]
pub struct Transition {
    /// Target state ID to transition to.
    pub to: u32,
    /// WGSL boolean expression that triggers this transition.
    /// Has access to `p` (particle), `uniforms.time`, `uniforms.delta_time`, etc.
    pub condition: String,
    /// Priority (higher = checked first). Default: 0.
    pub priority: i32,
}

impl Transition {
    /// Create a new transition with default priority.
    pub fn new(to: u32, condition: impl Into<String>) -> Self {
        Self {
            to,
            condition: condition.into(),
            priority: 0,
        }
    }

    /// Create a transition with explicit priority.
    pub fn with_priority(to: u32, condition: impl Into<String>, priority: i32) -> Self {
        Self {
            to,
            condition: condition.into(),
            priority,
        }
    }
}

/// A state in an agent state machine.
///
/// Each state can have:
/// - Entry action: runs once when entering this state
/// - Update action: runs every frame while in this state
/// - Exit action: runs once when leaving this state
/// - Transitions: conditions that trigger moving to other states
#[derive(Clone, Debug)]
pub struct AgentState {
    /// Unique state identifier (matches the particle's state field value).
    pub id: u32,
    /// Optional name for debugging/documentation.
    pub name: Option<String>,
    /// WGSL code to execute when entering this state.
    pub on_enter: Option<String>,
    /// WGSL code to execute every frame while in this state.
    pub on_update: Option<String>,
    /// WGSL code to execute when leaving this state.
    pub on_exit: Option<String>,
    /// Transitions to other states.
    pub transitions: Vec<Transition>,
}

impl AgentState {
    /// Create a new state with the given ID.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: None,
            on_enter: None,
            on_update: None,
            on_exit: None,
            transitions: Vec::new(),
        }
    }

    /// Set a name for this state (for documentation).
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the entry action (runs once when entering).
    pub fn on_enter(mut self, code: impl Into<String>) -> Self {
        self.on_enter = Some(code.into());
        self
    }

    /// Set the update action (runs every frame while in this state).
    pub fn on_update(mut self, code: impl Into<String>) -> Self {
        self.on_update = Some(code.into());
        self
    }

    /// Set the exit action (runs once when leaving).
    pub fn on_exit(mut self, code: impl Into<String>) -> Self {
        self.on_exit = Some(code.into());
        self
    }

    /// Add a transition to another state.
    pub fn transition(mut self, to: u32, condition: impl Into<String>) -> Self {
        self.transitions.push(Transition::new(to, condition));
        self
    }

    /// Add a transition with explicit priority.
    pub fn transition_priority(
        mut self,
        to: u32,
        condition: impl Into<String>,
        priority: i32,
    ) -> Self {
        self.transitions
            .push(Transition::with_priority(to, condition, priority));
        self
    }
}

/// Rules that define particle behavior.
///
/// Rules are applied every frame in the order they are added. Each rule
/// modifies particle velocity (and sometimes position or type). After all
/// rules execute, velocity is integrated: `position += velocity * delta_time`.
///
/// # Example
///
/// ```ignore
/// Simulation::<MyParticle>::new()
///     .with_rule(Rule::Gravity(9.8))
///     .with_rule(Rule::Separate { radius: 0.1, strength: 2.0 })
///     .with_rule(Rule::SpeedLimit { min: 0.0, max: 5.0 })
///     .with_rule(Rule::Drag(1.0))
///     .with_rule(Rule::BounceWalls)
///     .run();
/// ```
#[derive(Clone, Debug)]
pub enum Rule {
    /// Constant downward acceleration (negative Y).
    ///
    /// # Parameters
    ///
    /// - `strength` - Acceleration in units per second squared (typical: 9.8)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Gravity(9.8)  // Earth-like gravity
    /// Rule::Gravity(1.6)  // Moon-like gravity
    /// ```
    Gravity(f32),

    /// Reflect particles off bounding box walls.
    ///
    /// When a particle crosses a boundary, its position is clamped and
    /// its velocity component is reversed. Creates a contained simulation.
    ///
    /// The bounds are set with `.with_bounds(size)` which creates a cube
    /// from `-size` to `+size` on all axes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_bounds(1.0)           // Cube from -1 to +1
    /// .with_rule(Rule::BounceWalls)
    /// ```
    BounceWalls,

    /// Wrap particles around bounding box walls (toroidal topology).
    ///
    /// Particles exiting one side reappear on the opposite side with
    /// velocity preserved. Creates an infinite-feeling space with no edges.
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_bounds(1.0)
    /// .with_rule(Rule::WrapWalls)  // Endless space
    /// ```
    WrapWalls,

    /// Velocity damping (air resistance / friction).
    ///
    /// Reduces velocity over time. Higher values = more friction.
    /// A value of 1.0 would stop particles in ~1 second.
    ///
    /// # Parameters
    ///
    /// - `strength` - Damping coefficient (typical: 0.5 to 3.0)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Drag(1.0)   // Moderate air resistance
    /// Rule::Drag(0.1)   // Very little friction (space-like)
    /// Rule::Drag(5.0)   // Heavy friction (underwater feel)
    /// ```
    Drag(f32),

    /// Constant acceleration in any direction.
    ///
    /// Unlike Gravity which only affects Y, this applies force in any direction.
    /// Useful for wind, currents, or directional fields.
    ///
    /// # Parameters
    ///
    /// - `direction` - Acceleration vector (units per second squared)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Acceleration(Vec3::new(1.0, 0.0, 0.0))   // Rightward wind
    /// Rule::Acceleration(Vec3::new(0.0, -9.8, 0.0)) // Same as Gravity(9.8)
    /// ```
    Acceleration(Vec3),

    /// Attract particles toward a fixed point.
    ///
    /// All particles steer toward the target point. Force is constant
    /// regardless of distance (not inverse-square).
    ///
    /// # Fields
    ///
    /// - `point` - Target position to attract toward
    /// - `strength` - Force magnitude (higher = faster attraction)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::AttractTo {
    ///     point: Vec3::ZERO,     // Attract to center
    ///     strength: 2.0,
    /// }
    /// ```
    AttractTo {
        /// Target position.
        point: Vec3,
        /// Attraction strength.
        strength: f32,
    },

    /// Repel particles from a fixed point within a radius.
    ///
    /// Particles within `radius` of the point are pushed away.
    /// Force is stronger closer to the point, zero at the edge.
    ///
    /// # Fields
    ///
    /// - `point` - Center of repulsion
    /// - `strength` - Maximum force at center
    /// - `radius` - Effect radius (force falls off to zero at edge)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::RepelFrom {
    ///     point: Vec3::ZERO,
    ///     strength: 5.0,
    ///     radius: 0.5,           // Only affects particles within 0.5 units
    /// }
    /// ```
    RepelFrom {
        /// Center of repulsion.
        point: Vec3,
        /// Repulsion strength.
        strength: f32,
        /// Effect radius.
        radius: f32,
    },

    /// Rotational force around an axis (vortex/whirlpool effect).
    ///
    /// Creates tangential motion around a line through `center` along `axis`.
    /// Particles spiral around the axis, useful for tornados, whirlpools,
    /// and swirl effects.
    ///
    /// # Fields
    ///
    /// - `center` - Point on the rotation axis
    /// - `axis` - Direction of rotation axis (will be normalized)
    /// - `strength` - Rotational force (positive = counter-clockwise when looking down axis)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Vertical tornado around origin
    /// Rule::Vortex {
    ///     center: Vec3::ZERO,
    ///     axis: Vec3::Y,         // Rotate around Y axis
    ///     strength: 2.0,
    /// }
    ///
    /// // Horizontal whirlpool
    /// Rule::Vortex {
    ///     center: Vec3::new(0.0, -0.5, 0.0),
    ///     axis: Vec3::Y,
    ///     strength: 3.0,
    /// }
    /// ```
    Vortex {
        /// Point on rotation axis.
        center: Vec3,
        /// Direction of rotation axis.
        axis: Vec3,
        /// Rotational strength.
        strength: f32,
    },

    /// Noise-based chaotic force field.
    ///
    /// Applies forces based on 3D noise sampled at each particle's position.
    /// Creates organic, turbulent motion. The noise field evolves over time.
    ///
    /// # Fields
    ///
    /// - `scale` - Noise frequency (smaller = larger swirls, larger = finer detail)
    /// - `strength` - Force magnitude
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Turbulence {
    ///     scale: 2.0,      // Medium-sized turbulent structures
    ///     strength: 1.5,   // Moderate force
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// Uses built-in simplex noise. For more control, use `Rule::Custom`
    /// with `noise3()` or `fbm3()` functions directly.
    Turbulence {
        /// Noise frequency (spatial scale).
        scale: f32,
        /// Force magnitude.
        strength: f32,
    },

    /// Circular orbit around a center point.
    ///
    /// Applies forces to make particles orbit around a point. Combines
    /// centripetal attraction with tangential velocity to maintain orbit.
    ///
    /// # Fields
    ///
    /// - `center` - Point to orbit around
    /// - `strength` - Orbital force (higher = tighter orbits)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Orbit {
    ///     center: Vec3::ZERO,
    ///     strength: 2.0,
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// For stable orbits, balance with `Rule::Drag`. Without drag,
    /// particles may spiral inward or outward.
    Orbit {
        /// Center of orbit.
        center: Vec3,
        /// Orbital strength.
        strength: f32,
    },

    /// Curl noise for fluid-like, divergence-free flow.
    ///
    /// Creates smooth, swirling motion that never converges to a point
    /// (divergence-free). Particles flow like smoke or fluid. Based on
    /// the curl of a 3D noise field.
    ///
    /// # Fields
    ///
    /// - `scale` - Noise frequency (smaller = larger flow structures)
    /// - `strength` - Flow speed
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Curl {
    ///     scale: 1.5,      // Large flowing structures
    ///     strength: 2.0,   // Moderate flow speed
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// More computationally expensive than `Turbulence` (samples noise 6x).
    /// Use for fluid/smoke effects where particles shouldn't bunch up.
    Curl {
        /// Noise frequency.
        scale: f32,
        /// Flow strength.
        strength: f32,
    },

    /// Inverse-square gravity toward a fixed point.
    ///
    /// Like `NBodyGravity` but attracts to a single static point rather
    /// than between particles. Classic for black holes, attractors, and
    /// orbital mechanics around a central body.
    ///
    /// # Fields
    ///
    /// - `point` - Center of attraction
    /// - `strength` - Gravitational constant
    /// - `softening` - Minimum distance to prevent singularities
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::PointGravity {
    ///     point: Vec3::ZERO,
    ///     strength: 2.0,
    ///     softening: 0.05,
    /// }
    /// ```
    PointGravity {
        /// Center of attraction.
        point: Vec3,
        /// Gravitational strength.
        strength: f32,
        /// Softening to prevent singularities.
        softening: f32,
    },

    /// Spring force tethering particles to a point.
    ///
    /// Applies Hooke's law: force proportional to displacement from rest
    /// position. Good for bouncy effects, soft bodies, and cloth-like
    /// behavior.
    ///
    /// # Fields
    ///
    /// - `anchor` - Rest position (or use `Vec3::ZERO` for origin)
    /// - `stiffness` - Spring constant (higher = stiffer, snappier)
    /// - `damping` - Velocity damping (prevents endless oscillation)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Spring {
    ///     anchor: Vec3::ZERO,
    ///     stiffness: 5.0,
    ///     damping: 0.5,
    /// }
    /// ```
    Spring {
        /// Rest position.
        anchor: Vec3,
        /// Spring stiffness.
        stiffness: f32,
        /// Damping factor.
        damping: f32,
    },

    /// Radial force (explode/implode) with configurable falloff.
    ///
    /// Positive strength pushes particles outward (explode).
    /// Negative strength pulls particles inward (implode).
    /// The `falloff` parameter controls how the force changes with distance.
    ///
    /// # Fields
    ///
    /// - `point` - Center of the radial force
    /// - `strength` - Force magnitude (positive = outward, negative = inward)
    /// - `radius` - Maximum effect radius (0.0 = unlimited)
    /// - `falloff` - How force decreases with distance
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Explosion from center
    /// Rule::Radial {
    ///     point: Vec3::ZERO,
    ///     strength: 5.0,
    ///     radius: 2.0,
    ///     falloff: Falloff::InverseSquare,
    /// }
    ///
    /// // Black hole with smooth falloff
    /// Rule::Radial {
    ///     point: Vec3::ZERO,
    ///     strength: -3.0,  // Negative = inward
    ///     radius: 1.5,
    ///     falloff: Falloff::Smooth,
    /// }
    /// ```
    Radial {
        /// Center of radial force.
        point: Vec3,
        /// Force strength (positive = out, negative = in).
        strength: f32,
        /// Maximum effect radius (0.0 = unlimited).
        radius: f32,
        /// Distance falloff function.
        falloff: Falloff,
    },

    /// Expanding shockwave that pushes particles as it passes.
    ///
    /// Creates a ring/sphere that expands outward from the origin point.
    /// Particles get pushed when the wavefront passes through them.
    /// The wave repeats based on the `repeat` parameter.
    ///
    /// # Fields
    ///
    /// - `origin` - Center point of the shockwave
    /// - `speed` - How fast the wave expands (units per second)
    /// - `width` - Thickness of the wavefront
    /// - `strength` - Push force when wave passes
    /// - `repeat` - Time between wave repetitions (0.0 = single wave)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Single expanding shockwave
    /// Rule::Shockwave {
    ///     origin: Vec3::ZERO,
    ///     speed: 2.0,
    ///     width: 0.3,
    ///     strength: 5.0,
    ///     repeat: 0.0,  // One-time
    /// }
    ///
    /// // Repeating pulses
    /// Rule::Shockwave {
    ///     origin: Vec3::ZERO,
    ///     speed: 1.5,
    ///     width: 0.2,
    ///     strength: 3.0,
    ///     repeat: 2.0,  // Every 2 seconds
    /// }
    /// ```
    Shockwave {
        /// Center of shockwave.
        origin: Vec3,
        /// Expansion speed (units per second).
        speed: f32,
        /// Wavefront thickness.
        width: f32,
        /// Push strength.
        strength: f32,
        /// Repetition period (0.0 = no repeat).
        repeat: f32,
    },

    /// Breathing/pulsing radial force that oscillates between expand and contract.
    ///
    /// Combines explosion and implosion in a smooth oscillation.
    /// Creates organic "breathing" motion where particles expand and contract.
    ///
    /// # Fields
    ///
    /// - `point` - Center of the pulse
    /// - `strength` - Maximum force magnitude
    /// - `frequency` - Oscillation speed (Hz)
    /// - `radius` - Effect radius (0.0 = unlimited)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Pulse {
    ///     point: Vec3::ZERO,
    ///     strength: 2.0,
    ///     frequency: 0.5,  // One breath every 2 seconds
    ///     radius: 1.5,
    /// }
    /// ```
    Pulse {
        /// Center of pulse.
        point: Vec3,
        /// Maximum force strength.
        strength: f32,
        /// Oscillation frequency (Hz).
        frequency: f32,
        /// Effect radius (0.0 = unlimited).
        radius: f32,
    },

    /// Sine-wave oscillation applied to velocity.
    ///
    /// Creates pulsing, breathing, or wave-like motion. Each particle
    /// oscillates based on time and optionally creates radial waves
    /// emanating outward from the oscillation axis.
    ///
    /// # Fields
    ///
    /// - `axis` - Direction of oscillation (will be normalized)
    /// - `amplitude` - Oscillation strength
    /// - `frequency` - Oscillations per second
    /// - `spatial_scale` - If > 0, creates radial waves based on distance from axis
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Simple up-down pulse (all particles in sync)
    /// Rule::Oscillate {
    ///     axis: Vec3::Y,
    ///     amplitude: 0.5,
    ///     frequency: 2.0,
    ///     spatial_scale: 0.0,
    /// }
    ///
    /// // Radial ripples (like dropping a stone in water)
    /// Rule::Oscillate {
    ///     axis: Vec3::Y,
    ///     amplitude: 0.3,
    ///     frequency: 1.0,
    ///     spatial_scale: 5.0,  // Higher = tighter ripples
    /// }
    /// ```
    Oscillate {
        /// Direction of oscillation.
        axis: Vec3,
        /// Oscillation amplitude.
        amplitude: f32,
        /// Frequency in Hz.
        frequency: f32,
        /// Spatial wave scale (0 = uniform, >0 = traveling wave).
        spatial_scale: f32,
    },

    /// Position jitter from noise field.
    ///
    /// Adds organic, pseudo-random displacement to particle positions.
    /// Different from `Turbulence` which affects velocity - this directly
    /// offsets position for a jittery, vibrating effect.
    ///
    /// # Fields
    ///
    /// - `scale` - Noise frequency (smaller = larger jitter patterns)
    /// - `strength` - Maximum displacement
    /// - `speed` - How fast the noise field evolves
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::PositionNoise {
    ///     scale: 5.0,
    ///     strength: 0.02,
    ///     speed: 2.0,
    /// }
    /// ```
    PositionNoise {
        /// Noise frequency.
        scale: f32,
        /// Displacement strength.
        strength: f32,
        /// Time evolution speed.
        speed: f32,
    },

    /// Elastic collision response between particles.
    ///
    /// **Requires spatial hashing.** Particles within `radius` of each other
    /// exchange velocity along the collision normal, simulating elastic bounces.
    /// Also separates overlapping particles to prevent interpenetration.
    ///
    /// # Fields
    ///
    /// - `radius` - Collision distance (sum of particle radii)
    /// - `restitution` - Coefficient of restitution:
    ///   - `0.0` = perfectly inelastic (particles stick together)
    ///   - `1.0` = perfectly elastic (full energy preserved)
    ///   - `0.5` = typical bouncy collision
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.1, 32)
    /// .with_rule(Rule::Collide {
    ///     radius: 0.05,          // Collision distance
    ///     restitution: 0.8,      // Bouncy collision
    /// })
    /// ```
    Collide {
        /// Collision distance (triggers when particles are closer than this).
        radius: f32,
        /// Coefficient of restitution (0.0 = inelastic, 1.0 = elastic).
        restitution: f32,
    },

    /// N-body gravitational attraction between particles.
    ///
    /// **Requires spatial hashing.** Every particle attracts nearby particles
    /// with inverse-square force (like gravity). Classic for galaxy simulations,
    /// particle clustering, and organic clumping behavior.
    ///
    /// # Fields
    ///
    /// - `strength` - Gravitational constant (higher = stronger pull)
    /// - `softening` - Minimum distance to prevent division by zero and
    ///   extreme forces at close range. Typical: 0.01-0.05
    /// - `radius` - Maximum interaction range (for performance)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.3, 32)
    /// .with_rule(Rule::NBodyGravity {
    ///     strength: 0.5,
    ///     softening: 0.02,       // Prevents singularities
    ///     radius: 0.5,           // Only attract within range
    /// })
    /// ```
    ///
    /// # Physics Note
    ///
    /// True n-body is O(n²), but spatial hashing limits to nearby particles.
    /// For galaxy-scale simulations, combine with `Rule::Drag` to prevent
    /// runaway velocities.
    NBodyGravity {
        /// Gravitational strength.
        strength: f32,
        /// Softening parameter (prevents singularities).
        softening: f32,
        /// Maximum interaction radius.
        radius: f32,
    },

    /// Velocity smoothing with nearby particles (fluid viscosity).
    ///
    /// **Requires spatial hashing.** Particles blend their velocity with
    /// neighbors, creating smooth, fluid-like motion. Higher strength means
    /// more uniform flow; lower means more chaotic.
    ///
    /// # Fields
    ///
    /// - `radius` - Interaction range
    /// - `strength` - Blending rate (0.0-1.0 typical, higher = more viscous)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.15, 32)
    /// .with_rule(Rule::Viscosity {
    ///     radius: 0.1,
    ///     strength: 0.5,         // Medium viscosity
    /// })
    /// ```
    ///
    /// # Physics Note
    ///
    /// Similar to `Align` but uses distance-weighted averaging for smoother
    /// gradients. Good for smoke, water, and gooey substances.
    Viscosity {
        /// Interaction radius.
        radius: f32,
        /// Viscosity strength (higher = thicker fluid).
        strength: f32,
    },

    /// Density-based repulsion (SPH-style pressure).
    ///
    /// **Requires spatial hashing.** Particles in crowded areas get pushed
    /// outward. Creates incompressible fluid behavior where particles spread
    /// to fill space evenly.
    ///
    /// # Fields
    ///
    /// - `radius` - Kernel radius for density estimation
    /// - `strength` - Pressure force magnitude
    /// - `target_density` - Desired neighbor count (particles push when above this)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.15, 32)
    /// .with_rule(Rule::Pressure {
    ///     radius: 0.1,
    ///     strength: 2.0,
    ///     target_density: 8.0,   // Comfortable with ~8 neighbors
    /// })
    /// ```
    ///
    /// # Physics Note
    ///
    /// This is a simplified SPH pressure. For realistic fluids, combine with
    /// `Viscosity` and `Gravity`.
    Pressure {
        /// Kernel radius for density calculation.
        radius: f32,
        /// Pressure force strength.
        strength: f32,
        /// Target neighbor density (pushes when exceeded).
        target_density: f32,
    },

    /// Charge-based attraction and repulsion (magnetism/electrostatics).
    ///
    /// **Requires spatial hashing.** Particles attract or repel based on
    /// their `particle_type` field acting as charge polarity. Same types
    /// repel, different types attract (or vice versa).
    ///
    /// # Fields
    ///
    /// - `radius` - Interaction range
    /// - `strength` - Force magnitude
    /// - `same_repel` - If true, same types repel; if false, same types attract
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Classic magnetism: opposites attract
    /// .with_spatial_config(0.2, 32)
    /// .with_rule(Rule::Magnetism {
    ///     radius: 0.3,
    ///     strength: 1.5,
    ///     same_repel: true,      // Same charge repels
    /// })
    /// ```
    ///
    /// # Usage
    ///
    /// Set `particle_type` to 0 or 1 for two polarities. Particles with
    /// type 0 and type 1 will attract (if `same_repel: true`), while
    /// particles with matching types will repel.
    Magnetism {
        /// Interaction radius.
        radius: f32,
        /// Force strength.
        strength: f32,
        /// If true, same types repel and opposites attract.
        same_repel: bool,
    },

    /// Surface tension keeping fluid blobs together.
    ///
    /// **Requires spatial hashing.** Particles with fewer neighbors (at the
    /// edge of a group) get pulled toward the center of mass of their
    /// neighbors. Creates cohesive fluid blobs that resist dispersing.
    ///
    /// # Fields
    ///
    /// - `radius` - Neighbor detection radius
    /// - `strength` - Pull strength toward center
    /// - `threshold` - Neighbor count below which tension applies
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.15, 32)
    /// .with_rule(Rule::SurfaceTension {
    ///     radius: 0.1,
    ///     strength: 2.0,
    ///     threshold: 8.0,  // Apply tension when < 8 neighbors
    /// })
    /// ```
    SurfaceTension {
        /// Neighbor detection radius.
        radius: f32,
        /// Tension strength.
        strength: f32,
        /// Apply when neighbor count is below this.
        threshold: f32,
    },

    /// Smooth steering-based avoidance.
    ///
    /// **Requires spatial hashing.** Unlike `Separate` which pushes directly
    /// away, this steers particles to flow around obstacles smoothly. Better
    /// for flocking and crowd simulation.
    ///
    /// # Fields
    ///
    /// - `radius` - Detection distance
    /// - `strength` - Avoidance steering force
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.15, 32)
    /// .with_rule(Rule::Avoid {
    ///     radius: 0.1,
    ///     strength: 3.0,
    /// })
    /// ```
    Avoid {
        /// Detection radius.
        radius: f32,
        /// Steering strength.
        strength: f32,
    },

    /// Separation: steer away from nearby neighbors.
    ///
    /// **Requires spatial hashing.** Part of classic boids algorithm.
    /// Particles avoid crowding by steering away from neighbors.
    ///
    /// # Fields
    ///
    /// - `radius` - Detection distance
    /// - `strength` - Separation force
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.15, 32)
    /// .with_rule(Rule::Separate {
    ///     radius: 0.05,          // Personal space
    ///     strength: 2.0,
    /// })
    /// ```
    Separate {
        /// Detection radius.
        radius: f32,
        /// Separation strength.
        strength: f32,
    },

    /// Cohesion: steer toward center of nearby neighbors.
    ///
    /// **Requires spatial hashing.** Part of classic boids algorithm.
    /// Particles steer toward the average position of their neighbors.
    ///
    /// # Fields
    ///
    /// - `radius` - Detection distance
    /// - `strength` - Cohesion force
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.15, 32)
    /// .with_rule(Rule::Cohere {
    ///     radius: 0.15,          // Flock awareness range
    ///     strength: 1.0,
    /// })
    /// ```
    Cohere {
        /// Detection radius.
        radius: f32,
        /// Cohesion strength.
        strength: f32,
    },

    /// Alignment: match velocity with nearby neighbors.
    ///
    /// **Requires spatial hashing.** Part of classic boids algorithm.
    /// Particles steer to match the average velocity of neighbors.
    ///
    /// # Fields
    ///
    /// - `radius` - Detection distance
    /// - `strength` - Alignment force
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.15, 32)
    /// .with_rule(Rule::Align {
    ///     radius: 0.1,
    ///     strength: 1.5,
    /// })
    /// ```
    Align {
        /// Detection radius.
        radius: f32,
        /// Alignment strength.
        strength: f32,
    },

    /// Random wandering force for organic movement.
    ///
    /// Applies a pseudo-random force that changes over time.
    /// Each particle gets its own random direction based on index and time.
    ///
    /// # Fields
    ///
    /// - `strength` - Force magnitude
    /// - `frequency` - How fast direction changes (higher = more jittery)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Wander {
    ///     strength: 0.5,
    ///     frequency: 100.0,      // Smooth wandering
    /// }
    ///
    /// Rule::Wander {
    ///     strength: 0.3,
    ///     frequency: 500.0,      // Jittery movement
    /// }
    /// ```
    Wander {
        /// Force magnitude.
        strength: f32,
        /// Direction change rate (higher = jittery).
        frequency: f32,
    },

    /// Clamp velocity magnitude to min/max bounds.
    ///
    /// Prevents particles from stopping completely or moving too fast.
    /// Applied after other forces, before drag.
    ///
    /// # Fields
    ///
    /// - `min` - Minimum speed (use 0.0 for no minimum)
    /// - `max` - Maximum speed
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::SpeedLimit {
    ///     min: 0.5,              // Always moving
    ///     max: 3.0,              // But not too fast
    /// }
    ///
    /// Rule::SpeedLimit {
    ///     min: 0.0,              // Can stop
    ///     max: 10.0,             // Hard speed cap
    /// }
    /// ```
    SpeedLimit {
        /// Minimum speed (0.0 for no minimum).
        min: f32,
        /// Maximum speed.
        max: f32,
    },

    /// Raw WGSL code for custom behavior.
    ///
    /// For advanced users who need behavior not covered by built-in rules.
    /// The code runs in the compute shader with access to:
    ///
    /// - `p` - Current particle (read/write)
    /// - `index` - Particle index (`u32`)
    /// - `uniforms.time` - Elapsed time (`f32`)
    /// - `uniforms.delta_time` - Frame delta time (`f32`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Custom(r#"
    ///     // Oscillate Y velocity based on time
    ///     p.velocity.y += sin(uniforms.time * 2.0) * 0.1;
    ///
    ///     // Color based on speed
    ///     let speed = length(p.velocity);
    ///     p.color = vec3<f32>(speed, 0.5, 1.0 - speed);
    /// "#.to_string())
    /// ```
    ///
    /// # Note
    ///
    /// Custom rules don't have access to neighbor data. For neighbor-aware
    /// custom behavior, use [`Rule::NeighborCustom`].
    Custom(String),

    /// Raw WGSL code that runs inside the neighbor iteration loop.
    ///
    /// **Requires spatial hashing.** For advanced users who need custom
    /// particle-particle interactions not covered by built-in rules.
    ///
    /// # Available Variables
    ///
    /// Inside your code, these variables are available:
    ///
    /// - `p` - Current particle (read/write)
    /// - `other` - Neighbor particle (read-only)
    /// - `neighbor_dist` - Distance to neighbor (`f32`)
    /// - `neighbor_dir` - Unit vector from neighbor to self (`vec3<f32>`)
    /// - `neighbor_pos` - Neighbor position (`vec3<f32>`)
    /// - `neighbor_vel` - Neighbor velocity (`vec3<f32>`)
    /// - `index` - Current particle index (`u32`)
    /// - `other_idx` - Neighbor particle index (`u32`)
    /// - `uniforms.time` - Elapsed time (`f32`)
    /// - `uniforms.delta_time` - Frame delta time (`f32`)
    ///
    /// # Example: Magnetic attraction
    ///
    /// ```ignore
    /// .with_spatial_config(0.3, 32)
    /// .with_rule(Rule::NeighborCustom(r#"
    ///     if neighbor_dist < 0.2 && neighbor_dist > 0.01 {
    ///         // Inverse-square attraction
    ///         let force = 0.5 / (neighbor_dist * neighbor_dist);
    ///         p.velocity -= neighbor_dir * force * uniforms.delta_time;
    ///     }
    /// "#.into()))
    /// ```
    ///
    /// # Example: Color blending with neighbors
    ///
    /// ```ignore
    /// .with_rule(Rule::NeighborCustom(r#"
    ///     if neighbor_dist < 0.1 {
    ///         // Blend colors with nearby particles
    ///         let blend = 0.1 * (1.0 - neighbor_dist / 0.1);
    ///         p.color = mix(p.color, other.color, blend * uniforms.delta_time);
    ///     }
    /// "#.into()))
    /// ```
    ///
    /// # Example: Type-based interactions
    ///
    /// ```ignore
    /// .with_rule(Rule::NeighborCustom(r#"
    ///     // Red particles attract blue, blue repels red
    ///     if p.particle_type == 0u && other.particle_type == 1u {
    ///         if neighbor_dist < 0.3 {
    ///             p.velocity -= neighbor_dir * 2.0 * uniforms.delta_time;
    ///         }
    ///     }
    ///     if p.particle_type == 1u && other.particle_type == 0u {
    ///         if neighbor_dist < 0.2 {
    ///             p.velocity += neighbor_dir * 3.0 * uniforms.delta_time;
    ///         }
    ///     }
    /// "#.into()))
    /// ```
    ///
    /// # Performance Note
    ///
    /// This code runs once for every nearby particle pair. Keep it efficient!
    /// Complex calculations here multiply by neighbor count.
    NeighborCustom(String),

    /// Custom collision response with user-defined WGSL code.
    ///
    /// **Requires spatial hashing.** When particles are within `radius` of each
    /// other, executes your custom WGSL code. This is the power-user version
    /// of [`Rule::Collide`] for implementing custom collision physics.
    ///
    /// # Fields
    ///
    /// - `radius` - Collision distance (code only runs when closer than this)
    /// - `response` - WGSL code to execute on collision
    ///
    /// # Available Variables
    ///
    /// Inside your code, these variables are available:
    ///
    /// - `p` - Current particle (read/write)
    /// - `other` - Colliding particle (read-only)
    /// - `neighbor_dist` - Distance to neighbor (`f32`)
    /// - `neighbor_dir` - Unit vector from neighbor toward self (`vec3<f32>`)
    /// - `neighbor_pos` - Neighbor position (`vec3<f32>`)
    /// - `neighbor_vel` - Neighbor velocity (`vec3<f32>`)
    /// - `overlap` - Penetration depth: `radius - neighbor_dist` (`f32`)
    /// - `rel_vel` - Relative velocity along collision normal (`f32`, positive = approaching)
    /// - `uniforms.delta_time` - Frame delta time (`f32`)
    ///
    /// # Example: Sticky collision
    ///
    /// ```ignore
    /// .with_spatial_config(0.1, 32)
    /// .with_rule(Rule::OnCollision {
    ///     radius: 0.05,
    ///     response: r#"
    ///         // Average velocities on collision (perfectly inelastic)
    ///         let avg_vel = (p.velocity + neighbor_vel) * 0.5;
    ///         p.velocity = avg_vel;
    ///         // Separate overlapping particles
    ///         p.position += neighbor_dir * overlap * 0.5;
    ///     "#.into(),
    /// })
    /// ```
    ///
    /// # Example: Explosive collision
    ///
    /// ```ignore
    /// .with_rule(Rule::OnCollision {
    ///     radius: 0.05,
    ///     response: r#"
    ///         // Explode apart on collision
    ///         p.velocity += neighbor_dir * 5.0;
    ///         p.color = vec3<f32>(1.0, 0.5, 0.0); // Flash orange
    ///     "#.into(),
    /// })
    /// ```
    ///
    /// # Example: Type-dependent collision
    ///
    /// ```ignore
    /// .with_rule(Rule::OnCollision {
    ///     radius: 0.04,
    ///     response: r#"
    ///         if p.particle_type != other.particle_type {
    ///             // Different types bounce hard
    ///             p.velocity += neighbor_dir * rel_vel * 2.0;
    ///         } else {
    ///             // Same types stick together
    ///             p.velocity = mix(p.velocity, neighbor_vel, 0.1);
    ///         }
    ///     "#.into(),
    /// })
    /// ```
    OnCollision {
        /// Collision distance threshold.
        radius: f32,
        /// WGSL code to execute when particles collide.
        response: String,
    },

    /// Oscillator synchronization via field coupling (Kuramoto model).
    ///
    /// Each particle has an internal phase that advances over time. When the
    /// phase exceeds 2π, the particle "fires" (emits to field, runs callback,
    /// resets phase). Particles detect nearby firing through the field and
    /// adjust their phase to synchronize.
    ///
    /// This implements the Kuramoto model for coupled oscillators, which
    /// describes synchronization in fireflies, neurons, pacemaker cells,
    /// applause, and many other natural phenomena.
    ///
    /// # Fields
    ///
    /// - `phase_field` - Name of the particle field storing phase (must be `f32`)
    /// - `frequency` - Base oscillation frequency in Hz (cycles per second)
    /// - `field` - Field index to emit to and read from
    /// - `emit_amount` - How much to deposit to field when firing
    /// - `coupling` - How strongly to adjust phase when detecting neighbors (0.0-1.0)
    /// - `detection_threshold` - Minimum field value to trigger coupling
    /// - `on_fire` - Optional WGSL code to run when the oscillator fires
    ///
    /// # Available Variables in `on_fire`
    ///
    /// - `p` - Current particle (read/write)
    /// - `uniforms.time`, `uniforms.delta_time` - Time values
    ///
    /// # Example: Firefly synchronization
    ///
    /// ```ignore
    /// #[derive(Particle)]
    /// struct Firefly {
    ///     position: Vec3,
    ///     velocity: Vec3,
    ///     phase: f32,      // Oscillator phase
    ///     brightness: f32, // For visual flash
    /// }
    ///
    /// .with_field("light", FieldConfig::new(48).with_decay(0.85))
    /// .with_rule(Rule::Sync {
    ///     phase_field: "phase".into(),
    ///     frequency: 1.0,
    ///     field: 0,
    ///     emit_amount: 0.5,
    ///     coupling: 0.3,
    ///     detection_threshold: 0.1,
    ///     on_fire: Some(r#"
    ///         p.brightness = 1.0;
    ///         p.color = vec3<f32>(1.0, 1.0, 0.5);
    ///     "#.into()),
    /// })
    /// ```
    ///
    /// # Example: Neuron firing
    ///
    /// ```ignore
    /// .with_rule(Rule::Sync {
    ///     phase_field: "membrane_potential".into(),
    ///     frequency: 2.0,  // 2 Hz base firing rate
    ///     field: 0,
    ///     emit_amount: 0.8,
    ///     coupling: 0.5,
    ///     detection_threshold: 0.2,
    ///     on_fire: Some(r#"
    ///         // Action potential!
    ///         p.color = vec3<f32>(1.0, 0.3, 0.1);
    ///         // Could trigger downstream effects here
    ///     "#.into()),
    /// })
    /// ```
    ///
    /// # The Math
    ///
    /// Phase advances: `dφ/dt = ω + K * sin(φ/2) * field_value`
    ///
    /// Where `ω` is frequency and `K` is coupling. The `sin(φ/2)` term means
    /// particles respond most strongly when halfway through their cycle.
    Sync {
        /// Name of the particle field storing phase (f32, 0 to 2π).
        phase_field: String,
        /// Base oscillation frequency in Hz.
        frequency: f32,
        /// Field index to use for communication.
        field: u32,
        /// Amount to deposit to field when firing.
        emit_amount: f32,
        /// Coupling strength for phase adjustment (0.0-1.0 typical).
        coupling: f32,
        /// Minimum field value to trigger phase adjustment.
        detection_threshold: f32,
        /// Optional WGSL code to execute when the oscillator fires.
        on_fire: Option<String>,
    },

    /// Spring forces between bonded particles.
    ///
    /// Applies spring physics to particles connected by stored bond indices.
    /// Each particle stores indices of bonded neighbors in `u32` fields, and
    /// this rule applies spring forces (Hooke's law + damping) to maintain
    /// rest lengths.
    ///
    /// This enables cloth, ropes, soft bodies, and molecular simulations
    /// without requiring special bond infrastructure.
    ///
    /// # Fields
    ///
    /// - `bonds` - Names of particle fields storing bond indices (e.g., `["bond_left", "bond_right"]`)
    /// - `stiffness` - Spring constant (higher = stiffer, typical: 50-1000)
    /// - `damping` - Velocity damping along spring (prevents oscillation, typical: 5-20)
    /// - `rest_length` - Natural spring length (when stretch = 0)
    /// - `max_stretch` - Optional maximum stretch ratio before extra-stiff correction (e.g., 1.3 = 130%)
    ///
    /// # Example: Cloth
    ///
    /// ```ignore
    /// #[derive(Particle)]
    /// struct ClothPoint {
    ///     position: Vec3,
    ///     velocity: Vec3,
    ///     bond_left: u32,   // Index of left neighbor (u32::MAX = none)
    ///     bond_right: u32,
    ///     bond_up: u32,
    ///     bond_down: u32,
    /// }
    ///
    /// // In spawner, set up bond indices based on grid position
    ///
    /// .with_rule(Rule::BondSprings {
    ///     bonds: vec!["bond_left", "bond_right", "bond_up", "bond_down"],
    ///     stiffness: 800.0,
    ///     damping: 15.0,
    ///     rest_length: 0.05,
    ///     max_stretch: Some(1.3),
    /// })
    /// ```
    ///
    /// # Bond Index Convention
    ///
    /// Use `u32::MAX` (4294967295) as a sentinel for "no bond". The rule
    /// automatically skips these.
    ///
    /// # Physics Note
    ///
    /// The force applied is:
    /// - Spring: `F = stiffness * (distance - rest_length)`
    /// - Damping: `F += damping * relative_velocity_along_spring`
    /// - Over-stretch: When `distance/rest_length > max_stretch`, additional
    ///   corrective force kicks in to prevent runaway stretching.
    BondSprings {
        /// Particle field names containing bond indices.
        bonds: Vec<&'static str>,
        /// Spring stiffness (Hooke's constant).
        stiffness: f32,
        /// Damping coefficient.
        damping: f32,
        /// Rest length (natural spring length).
        rest_length: f32,
        /// Maximum stretch ratio before extra stiffening (e.g., 1.3 = 130%).
        max_stretch: Option<f32>,
    },

    /// Spring chain using sequential particle indices.
    ///
    /// Automatically bonds particle `i` to particles `i-1` and `i+1`.
    /// No bond fields needed - just spawn particles in order!
    ///
    /// Perfect for ropes, chains, tentacles, snakes, and hair.
    ///
    /// # Fields
    ///
    /// - `stiffness` - Spring constant (higher = stiffer)
    /// - `damping` - Velocity damping along spring
    /// - `rest_length` - Natural length between particles
    /// - `max_stretch` - Optional maximum stretch ratio
    ///
    /// # Example: Rope
    ///
    /// ```ignore
    /// Simulation::<RopePoint>::new()
    ///     .with_particle_count(50)  // 50-segment rope
    ///     .with_spawner(|i, _| RopePoint {
    ///         position: Vec3::new(0.0, 0.5 - i as f32 * 0.02, 0.0),
    ///         velocity: Vec3::ZERO,
    ///         pinned: if i == 0 { 1.0 } else { 0.0 },
    ///     })
    ///     .with_rule(Rule::ChainSprings {
    ///         stiffness: 500.0,
    ///         damping: 10.0,
    ///         rest_length: 0.02,
    ///         max_stretch: Some(1.2),
    ///     })
    ///     .with_rule(Rule::Gravity(5.0))
    ///     .run();
    /// ```
    ChainSprings {
        /// Spring stiffness.
        stiffness: f32,
        /// Damping coefficient.
        damping: f32,
        /// Rest length between particles.
        rest_length: f32,
        /// Maximum stretch ratio.
        max_stretch: Option<f32>,
    },

    /// Radial spring structure with center hub.
    ///
    /// Particle 0 is the center hub. All other particles connect to the center
    /// AND to their sequential neighbors, forming a wheel/web structure.
    ///
    /// Great for spider webs, wheels, radial explosions, and soft body blobs.
    ///
    /// # Fields
    ///
    /// - `hub_stiffness` - Spring strength to center hub
    /// - `ring_stiffness` - Spring strength between ring neighbors
    /// - `damping` - Velocity damping
    /// - `hub_length` - Rest length to center
    /// - `ring_length` - Rest length between ring neighbors
    ///
    /// # Example: Spider Web
    ///
    /// ```ignore
    /// .with_rule(Rule::RadialSprings {
    ///     hub_stiffness: 200.0,
    ///     ring_stiffness: 100.0,
    ///     damping: 5.0,
    ///     hub_length: 0.3,
    ///     ring_length: 0.1,
    /// })
    /// ```
    RadialSprings {
        /// Spring stiffness to center hub.
        hub_stiffness: f32,
        /// Spring stiffness between ring neighbors.
        ring_stiffness: f32,
        /// Damping coefficient.
        damping: f32,
        /// Rest length to center hub.
        hub_length: f32,
        /// Rest length between ring neighbors.
        ring_length: f32,
    },

    /// Buoyancy force based on height.
    ///
    /// Particles below `surface_y` experience upward force proportional to
    /// depth. Creates floating/sinking behavior for water simulations.
    ///
    /// # Fields
    ///
    /// - `surface_y` - Y coordinate of the water surface
    /// - `density` - Buoyancy strength (1.0 = neutral, >1 = floats, <1 = sinks)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_rule(Rule::Gravity(9.8))
    /// .with_rule(Rule::Buoyancy {
    ///     surface_y: 0.0,
    ///     density: 1.2,  // Slightly buoyant - floats up
    /// })
    /// ```
    Buoyancy {
        /// Y coordinate of the surface.
        surface_y: f32,
        /// Buoyancy factor (>1 floats, <1 sinks).
        density: f32,
    },

    /// Ground friction that slows particles near a surface.
    ///
    /// Particles below `ground_y` experience velocity damping, simulating
    /// friction with the ground. Useful for particles that roll/slide.
    ///
    /// # Fields
    ///
    /// - `ground_y` - Y coordinate of the ground plane
    /// - `strength` - Friction coefficient (0-1, higher = more friction)
    /// - `threshold` - Distance above ground where friction starts
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_rule(Rule::Gravity(9.8))
    /// .with_rule(Rule::Friction {
    ///     ground_y: -1.0,
    ///     strength: 0.8,
    ///     threshold: 0.1,
    /// })
    /// ```
    Friction {
        /// Y coordinate of the ground.
        ground_y: f32,
        /// Friction strength (0-1).
        strength: f32,
        /// Distance above ground where friction applies.
        threshold: f32,
    },

    /// Directional wind force with optional turbulence.
    ///
    /// Applies a constant directional force plus optional noise-based
    /// turbulence for realistic wind effects.
    ///
    /// # Fields
    ///
    /// - `direction` - Wind direction (will be normalized)
    /// - `strength` - Base wind force
    /// - `turbulence` - Random variation (0 = steady, 1 = very gusty)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Gentle breeze from the left
    /// Rule::Wind {
    ///     direction: Vec3::new(1.0, 0.0, 0.0),
    ///     strength: 2.0,
    ///     turbulence: 0.3,
    /// }
    ///
    /// // Strong gusty wind
    /// Rule::Wind {
    ///     direction: Vec3::new(1.0, 0.2, 0.5),
    ///     strength: 5.0,
    ///     turbulence: 0.8,
    /// }
    /// ```
    Wind {
        /// Wind direction (normalized internally).
        direction: Vec3,
        /// Wind strength.
        strength: f32,
        /// Turbulence factor (0-1).
        turbulence: f32,
    },

    /// Follow a 3D field as a flow/current.
    ///
    /// Particles are pushed in the direction of the field gradient,
    /// creating river-like currents or atmospheric flows.
    ///
    /// **Requires a field** defined with `.with_field()`.
    ///
    /// # Fields
    ///
    /// - `field` - Name of the field to follow
    /// - `strength` - Flow strength
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Define a swirling flow field
    /// .with_field("flow", 32, |x, y, z| {
    ///     (x * x + z * z).sqrt()  // Distance from Y axis
    /// })
    /// .with_rule(Rule::Current {
    ///     field: "flow",
    ///     strength: 2.0,
    /// })
    /// ```
    ///
    /// # Note
    ///
    /// The current follows the field's gradient (direction of increasing values).
    /// For circular flows, use a field based on angle; for streams, use
    /// directional gradients.
    Current {
        /// Name of the field to follow.
        field: &'static str,
        /// Current strength.
        strength: f32,
    },

    /// Respawn particles that fall below a threshold.
    ///
    /// When a particle's Y position drops below `threshold_y`, it gets
    /// teleported back to `spawn_y` with reset velocity. Perfect for
    /// fountains, rain, and endless falling effects.
    ///
    /// # Fields
    ///
    /// - `threshold_y` - Y position that triggers respawn
    /// - `spawn_y` - Y position to respawn at
    /// - `reset_velocity` - Whether to zero velocity on respawn
    ///
    /// # Example: Rain
    ///
    /// ```ignore
    /// .with_rule(Rule::Gravity(5.0))
    /// .with_rule(Rule::RespawnBelow {
    ///     threshold_y: -1.0,
    ///     spawn_y: 1.0,
    ///     reset_velocity: true,
    /// })
    /// ```
    ///
    /// # Example: Fountain (keep momentum)
    ///
    /// ```ignore
    /// .with_rule(Rule::RespawnBelow {
    ///     threshold_y: -0.5,
    ///     spawn_y: 0.0,
    ///     reset_velocity: false,  // Keep horizontal velocity
    /// })
    /// ```
    RespawnBelow {
        /// Y position that triggers respawn.
        threshold_y: f32,
        /// Y position to respawn at.
        spawn_y: f32,
        /// Whether to reset velocity on respawn.
        reset_velocity: bool,
    },

    /// Type-filtered wrapper for neighbor rules.
    ///
    /// Wraps a neighbor-based rule to only apply when particle types match.
    /// The inner rule only executes for matching type combinations.
    ///
    /// # Fields
    ///
    /// - `self_type` - Type of particle this rule applies to
    /// - `other_type` - Type of neighbors to consider (`None` = all types)
    /// - `rule` - The wrapped neighbor rule
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Prey only flocks with other prey
    /// Rule::Typed {
    ///     self_type: Species::Prey.into(),
    ///     other_type: Some(Species::Prey.into()),
    ///     rule: Box::new(Rule::Cohere { radius: 0.15, strength: 1.0 }),
    /// }
    ///
    /// // Predators collide with everyone
    /// Rule::Typed {
    ///     self_type: Species::Predator.into(),
    ///     other_type: None,  // All types
    ///     rule: Box::new(Rule::Collide { radius: 0.05, restitution: 0.8 }),
    /// }
    /// ```
    Typed {
        /// Type of particle this rule applies to.
        self_type: u32,
        /// Type of neighbors to interact with (`None` = any).
        other_type: Option<u32>,
        /// The wrapped rule (must be a neighbor rule).
        rule: Box<Rule>,
    },

    /// Change particle type on proximity to trigger type.
    ///
    /// **Requires spatial hashing.** When a particle of `from_type` is
    /// within `radius` of a particle of `trigger_type`, it may convert
    /// to `to_type` based on `probability`.
    ///
    /// # Fields
    ///
    /// - `from_type` - Type that can be converted
    /// - `trigger_type` - Type that triggers conversion
    /// - `to_type` - Result type after conversion
    /// - `radius` - Contact distance
    /// - `probability` - Chance per neighbor per frame (0.0 to 1.0)
    ///
    /// # Example: Infection
    ///
    /// ```ignore
    /// // Healthy → Infected on contact
    /// Rule::Convert {
    ///     from_type: Health::Healthy.into(),
    ///     trigger_type: Health::Infected.into(),
    ///     to_type: Health::Infected.into(),
    ///     radius: 0.08,
    ///     probability: 0.15,     // 15% chance per contact
    /// }
    ///
    /// // Self-triggered recovery (use same type with tiny radius)
    /// Rule::Convert {
    ///     from_type: Health::Infected.into(),
    ///     trigger_type: Health::Infected.into(),
    ///     to_type: Health::Recovered.into(),
    ///     radius: 0.01,
    ///     probability: 0.002,
    /// }
    /// ```
    Convert {
        /// Type that can be converted.
        from_type: u32,
        /// Type that triggers conversion.
        trigger_type: u32,
        /// Result type.
        to_type: u32,
        /// Contact distance.
        radius: f32,
        /// Conversion probability (0.0-1.0).
        probability: f32,
    },

    /// Steer toward nearest particle of target type.
    ///
    /// **Requires spatial hashing.** Finds the closest particle of
    /// `target_type` within `radius` and steers toward it.
    /// Unlike [`Rule::Cohere`], this targets the *nearest* rather than averaging.
    ///
    /// # Fields
    ///
    /// - `self_type` - Type of chaser
    /// - `target_type` - Type to chase
    /// - `radius` - Vision range
    /// - `strength` - Pursuit force
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Chase {
    ///     self_type: Species::Predator.into(),
    ///     target_type: Species::Prey.into(),
    ///     radius: 0.4,           // Can see prey within 0.4 units
    ///     strength: 4.0,         // Strong pursuit
    /// }
    /// ```
    Chase {
        /// Type of particle that chases.
        self_type: u32,
        /// Type of particle to chase.
        target_type: u32,
        /// Vision radius.
        radius: f32,
        /// Pursuit strength.
        strength: f32,
    },

    /// Steer away from nearest particle of threat type.
    ///
    /// **Requires spatial hashing.** Finds the closest particle of
    /// `threat_type` within `radius` and steers away from it.
    /// Unlike [`Rule::Separate`], this flees from the *nearest* threat only.
    ///
    /// # Fields
    ///
    /// - `self_type` - Type that evades
    /// - `threat_type` - Type to flee from
    /// - `radius` - Awareness range
    /// - `strength` - Evasion force
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Evade {
    ///     self_type: Species::Prey.into(),
    ///     threat_type: Species::Predator.into(),
    ///     radius: 0.25,          // Detect predators at 0.25 units
    ///     strength: 6.0,         // Flee faster than predator chases
    /// }
    /// ```
    Evade {
        /// Type of particle that evades.
        self_type: u32,
        /// Type of particle to flee from.
        threat_type: u32,
        /// Awareness radius.
        radius: f32,
        /// Evasion strength.
        strength: f32,
    },

    /// Increment particle age each frame.
    ///
    /// Adds `delta_time` to the particle's `age` field every frame.
    /// Use with [`Rule::Lifetime`] to kill particles after a duration,
    /// or access `p.age` in custom rules for age-based effects.
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_rule(Rule::Age)
    /// .with_rule(Rule::Lifetime(2.0))  // Die after 2 seconds
    /// ```
    ///
    /// # Custom age-based effects
    ///
    /// ```ignore
    /// .with_rule(Rule::Age)
    /// .with_rule(Rule::Custom(r#"
    ///     // Fade out as particle ages
    ///     let fade = 1.0 - (p.age / 2.0);
    ///     p.color = p.color * fade;
    /// "#.to_string()))
    /// ```
    Age,

    /// Kill particles after a duration.
    ///
    /// When a particle's `age` exceeds the specified duration, its `alive`
    /// field is set to 0. Dead particles are not simulated or rendered.
    ///
    /// # Parameters
    ///
    /// - `seconds` - Maximum particle lifetime
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_rule(Rule::Age)            // Must age particles first
    /// .with_rule(Rule::Lifetime(3.0))  // Die after 3 seconds
    /// ```
    ///
    /// # Note
    ///
    /// This rule requires [`Rule::Age`] to be active, otherwise particles
    /// never age and will never die.
    Lifetime(f32),

    /// Fade out particle color over its lifetime.
    ///
    /// Multiplies particle color by `(1.0 - age / duration)`, creating a
    /// smooth fade to black as the particle ages.
    ///
    /// # Parameters
    ///
    /// - `duration` - Time in seconds over which to fade (should match Lifetime)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_rule(Rule::Age)
    /// .with_rule(Rule::FadeOut(2.0))   // Fade over 2 seconds
    /// .with_rule(Rule::Lifetime(2.0))  // Die at 2 seconds
    /// ```
    ///
    /// # Note
    ///
    /// Requires [`Rule::Age`] to be active. Works by dimming RGB values,
    /// not alpha (particles render as additive blended points).
    FadeOut(f32),

    /// Shrink particle scale over its lifetime.
    ///
    /// Sets particle scale to `1.0 - (age / duration)`, shrinking from
    /// full size to zero as the particle ages.
    ///
    /// # Parameters
    ///
    /// - `duration` - Time in seconds over which to shrink (should match Lifetime)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_rule(Rule::Age)
    /// .with_rule(Rule::ShrinkOut(2.0)) // Shrink over 2 seconds
    /// .with_rule(Rule::Lifetime(2.0))  // Die at 2 seconds
    /// ```
    ///
    /// # Note
    ///
    /// Requires [`Rule::Age`] to be active.
    ShrinkOut(f32),

    /// Lerp particle color from start to end over its lifetime.
    ///
    /// Smoothly transitions particle color based on age. At age 0, color
    /// is `start`. At `duration`, color is `end`.
    ///
    /// # Fields
    ///
    /// - `start` - Color at birth (RGB, 0.0-1.0)
    /// - `end` - Color at death (RGB, 0.0-1.0)
    /// - `duration` - Transition time in seconds (should match Lifetime)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_rule(Rule::Age)
    /// .with_rule(Rule::ColorOverLife {
    ///     start: Vec3::new(1.0, 1.0, 0.0),  // Yellow
    ///     end: Vec3::new(1.0, 0.0, 0.0),    // Red
    ///     duration: 2.0,
    /// })
    /// .with_rule(Rule::Lifetime(2.0))
    /// ```
    ///
    /// # Note
    ///
    /// Requires [`Rule::Age`] to be active. Overwrites color each frame,
    /// so place after any other color modifications.
    ColorOverLife {
        /// Color at age 0.
        start: Vec3,
        /// Color at max age.
        end: Vec3,
        /// Transition duration in seconds.
        duration: f32,
    },

    /// Color particles based on their speed.
    ///
    /// Maps velocity magnitude to a color gradient. Slow particles get
    /// `slow_color`, fast particles get `fast_color`. Very common effect
    /// for visualizing flow and energy.
    ///
    /// # Fields
    ///
    /// - `slow_color` - Color at zero velocity
    /// - `fast_color` - Color at max_speed
    /// - `max_speed` - Speed at which color is fully `fast_color`
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::ColorBySpeed {
    ///     slow_color: Vec3::new(0.2, 0.3, 0.8),  // Blue when slow
    ///     fast_color: Vec3::new(1.0, 0.9, 0.5),  // Yellow when fast
    ///     max_speed: 2.0,
    /// }
    /// ```
    ColorBySpeed {
        /// Color at zero speed.
        slow_color: Vec3,
        /// Color at max speed.
        fast_color: Vec3,
        /// Speed for full fast_color.
        max_speed: f32,
    },

    /// Color particles based on their age.
    ///
    /// Maps age to a color gradient. Similar to `ColorOverLife` but uses
    /// a max_age parameter instead of duration, and won't reset.
    ///
    /// # Fields
    ///
    /// - `young_color` - Color at age 0
    /// - `old_color` - Color at max_age
    /// - `max_age` - Age at which color is fully `old_color`
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_rule(Rule::Age)
    /// .with_rule(Rule::ColorByAge {
    ///     young_color: Vec3::new(1.0, 1.0, 1.0),  // White when young
    ///     old_color: Vec3::new(1.0, 0.3, 0.1),    // Red when old
    ///     max_age: 3.0,
    /// })
    /// ```
    ColorByAge {
        /// Color at age 0.
        young_color: Vec3,
        /// Color at max age.
        old_color: Vec3,
        /// Age for full old_color.
        max_age: f32,
    },

    /// Scale particles based on their speed.
    ///
    /// Fast-moving particles become larger (or smaller). Creates visual
    /// emphasis on motion and energy.
    ///
    /// # Fields
    ///
    /// - `min_scale` - Scale at zero velocity
    /// - `max_scale` - Scale at max_speed
    /// - `max_speed` - Speed at which scale is `max_scale`
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Fast particles get bigger
    /// Rule::ScaleBySpeed {
    ///     min_scale: 0.5,
    ///     max_scale: 2.0,
    ///     max_speed: 3.0,
    /// }
    ///
    /// // Fast particles get smaller (motion blur feel)
    /// Rule::ScaleBySpeed {
    ///     min_scale: 1.0,
    ///     max_scale: 0.3,
    ///     max_speed: 2.0,
    /// }
    /// ```
    ScaleBySpeed {
        /// Scale at zero speed.
        min_scale: f32,
        /// Scale at max speed.
        max_scale: f32,
        /// Speed for full max_scale.
        max_speed: f32,
    },

    /// Probabilistic execution of custom WGSL code.
    ///
    /// Runs the provided action with a given probability each frame.
    /// Useful for random events like spontaneous death, mutation, color changes,
    /// or any stochastic behavior.
    ///
    /// # Fields
    ///
    /// - `probability` - Chance per frame (0.0 to 1.0). Note: higher framerates
    ///   mean more rolls, so 0.01 at 60fps ≈ 0.6 triggers/second.
    /// - `action` - WGSL code to execute when the roll succeeds.
    ///
    /// # Available Variables
    ///
    /// Same as [`Rule::Custom`]:
    /// - `p` - Current particle (read/write)
    /// - `index` - Particle index (`u32`)
    /// - `uniforms.time`, `uniforms.delta_time` - Time values
    /// - Field functions if fields are configured
    ///
    /// # Example: Random death
    ///
    /// ```ignore
    /// Rule::Maybe {
    ///     probability: 0.001,  // 0.1% chance per frame
    ///     action: r#"
    ///         p.alive = false;
    ///     "#.into(),
    /// }
    /// ```
    ///
    /// # Example: Random color flash
    ///
    /// ```ignore
    /// Rule::Maybe {
    ///     probability: 0.02,
    ///     action: r#"
    ///         p.color = vec3<f32>(1.0, 0.0, 0.0);  // Flash red
    ///     "#.into(),
    /// }
    /// ```
    ///
    /// # Example: Spontaneous direction change
    ///
    /// ```ignore
    /// Rule::Maybe {
    ///     probability: 0.005,
    ///     action: r#"
    ///         // Random new direction (using position as seed)
    ///         let seed = p.position * 12.9898 + uniforms.time;
    ///         let rx = fract(sin(dot(seed.xy, vec2(12.9898, 78.233))) * 43758.5453);
    ///         let ry = fract(sin(dot(seed.yz, vec2(12.9898, 78.233))) * 43758.5453);
    ///         let rz = fract(sin(dot(seed.xz, vec2(12.9898, 78.233))) * 43758.5453);
    ///         p.velocity = normalize(vec3(rx, ry, rz) - 0.5) * length(p.velocity);
    ///     "#.into(),
    /// }
    /// ```
    Maybe {
        /// Probability of executing action (0.0 to 1.0).
        probability: f32,
        /// WGSL code to execute when probability check passes.
        action: String,
    },

    /// Conditional action execution.
    ///
    /// Evaluates a WGSL boolean condition and executes action if true.
    /// The fundamental "when X, do Y" pattern for reactive particle behavior.
    ///
    /// # Fields
    ///
    /// - `condition` - WGSL boolean expression (must evaluate to `bool`)
    /// - `action` - WGSL code to execute when condition is true
    ///
    /// # Available Variables
    ///
    /// Same as [`Rule::Custom`]:
    /// - `p` - Current particle (read/write)
    /// - `index` - Particle index (`u32`)
    /// - `uniforms.time`, `uniforms.delta_time` - Time values
    /// - Field functions if fields are configured
    ///
    /// # Example: Low energy warning
    ///
    /// ```ignore
    /// Rule::Trigger {
    ///     condition: "p.energy < 0.2".into(),
    ///     action: r#"
    ///         p.color = vec3<f32>(1.0, 0.0, 0.0);  // Flash red
    ///     "#.into(),
    /// }
    /// ```
    ///
    /// # Example: Boundary reaction
    ///
    /// ```ignore
    /// Rule::Trigger {
    ///     condition: "p.position.y < -0.8".into(),
    ///     action: r#"
    ///         p.velocity.y = abs(p.velocity.y) * 1.5;  // Bounce up hard
    ///         p.color = vec3<f32>(1.0, 1.0, 1.0);
    ///     "#.into(),
    /// }
    /// ```
    ///
    /// # Example: Field-reactive behavior
    ///
    /// ```ignore
    /// Rule::Trigger {
    ///     condition: "field_read(0u, p.position) > 0.5".into(),
    ///     action: r#"
    ///         p.velocity *= 2.0;  // Speed up in high-field regions
    ///     "#.into(),
    /// }
    /// ```
    ///
    /// # Example: Compound conditions
    ///
    /// ```ignore
    /// Rule::Trigger {
    ///     condition: "p.age > 2.0 && p.energy < 0.3".into(),
    ///     action: r#"
    ///         p.state = 2u;  // Transition to dying state
    ///     "#.into(),
    /// }
    /// ```
    Trigger {
        /// WGSL boolean expression (e.g., "p.energy < 0.1").
        condition: String,
        /// WGSL code to execute when condition is true.
        action: String,
    },

    /// Periodic time-based action execution.
    ///
    /// Executes action at regular intervals. Each particle fires independently
    /// based on global time, with optional per-particle phase offset for
    /// staggered timing.
    ///
    /// # Fields
    ///
    /// - `interval` - Time between executions in seconds
    /// - `phase_field` - Optional particle field name for phase offset (creates staggered pulses)
    /// - `action` - WGSL code to execute on each pulse
    ///
    /// # Available Variables
    ///
    /// Same as [`Rule::Custom`]:
    /// - `p` - Current particle (read/write)
    /// - `index` - Particle index (`u32`)
    /// - `uniforms.time`, `uniforms.delta_time` - Time values
    ///
    /// # Example: Heartbeat emission
    ///
    /// ```ignore
    /// Rule::Periodic {
    ///     interval: 0.5,  // Every 0.5 seconds
    ///     phase_field: None,
    ///     action: r#"
    ///         field_write(0u, p.position, 1.0);
    ///         p.color = vec3<f32>(1.0, 0.5, 0.5);
    ///     "#.into(),
    /// }
    /// ```
    ///
    /// # Example: Staggered pulses using phase field
    ///
    /// ```ignore
    /// // Particles pulse at same interval but offset by their phase
    /// Rule::Periodic {
    ///     interval: 1.0,
    ///     phase_field: Some("phase".into()),  // Uses p.phase for offset
    ///     action: r#"
    ///         p.brightness = 1.0;
    ///     "#.into(),
    /// }
    /// ```
    ///
    /// # Example: Periodic spawning trigger
    ///
    /// ```ignore
    /// Rule::Periodic {
    ///     interval: 2.0,
    ///     phase_field: None,
    ///     action: r#"
    ///         p.should_spawn = 1u;  // Flag for spawn system
    ///     "#.into(),
    /// }
    /// ```
    Periodic {
        /// Time between pulses in seconds.
        interval: f32,
        /// Optional particle field for phase offset (staggered timing).
        phase_field: Option<String>,
        /// WGSL code to execute on each pulse.
        action: String,
    },

    /// Move toward higher or lower field values (chemotaxis/gradient following).
    ///
    /// Samples the field gradient at each particle's position and applies
    /// force in that direction. Classic for slime mold, bacteria following
    /// nutrients, heat-seeking, or any gradient-based navigation.
    ///
    /// # Fields
    ///
    /// - `field` - Field index to sample
    /// - `strength` - Force magnitude
    /// - `ascending` - If true, move toward higher values; if false, toward lower
    ///
    /// # How It Works
    ///
    /// Samples field at particle position ± small offset to estimate gradient,
    /// then applies force along that gradient direction.
    ///
    /// # Example: Slime mold following pheromones
    ///
    /// ```ignore
    /// // Particles emit pheromones and follow the gradient
    /// .with_field("pheromone", FieldConfig::new(64).with_decay(0.98))
    /// .with_rule(Rule::Custom(r#"
    ///     field_write(0u, p.position, 0.1);  // Emit pheromone
    /// "#.into()))
    /// .with_rule(Rule::Gradient {
    ///     field: 0,
    ///     strength: 2.0,
    ///     ascending: true,  // Move toward higher pheromone
    /// })
    /// ```
    ///
    /// # Example: Heat-seeking particles
    ///
    /// ```ignore
    /// Rule::Gradient {
    ///     field: 0,  // Heat field
    ///     strength: 5.0,
    ///     ascending: true,  // Move toward heat
    /// }
    /// ```
    ///
    /// # Example: Flee from danger
    ///
    /// ```ignore
    /// Rule::Gradient {
    ///     field: 1,  // Danger field
    ///     strength: 3.0,
    ///     ascending: false,  // Move away from danger (descending)
    /// }
    /// ```
    Gradient {
        /// Field index to sample for gradient.
        field: u32,
        /// Force strength.
        strength: f32,
        /// If true, move toward higher values; if false, toward lower.
        ascending: bool,
    },

    /// Smoothly interpolate a particle field toward a target value.
    ///
    /// Exponential decay toward the target: `field = lerp(field, target, rate * dt)`.
    /// Useful for equilibration, smooth transitions, relaxation dynamics.
    ///
    /// # Fields
    ///
    /// - `field` - Particle field name to interpolate (must be `f32`)
    /// - `target` - Target value to approach
    /// - `rate` - Interpolation speed (higher = faster approach)
    ///
    /// # Example: Temperature equilibration
    ///
    /// ```ignore
    /// // Temperature relaxes toward ambient (0.5) over time
    /// Rule::Lerp {
    ///     field: "temperature".into(),
    ///     target: 0.5,
    ///     rate: 2.0,
    /// }
    /// ```
    ///
    /// # Example: Energy decay
    ///
    /// ```ignore
    /// Rule::Lerp {
    ///     field: "energy".into(),
    ///     target: 0.0,
    ///     rate: 0.5,  // Slow decay
    /// }
    /// ```
    ///
    /// # Example: Brightness fade
    ///
    /// ```ignore
    /// Rule::Lerp {
    ///     field: "brightness".into(),
    ///     target: 0.1,  // Dim but not off
    ///     rate: 3.0,
    /// }
    /// ```
    Lerp {
        /// Particle field name to interpolate (f32).
        field: String,
        /// Target value to approach.
        target: f32,
        /// Interpolation rate (speed of approach).
        rate: f32,
    },

    /// Classic boid flocking behavior (separation + cohesion + alignment).
    ///
    /// **Requires spatial hashing.** Combines the three fundamental boid rules
    /// into one convenient rule. For fine-grained control, use the individual
    /// [`Rule::Separate`], [`Rule::Cohere`], and [`Rule::Align`] rules.
    ///
    /// # Fields
    ///
    /// - `radius` - Detection distance for all three behaviors
    /// - `separation` - Strength of avoidance (prevent crowding)
    /// - `cohesion` - Strength of attraction to flock center
    /// - `alignment` - Strength of velocity matching
    ///
    /// # Example: Basic boid flock
    ///
    /// ```ignore
    /// .with_spatial_config(0.2, 32)
    /// .with_rule(Rule::Flock {
    ///     radius: 0.15,
    ///     separation: 2.0,
    ///     cohesion: 1.0,
    ///     alignment: 1.5,
    /// })
    /// .with_rule(Rule::SpeedLimit { min: 0.5, max: 2.0 })
    /// ```
    ///
    /// # Tuning Tips
    ///
    /// - Higher separation → more spread out, less clumping
    /// - Higher cohesion → tighter groups, may become too dense
    /// - Higher alignment → smoother motion, more synchronized turns
    /// - Balance is key: start with separation=2, cohesion=1, alignment=1.5
    Flock {
        /// Detection radius for all behaviors.
        radius: f32,
        /// Separation strength (avoid crowding).
        separation: f32,
        /// Cohesion strength (move toward center).
        cohesion: f32,
        /// Alignment strength (match velocity).
        alignment: f32,
    },

    /// Conditional particle death.
    ///
    /// Evaluates a WGSL condition and "kills" the particle if true by setting
    /// a specified boolean field to false. Useful for lifecycle management,
    /// particles leaving bounds, energy depletion, etc.
    ///
    /// # Fields
    ///
    /// - `condition` - WGSL boolean expression for when to die
    /// - `field` - Particle field name to set false (typically "alive")
    ///
    /// # Example: Die when out of energy
    ///
    /// ```ignore
    /// #[derive(Particle)]
    /// struct Cell {
    ///     // ...
    ///     alive: bool,
    ///     energy: f32,
    /// }
    ///
    /// Rule::Die {
    ///     condition: "p.energy <= 0.0".into(),
    ///     field: "alive".into(),
    /// }
    /// ```
    ///
    /// # Example: Die when old
    ///
    /// ```ignore
    /// Rule::Die {
    ///     condition: "p.age > 10.0".into(),
    ///     field: "alive".into(),
    /// }
    /// ```
    ///
    /// # Example: Die when leaving bounds
    ///
    /// ```ignore
    /// Rule::Die {
    ///     condition: "length(p.position) > 2.0".into(),
    ///     field: "alive".into(),
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// The particle isn't removed from simulation (that would require CPU
    /// intervention). Instead, "dead" particles can be:
    /// - Rendered invisible (check alive in color logic)
    /// - Recycled (respawn at new position when alive == false)
    /// - Ignored in interactions (check alive in neighbor rules)
    Die {
        /// WGSL boolean condition for death.
        condition: String,
        /// Particle field to set false (e.g., "alive").
        field: String,
    },

    /// Finite state machine with conditional transitions.
    ///
    /// Particles have a state (stored in a `u32` field) and transition between
    /// states based on WGSL conditions. Useful for lifecycle stages, behavior
    /// modes, or any discrete state-based logic.
    ///
    /// # Fields
    ///
    /// - `field` - Particle field storing state (must be `u32`)
    /// - `transitions` - List of (from_state, to_state, condition) tuples
    ///
    /// # Example: Simple lifecycle
    ///
    /// ```ignore
    /// #[derive(Particle)]
    /// struct Cell {
    ///     // ...
    ///     state: u32,  // 0=young, 1=mature, 2=old, 3=dead
    ///     age: f32,
    /// }
    ///
    /// Rule::State {
    ///     field: "state".into(),
    ///     transitions: vec![
    ///         (0, 1, "p.age > 2.0".into()),   // young → mature
    ///         (1, 2, "p.age > 5.0".into()),   // mature → old
    ///         (2, 3, "p.age > 8.0".into()),   // old → dead
    ///     ],
    /// }
    /// ```
    ///
    /// # Example: Behavior modes
    ///
    /// ```ignore
    /// // States: 0=idle, 1=hunting, 2=fleeing
    /// Rule::State {
    ///     field: "behavior".into(),
    ///     transitions: vec![
    ///         (0, 1, "p.hunger > 0.7".into()),           // idle → hunting
    ///         (0, 2, "p.threat_level > 0.5".into()),     // idle → fleeing
    ///         (1, 0, "p.hunger < 0.3".into()),           // hunting → idle
    ///         (2, 0, "p.threat_level < 0.2".into()),     // fleeing → idle
    ///     ],
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// - Transitions are evaluated in order; first matching transition wins
    /// - Use separate [`Rule::Trigger`] rules to execute actions on state entry
    /// - Combine with [`Rule::Custom`] to implement state-dependent behavior
    State {
        /// Particle field storing state (u32).
        field: String,
        /// Transitions: (from_state, to_state, condition).
        transitions: Vec<(u32, u32, String)>,
    },

    /// Full-featured agent state machine.
    ///
    /// A more powerful alternative to [`Rule::State`] that supports:
    /// - Entry actions (run once when entering a state)
    /// - Update actions (run every frame while in a state)
    /// - Exit actions (run once when leaving a state)
    /// - Priority-based transitions
    /// - Optional state duration tracking
    ///
    /// # Particle Requirements
    ///
    /// Your particle must have:
    /// - A `u32` field to store the current state (specified by `state_field`)
    /// - A `u32` field to store the previous state for edge detection (`prev_state_field`)
    /// - Optionally, a `f32` field to track time in current state (`state_timer_field`)
    ///
    /// # Example: Predator behavior
    ///
    /// ```ignore
    /// #[derive(Particle, Clone)]
    /// struct Predator {
    ///     position: Vec3,
    ///     velocity: Vec3,
    ///     state: u32,        // 0=roaming, 1=chasing, 2=eating, 3=resting
    ///     prev_state: u32,
    ///     state_timer: f32,
    ///     energy: f32,
    ///     target_dist: f32,
    /// }
    ///
    /// Rule::Agent {
    ///     state_field: "state".into(),
    ///     prev_state_field: "prev_state".into(),
    ///     state_timer_field: Some("state_timer".into()),
    ///     states: vec![
    ///         AgentState::new(0)
    ///             .named("roaming")
    ///             .on_update("p.velocity += rand_sphere(index) * 0.1;")
    ///             .transition(1, "p.target_dist < 0.5")     // See prey → chase
    ///             .transition(3, "p.energy < 0.2"),         // Tired → rest
    ///
    ///         AgentState::new(1)
    ///             .named("chasing")
    ///             .on_enter("p.color = vec3<f32>(1.0, 0.0, 0.0);")  // Turn red
    ///             .on_update("p.energy -= 0.01 * uniforms.delta_time;")
    ///             .transition(2, "p.target_dist < 0.05")    // Caught → eat
    ///             .transition(0, "p.target_dist > 1.0"),    // Lost prey → roam
    ///
    ///         AgentState::new(2)
    ///             .named("eating")
    ///             .on_enter("p.velocity = vec3<f32>(0.0);")
    ///             .on_update("p.energy += 0.1 * uniforms.delta_time;")
    ///             .transition(0, "p.state_timer > 2.0"),    // Done eating → roam
    ///
    ///         AgentState::new(3)
    ///             .named("resting")
    ///             .on_enter("p.color = vec3<f32>(0.5, 0.5, 0.5);")
    ///             .on_update("p.energy += 0.05 * uniforms.delta_time;")
    ///             .transition(0, "p.energy > 0.8"),         // Rested → roam
    ///     ],
    /// }
    /// ```
    ///
    /// # How It Works
    ///
    /// Each frame, for each particle:
    /// 1. If state changed since last frame, run the exit action of the old state
    ///    and the entry action of the new state
    /// 2. Run the update action for the current state
    /// 3. Increment the state timer (if configured)
    /// 4. Check transitions in priority order; first matching transition wins
    /// 5. If a transition fires, update the state field (entry/exit run next frame)
    ///
    /// # Note
    ///
    /// - State timer resets to 0 when the state changes
    /// - Transitions are sorted by priority (highest first) at compile time
    /// - Use `prev_state_field` to detect state changes and run entry/exit actions
    Agent {
        /// Particle field storing current state (u32).
        state_field: String,
        /// Particle field storing previous frame's state (u32).
        /// Used to detect state changes and trigger entry/exit actions.
        prev_state_field: String,
        /// Optional field to track time in current state (f32).
        /// Resets to 0 on state change.
        state_timer_field: Option<String>,
        /// The state definitions.
        states: Vec<AgentState>,
    },

    /// Grow or shrink particle scale over time.
    ///
    /// Changes `p.scale` at a constant rate, clamped to min/max bounds.
    /// Positive rate = grow, negative rate = shrink.
    ///
    /// # Fields
    ///
    /// - `rate` - Scale change per second (can be negative)
    /// - `min` - Minimum scale (won't shrink below this)
    /// - `max` - Maximum scale (won't grow above this)
    ///
    /// # Example: Growing particles
    ///
    /// ```ignore
    /// Rule::Grow { rate: 0.5, min: 0.1, max: 2.0 }
    /// ```
    ///
    /// # Example: Shrinking particles
    ///
    /// ```ignore
    /// Rule::Grow { rate: -0.3, min: 0.0, max: 1.0 }
    /// ```
    ///
    /// # Example: Grow then die when too big
    ///
    /// ```ignore
    /// Rule::Grow { rate: 0.2, min: 0.1, max: 3.0 },
    /// Rule::Die { condition: "p.scale >= 3.0".into(), field: "alive".into() },
    /// ```
    Grow {
        /// Scale change per second (positive = grow, negative = shrink).
        rate: f32,
        /// Minimum scale bound.
        min: f32,
        /// Maximum scale bound.
        max: f32,
    },

    /// Multiplicative decay of a field toward zero.
    ///
    /// Each frame: `field *= rate`. Different from [`Rule::Lerp`] which does
    /// additive interpolation. Decay is good for exponential falloff (energy,
    /// heat, intensity).
    ///
    /// # Fields
    ///
    /// - `field` - Particle field to decay (f32)
    /// - `rate` - Multiplier per second (0.0-1.0 for decay, >1.0 for growth)
    ///
    /// # Example: Energy decay
    ///
    /// ```ignore
    /// // Energy halves roughly every second
    /// Rule::Decay { field: "energy".into(), rate: 0.5 }
    /// ```
    ///
    /// # Example: Slow fade
    ///
    /// ```ignore
    /// Rule::Decay { field: "brightness".into(), rate: 0.9 }
    /// ```
    ///
    /// # Math Note
    ///
    /// The actual per-frame multiplier is `rate^delta_time` to be framerate
    /// independent. A rate of 0.5 means "multiply by 0.5 per second".
    Decay {
        /// Field to decay (f32).
        field: String,
        /// Decay rate per second (0.5 = halve per second).
        rate: f32,
    },

    /// Buoyancy force based on per-particle density.
    ///
    /// Applies vertical force based on particle density vs medium density.
    /// Light particles (density < medium) float up, heavy particles sink.
    /// Classic for fluid simulations with varying densities.
    ///
    /// Unlike `Rule::Buoyancy` which uses a fixed buoyancy factor, this reads
    /// density from a particle field, enabling per-particle buoyancy effects.
    ///
    /// # Fields
    ///
    /// - `density_field` - Particle field storing density (f32)
    /// - `medium_density` - Density of the surrounding medium
    /// - `strength` - Force multiplier
    ///
    /// # Example: Oil and water
    ///
    /// ```ignore
    /// // Oil (density 0.8) floats, rocks (density 2.5) sink
    /// Rule::DensityBuoyancy {
    ///     density_field: "density".into(),
    ///     medium_density: 1.0,  // Water
    ///     strength: 5.0,
    /// }
    /// ```
    ///
    /// # Physics Note
    ///
    /// Force = (medium_density - particle_density) * strength * up
    /// Positive when lighter than medium (floats), negative when heavier (sinks).
    DensityBuoyancy {
        /// Particle field storing density (f32).
        density_field: String,
        /// Density of surrounding medium.
        medium_density: f32,
        /// Force strength multiplier.
        strength: f32,
    },

    /// Property diffusion through neighbor averaging.
    ///
    /// **Requires spatial hashing.** A particle's property value moves toward
    /// the average of its neighbors. Classic for heat diffusion, chemical
    /// concentration spreading, or any equilibrating property.
    ///
    /// # Fields
    ///
    /// - `field` - Particle field to diffuse (f32)
    /// - `rate` - Diffusion speed (0.0-1.0, higher = faster equilibration)
    /// - `radius` - Neighbor detection distance
    ///
    /// # Example: Heat diffusion
    ///
    /// ```ignore
    /// Rule::Diffuse {
    ///     field: "temperature".into(),
    ///     rate: 0.3,
    ///     radius: 0.15,
    /// }
    /// ```
    ///
    /// # Example: Chemical spreading
    ///
    /// ```ignore
    /// Rule::Diffuse {
    ///     field: "concentration".into(),
    ///     rate: 0.1,
    ///     radius: 0.1,
    /// }
    /// ```
    Diffuse {
        /// Particle field to diffuse (f32).
        field: String,
        /// Diffusion rate (0.0-1.0).
        rate: f32,
        /// Neighbor detection radius.
        radius: f32,
    },

    /// Scale accelerations by inverse mass (F=ma → a=F/m).
    ///
    /// Makes heavy particles sluggish and light particles responsive.
    /// Apply this rule AFTER force-applying rules to scale their effect
    /// by particle mass.
    ///
    /// # Fields
    ///
    /// - `field` - Particle field storing mass (f32, should be > 0)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Apply forces first
    /// Rule::Gravity(9.8),
    /// Rule::Attract { point: Vec3::ZERO, strength: 2.0 },
    /// // Then scale by mass
    /// Rule::Mass { field: "mass".into() },
    /// ```
    ///
    /// # Implementation Note
    ///
    /// This divides velocity changes by mass. For proper physics, place it
    /// after all force rules but before position integration (which RDPE
    /// does automatically).
    Mass {
        /// Particle field storing mass (f32).
        field: String,
    },

    /// Animate a property from start to end over a duration.
    ///
    /// Smoothly interpolates a field based on elapsed time from a timer field.
    /// Great for spawn animations, death fades, or any time-based transitions.
    ///
    /// # Fields
    ///
    /// - `field` - Property to animate (f32)
    /// - `from` - Starting value
    /// - `to` - Ending value
    /// - `duration` - Animation duration in seconds
    /// - `timer_field` - Particle field tracking elapsed time (use with Rule::Age)
    ///
    /// # Example: Spawn scale-in
    ///
    /// ```ignore
    /// Rule::Age { field: "age".into() },
    /// Rule::Tween {
    ///     field: "scale".into(),
    ///     from: 0.0,
    ///     to: 1.0,
    ///     duration: 0.5,
    ///     timer_field: "age".into(),
    /// }
    /// ```
    ///
    /// # Example: Fade out before death
    ///
    /// ```ignore
    /// // Assumes death_timer starts counting when dying
    /// Rule::Tween {
    ///     field: "alpha".into(),
    ///     from: 1.0,
    ///     to: 0.0,
    ///     duration: 1.0,
    ///     timer_field: "death_timer".into(),
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// Animation clamps at `to` value when timer exceeds duration.
    Tween {
        /// Property to animate (f32).
        field: String,
        /// Starting value.
        from: f32,
        /// Ending value.
        to: f32,
        /// Animation duration in seconds.
        duration: f32,
        /// Timer field tracking elapsed time.
        timer_field: String,
    },

    /// Binary step function (Schmitt trigger without hysteresis).
    ///
    /// Outputs one value when input is above threshold, another when below.
    /// Classic for state transitions, on/off switches, or discretizing
    /// continuous values.
    ///
    /// # Fields
    ///
    /// - `input_field` - Field to test against threshold
    /// - `output_field` - Field to write result to
    /// - `threshold` - The threshold value
    /// - `above` - Value to output when input >= threshold
    /// - `below` - Value to output when input < threshold
    ///
    /// # Example: Binary alive/dead state
    ///
    /// ```ignore
    /// Rule::Threshold {
    ///     input_field: "health".into(),
    ///     output_field: "alive".into(),
    ///     threshold: 0.0,
    ///     above: 1.0,
    ///     below: 0.0,
    /// }
    /// ```
    ///
    /// # Example: Hot/cold indicator
    ///
    /// ```ignore
    /// Rule::Threshold {
    ///     input_field: "temperature".into(),
    ///     output_field: "is_hot".into(),
    ///     threshold: 100.0,
    ///     above: 1.0,
    ///     below: 0.0,
    /// }
    /// ```
    Threshold {
        /// Field to test.
        input_field: String,
        /// Field to write result to.
        output_field: String,
        /// Threshold value.
        threshold: f32,
        /// Output when input >= threshold.
        above: f32,
        /// Output when input < threshold.
        below: f32,
    },

    /// Conditional action gate.
    ///
    /// Executes WGSL code only when a condition is true. Like `Rule::Trigger`
    /// but without the "only once" semantic - runs every frame the condition
    /// is met.
    ///
    /// # Fields
    ///
    /// - `condition` - WGSL boolean expression
    /// - `action` - WGSL code to execute when condition is true
    ///
    /// # Example: Boost speed when energy is high
    ///
    /// ```ignore
    /// Rule::Gate {
    ///     condition: "p.energy > 0.8".into(),
    ///     action: "p.velocity *= 1.5;".into(),
    /// }
    /// ```
    ///
    /// # Example: Glow when nearby other particles
    ///
    /// ```ignore
    /// Rule::Gate {
    ///     condition: "p.neighbor_count > 5.0".into(),
    ///     action: "p.brightness = 1.0;".into(),
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// For one-shot triggers (fire once then stop), use [`Rule::Trigger`].
    Gate {
        /// WGSL boolean condition.
        condition: String,
        /// WGSL code to run when true.
        action: String,
    },

    /// Add procedural noise to a field.
    ///
    /// Applies smooth Perlin-style noise to a particle field based on
    /// position and/or time. Great for organic movement, flickering,
    /// or natural variation.
    ///
    /// # Fields
    ///
    /// - `field` - Field to add noise to
    /// - `amplitude` - Noise strength (how much it varies)
    /// - `frequency` - Spatial frequency (higher = more detail)
    /// - `time_scale` - How fast noise evolves (0 = static)
    ///
    /// # Example: Flickering brightness
    ///
    /// ```ignore
    /// Rule::Noise {
    ///     field: "brightness".into(),
    ///     amplitude: 0.3,
    ///     frequency: 2.0,
    ///     time_scale: 5.0,
    /// }
    /// ```
    ///
    /// # Example: Organic position jitter
    ///
    /// ```ignore
    /// Rule::Noise {
    ///     field: "position.x".into(),
    ///     amplitude: 0.05,
    ///     frequency: 1.0,
    ///     time_scale: 2.0,
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// Uses the simulation's built-in `noise3` function. Noise is additive
    /// (adds to existing field value, doesn't replace it).
    Noise {
        /// Field to add noise to.
        field: String,
        /// Noise amplitude.
        amplitude: f32,
        /// Spatial frequency.
        frequency: f32,
        /// Time evolution speed (0 = static noise).
        time_scale: f32,
    },

    /// Remap a field from one range to another.
    ///
    /// Linear interpolation: maps `[in_min, in_max]` to `[out_min, out_max]`.
    /// Values outside input range are extrapolated (use with Clamp if needed).
    ///
    /// # Fields
    ///
    /// - `field` - Field to remap (modified in place)
    /// - `in_min`, `in_max` - Input range
    /// - `out_min`, `out_max` - Output range
    ///
    /// # Example: Age to opacity (young=visible, old=faded)
    ///
    /// ```ignore
    /// Rule::Remap {
    ///     field: "opacity".into(),
    ///     in_min: 0.0, in_max: 10.0,   // age range
    ///     out_min: 1.0, out_max: 0.0,  // fade out
    /// }
    /// ```
    ///
    /// # Example: Normalize velocity magnitude
    ///
    /// ```ignore
    /// Rule::Remap {
    ///     field: "speed_normalized".into(),
    ///     in_min: 0.0, in_max: 5.0,
    ///     out_min: 0.0, out_max: 1.0,
    /// }
    /// ```
    Remap {
        /// Field to remap.
        field: String,
        /// Input range minimum.
        in_min: f32,
        /// Input range maximum.
        in_max: f32,
        /// Output range minimum.
        out_min: f32,
        /// Output range maximum.
        out_max: f32,
    },

    /// Clamp a field to a range.
    ///
    /// Simple bounds constraint. Values below min become min,
    /// values above max become max.
    ///
    /// # Example: Limit energy
    ///
    /// ```ignore
    /// Rule::Clamp {
    ///     field: "energy".into(),
    ///     min: 0.0,
    ///     max: 100.0,
    /// }
    /// ```
    Clamp {
        /// Field to clamp.
        field: String,
        /// Minimum value.
        min: f32,
        /// Maximum value.
        max: f32,
    },

    /// Exponential smoothing toward a target value.
    ///
    /// Low-pass filter that smoothly moves a field toward a target.
    /// Higher rate = faster convergence. Classic for easing, damping,
    /// or filtering noisy values.
    ///
    /// # Fields
    ///
    /// - `field` - Field to smooth
    /// - `target` - Target value to approach
    /// - `rate` - Smoothing rate (0-1, higher = faster)
    ///
    /// # Example: Smooth brightness toward 0
    ///
    /// ```ignore
    /// Rule::Smooth {
    ///     field: "brightness".into(),
    ///     target: 0.0,
    ///     rate: 0.1,
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// Formula: `field = mix(field, target, rate * delta_time)`
    Smooth {
        /// Field to smooth.
        field: String,
        /// Target value.
        target: f32,
        /// Smoothing rate per second.
        rate: f32,
    },

    /// Quantize a field to discrete steps.
    ///
    /// Snaps continuous values to a grid. Useful for pixelated effects,
    /// discrete states, or grid-based movement.
    ///
    /// # Example: Snap position to grid
    ///
    /// ```ignore
    /// Rule::Quantize {
    ///     field: "position.x".into(),
    ///     step: 0.1,
    /// }
    /// ```
    ///
    /// # Example: Discrete energy levels
    ///
    /// ```ignore
    /// Rule::Quantize {
    ///     field: "energy".into(),
    ///     step: 10.0,  // 0, 10, 20, 30...
    /// }
    /// ```
    Quantize {
        /// Field to quantize.
        field: String,
        /// Step size (values snap to multiples of this).
        step: f32,
    },

    /// Wrap a field value within a range (modulo).
    ///
    /// Values that exceed max wrap back to min, and vice versa.
    /// Essential for cyclic quantities like angles, phases, or
    /// toroidal coordinates.
    ///
    /// # Example: Wrap phase to 0-2π
    ///
    /// ```ignore
    /// Rule::Modulo {
    ///     field: "phase".into(),
    ///     min: 0.0,
    ///     max: 6.28318,
    /// }
    /// ```
    ///
    /// # Example: Wrap hue for color cycling
    ///
    /// ```ignore
    /// Rule::Modulo {
    ///     field: "hue".into(),
    ///     min: 0.0,
    ///     max: 1.0,
    /// }
    /// ```
    Modulo {
        /// Field to wrap.
        field: String,
        /// Range minimum.
        min: f32,
        /// Range maximum.
        max: f32,
    },

    /// Copy one field to another.
    ///
    /// Optionally scale and offset the value during copy.
    /// Useful for derived values, backups, or transformations.
    ///
    /// # Fields
    ///
    /// - `from` - Source field
    /// - `to` - Destination field
    /// - `scale` - Multiply by this (default 1.0)
    /// - `offset` - Add this after scaling (default 0.0)
    ///
    /// # Example: Copy age to display value
    ///
    /// ```ignore
    /// Rule::Copy {
    ///     from: "age".into(),
    ///     to: "display_age".into(),
    ///     scale: 1.0,
    ///     offset: 0.0,
    /// }
    /// ```
    ///
    /// # Example: Invert and shift
    ///
    /// ```ignore
    /// Rule::Copy {
    ///     from: "health".into(),
    ///     to: "damage".into(),
    ///     scale: -1.0,
    ///     offset: 100.0,  // damage = 100 - health
    /// }
    /// ```
    Copy {
        /// Source field.
        from: String,
        /// Destination field.
        to: String,
        /// Scale factor.
        scale: f32,
        /// Offset added after scaling.
        offset: f32,
    },

    // =========================================================================
    // Logic Gates (analog/signal style)
    // =========================================================================

    /// Logical AND (analog: minimum of two fields).
    ///
    /// Output is the minimum of two input fields. In boolean terms,
    /// both must be "high" for output to be high.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::And {
    ///     a: "has_energy".into(),
    ///     b: "is_ready".into(),
    ///     output: "can_fire".into(),
    /// }
    /// ```
    And {
        /// First input field.
        a: String,
        /// Second input field.
        b: String,
        /// Output field.
        output: String,
    },

    /// Logical OR (analog: maximum of two fields).
    ///
    /// Output is the maximum of two input fields. In boolean terms,
    /// either being "high" makes output high.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Or {
    ///     a: "danger_left".into(),
    ///     b: "danger_right".into(),
    ///     output: "any_danger".into(),
    /// }
    /// ```
    Or {
        /// First input field.
        a: String,
        /// Second input field.
        b: String,
        /// Output field.
        output: String,
    },

    /// Logical NOT (inversion).
    ///
    /// Inverts a field value. Default is `1.0 - x` but can specify
    /// custom range for inversion.
    ///
    /// # Example: Simple invert (0↔1)
    ///
    /// ```ignore
    /// Rule::Not {
    ///     input: "alive".into(),
    ///     output: "dead".into(),
    ///     max: 1.0,
    /// }
    /// ```
    ///
    /// # Example: Invert in custom range
    ///
    /// ```ignore
    /// Rule::Not {
    ///     input: "brightness".into(),
    ///     output: "darkness".into(),
    ///     max: 100.0,  // darkness = 100 - brightness
    /// }
    /// ```
    Not {
        /// Input field.
        input: String,
        /// Output field.
        output: String,
        /// Maximum value (output = max - input).
        max: f32,
    },

    /// Logical XOR (analog: absolute difference).
    ///
    /// Output is `abs(a - b)`. High when inputs differ, low when same.
    /// Useful for detecting disagreement or change.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Xor {
    ///     a: "signal_a".into(),
    ///     b: "signal_b".into(),
    ///     output: "mismatch".into(),
    /// }
    /// ```
    Xor {
        /// First input field.
        a: String,
        /// Second input field.
        b: String,
        /// Output field.
        output: String,
    },

    // =========================================================================
    // Stateful Logic
    // =========================================================================

    /// Hysteresis (Schmitt trigger) - two-threshold switching.
    ///
    /// Prevents oscillation at boundaries by using separate thresholds
    /// for turning on vs off. Output goes high when input exceeds
    /// `high_threshold`, stays high until input drops below `low_threshold`.
    ///
    /// # Fields
    ///
    /// - `input` - Field to monitor
    /// - `output` - Field to set (acts as state memory too)
    /// - `low_threshold` - Turn off below this
    /// - `high_threshold` - Turn on above this
    /// - `on_value` - Value when "on" (default 1.0)
    /// - `off_value` - Value when "off" (default 0.0)
    ///
    /// # Example: Temperature control
    ///
    /// ```ignore
    /// // Heater turns on at 18°, stays on until 22°
    /// Rule::Hysteresis {
    ///     input: "temperature".into(),
    ///     output: "heater_on".into(),
    ///     low_threshold: 18.0,
    ///     high_threshold: 22.0,
    ///     on_value: 1.0,
    ///     off_value: 0.0,
    /// }
    /// ```
    Hysteresis {
        /// Input field to monitor.
        input: String,
        /// Output field (also stores state).
        output: String,
        /// Turn off when input drops below this.
        low_threshold: f32,
        /// Turn on when input rises above this.
        high_threshold: f32,
        /// Output value when "on".
        on_value: f32,
        /// Output value when "off".
        off_value: f32,
    },

    /// Set-Reset Latch (SR flip-flop).
    ///
    /// Persistent memory that sets on one condition, resets on another.
    /// Once set, stays set until explicitly reset.
    ///
    /// # Example: Alarm that latches on
    ///
    /// ```ignore
    /// Rule::Latch {
    ///     output: "alarm".into(),
    ///     set_condition: "p.danger > 0.9".into(),
    ///     reset_condition: "p.acknowledged > 0.5".into(),
    ///     set_value: 1.0,
    ///     reset_value: 0.0,
    /// }
    /// ```
    Latch {
        /// Output field (stores latched state).
        output: String,
        /// WGSL condition to set the latch.
        set_condition: String,
        /// WGSL condition to reset the latch.
        reset_condition: String,
        /// Value when set.
        set_value: f32,
        /// Value when reset.
        reset_value: f32,
    },

    /// Edge detector - fire on transition.
    ///
    /// Outputs a pulse (one frame) when input crosses a threshold.
    /// Requires a "previous value" field to track state.
    ///
    /// # Fields
    ///
    /// - `input` - Field to monitor
    /// - `prev_field` - Field storing previous frame's value
    /// - `output` - Field to pulse on edge
    /// - `threshold` - Crossing point
    /// - `rising` - Detect low→high transitions
    /// - `falling` - Detect high→low transitions
    ///
    /// # Example: Detect when energy crosses 50%
    ///
    /// ```ignore
    /// Rule::Edge {
    ///     input: "energy".into(),
    ///     prev_field: "energy_prev".into(),
    ///     output: "energy_crossed".into(),
    ///     threshold: 0.5,
    ///     rising: true,
    ///     falling: true,
    /// }
    /// ```
    Edge {
        /// Input field to monitor.
        input: String,
        /// Field storing previous value (you must initialize this).
        prev_field: String,
        /// Output pulse field.
        output: String,
        /// Threshold to detect crossing.
        threshold: f32,
        /// Detect rising edge (low to high).
        rising: bool,
        /// Detect falling edge (high to low).
        falling: bool,
    },

    // =========================================================================
    // Selectors
    // =========================================================================

    /// Conditional select (ternary operator).
    ///
    /// `output = condition ? then_value : else_value`
    ///
    /// # Example: Choose speed based on state
    ///
    /// ```ignore
    /// Rule::Select {
    ///     condition: "p.is_fleeing > 0.5".into(),
    ///     then_field: "fast_speed".into(),
    ///     else_field: "normal_speed".into(),
    ///     output: "current_speed".into(),
    /// }
    /// ```
    Select {
        /// WGSL boolean condition.
        condition: String,
        /// Field to use when true.
        then_field: String,
        /// Field to use when false.
        else_field: String,
        /// Output field.
        output: String,
    },

    /// Blend two fields by a weight field.
    ///
    /// `output = mix(a, b, weight)` where weight 0→a, weight 1→b.
    ///
    /// # Example: Blend colors by temperature
    ///
    /// ```ignore
    /// Rule::Blend {
    ///     a: "cold_color".into(),
    ///     b: "hot_color".into(),
    ///     weight: "temperature_normalized".into(),
    ///     output: "display_color".into(),
    /// }
    /// ```
    Blend {
        /// First input field (weight=0).
        a: String,
        /// Second input field (weight=1).
        b: String,
        /// Weight field (0-1).
        weight: String,
        /// Output field.
        output: String,
    },
}

impl Rule {
    /// Returns `true` if this rule requires spatial hashing.
    ///
    /// Neighbor-based rules (Collide, Separate, Cohere, Align, Convert,
    /// Chase, Evade, NeighborCustom) need spatial hashing enabled via
    /// `with_spatial_config()`.
    pub fn requires_neighbors(&self) -> bool {
        match self {
            Rule::Collide { .. }
            | Rule::OnCollision { .. }
            | Rule::NBodyGravity { .. }
            | Rule::Viscosity { .. }
            | Rule::Pressure { .. }
            | Rule::Magnetism { .. }
            | Rule::SurfaceTension { .. }
            | Rule::Avoid { .. }
            | Rule::Separate { .. }
            | Rule::Cohere { .. }
            | Rule::Align { .. }
            | Rule::Flock { .. }
            | Rule::Convert { .. }
            | Rule::Chase { .. }
            | Rule::Evade { .. }
            | Rule::Diffuse { .. }
            | Rule::NeighborCustom(_) => true,
            Rule::Typed { rule, .. } => rule.requires_neighbors(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses cohesion accumulators.
    pub(crate) fn needs_cohesion_accumulator(&self) -> bool {
        match self {
            Rule::Cohere { .. } | Rule::Flock { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_cohesion_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses alignment accumulators.
    pub(crate) fn needs_alignment_accumulator(&self) -> bool {
        match self {
            Rule::Align { .. } | Rule::Flock { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_alignment_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses chase accumulators.
    pub(crate) fn needs_chase_accumulator(&self) -> bool {
        match self {
            Rule::Chase { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_chase_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses evade accumulators.
    pub(crate) fn needs_evade_accumulator(&self) -> bool {
        match self {
            Rule::Evade { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_evade_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses viscosity accumulators.
    pub(crate) fn needs_viscosity_accumulator(&self) -> bool {
        match self {
            Rule::Viscosity { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_viscosity_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses pressure accumulators.
    pub(crate) fn needs_pressure_accumulator(&self) -> bool {
        match self {
            Rule::Pressure { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_pressure_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses surface tension accumulators.
    pub(crate) fn needs_surface_tension_accumulator(&self) -> bool {
        match self {
            Rule::SurfaceTension { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_surface_tension_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses avoid accumulators.
    pub(crate) fn needs_avoid_accumulator(&self) -> bool {
        match self {
            Rule::Avoid { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_avoid_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses diffuse accumulators.
    pub(crate) fn needs_diffuse_accumulator(&self) -> bool {
        match self {
            Rule::Diffuse { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_diffuse_accumulator(),
            _ => false,
        }
    }

    /// Generate WGSL code for non-neighbor rules.
    pub fn to_wgsl(&self, bounds: f32) -> String {
        match self {
            Rule::Gravity(g) => format!(
                "    // Gravity\n    p.velocity.y -= {} * uniforms.delta_time;",
                g
            ),

            Rule::BounceWalls => format!(
                r#"    // Bounce off walls
    if p.position.x < -{bounds} {{
        p.position.x = -{bounds};
        p.velocity.x = abs(p.velocity.x);
    }} else if p.position.x > {bounds} {{
        p.position.x = {bounds};
        p.velocity.x = -abs(p.velocity.x);
    }}
    if p.position.y < -{bounds} {{
        p.position.y = -{bounds};
        p.velocity.y = abs(p.velocity.y);
    }} else if p.position.y > {bounds} {{
        p.position.y = {bounds};
        p.velocity.y = -abs(p.velocity.y);
    }}
    if p.position.z < -{bounds} {{
        p.position.z = -{bounds};
        p.velocity.z = abs(p.velocity.z);
    }} else if p.position.z > {bounds} {{
        p.position.z = {bounds};
        p.velocity.z = -abs(p.velocity.z);
    }}"#,
                bounds = bounds
            ),

            Rule::WrapWalls => {
                let size = bounds * 2.0;
                format!(
                    r#"    // Wrap around walls (toroidal)
    if p.position.x < -{bounds} {{
        p.position.x += {size};
    }} else if p.position.x > {bounds} {{
        p.position.x -= {size};
    }}
    if p.position.y < -{bounds} {{
        p.position.y += {size};
    }} else if p.position.y > {bounds} {{
        p.position.y -= {size};
    }}
    if p.position.z < -{bounds} {{
        p.position.z += {size};
    }} else if p.position.z > {bounds} {{
        p.position.z -= {size};
    }}"#,
                    bounds = bounds,
                    size = size
                )
            }

            Rule::Drag(d) => format!(
                "    // Drag\n    p.velocity *= 1.0 - ({} * uniforms.delta_time);",
                d
            ),

            Rule::Acceleration(acc) => format!(
                "    // Acceleration\n    p.velocity += vec3<f32>({}, {}, {}) * uniforms.delta_time;",
                acc.x, acc.y, acc.z
            ),

            Rule::AttractTo { point, strength } => format!(
                r#"    // Attract to point
    {{
        let attract_dir = vec3<f32>({}, {}, {}) - p.position;
        let dist = length(attract_dir);
        if dist > 0.001 {{
            p.velocity += normalize(attract_dir) * {} * uniforms.delta_time;
        }}
    }}"#,
                point.x, point.y, point.z, strength
            ),

            Rule::RepelFrom { point, strength, radius } => format!(
                r#"    // Repel from point
    {{
        let repel_dir = p.position - vec3<f32>({}, {}, {});
        let dist = length(repel_dir);
        if dist < {} && dist > 0.001 {{
            let force = ({} - dist) / {} * {};
            p.velocity += normalize(repel_dir) * force * uniforms.delta_time;
        }}
    }}"#,
                point.x, point.y, point.z, radius, radius, radius, strength
            ),

            Rule::Vortex { center, axis, strength } => {
                // Normalize axis at compile time for the shader
                let axis_len = (axis.x * axis.x + axis.y * axis.y + axis.z * axis.z).sqrt();
                let (ax, ay, az) = if axis_len > 0.0001 {
                    (axis.x / axis_len, axis.y / axis_len, axis.z / axis_len)
                } else {
                    (0.0, 1.0, 0.0) // Default to Y axis
                };
                format!(
                    r#"    // Vortex
    {{
        let vortex_center = vec3<f32>({cx}, {cy}, {cz});
        let vortex_axis = vec3<f32>({ax}, {ay}, {az});
        let to_particle = p.position - vortex_center;
        // Project onto plane perpendicular to axis
        let along_axis = dot(to_particle, vortex_axis) * vortex_axis;
        let radial = to_particle - along_axis;
        let dist = length(radial);
        if dist > 0.001 {{
            // Tangent is perpendicular to both axis and radial
            let tangent = cross(vortex_axis, radial) / dist;
            p.velocity += tangent * {strength} * uniforms.delta_time;
        }}
    }}"#,
                    cx = center.x, cy = center.y, cz = center.z,
                    ax = ax, ay = ay, az = az,
                    strength = strength
                )
            }

            Rule::Turbulence { scale, strength } => format!(
                r#"    // Turbulence (noise-based force)
    {{
        let turb_pos = p.position * {scale} + uniforms.time * 0.5;
        let turb_force = vec3<f32>(
            noise3(turb_pos + vec3<f32>(0.0, 0.0, 0.0)),
            noise3(turb_pos + vec3<f32>(100.0, 0.0, 0.0)),
            noise3(turb_pos + vec3<f32>(0.0, 100.0, 0.0))
        );
        p.velocity += turb_force * {strength} * uniforms.delta_time;
    }}"#,
                scale = scale, strength = strength
            ),

            Rule::Orbit { center, strength } => format!(
                r#"    // Orbit
    {{
        let orbit_center = vec3<f32>({cx}, {cy}, {cz});
        let to_center = orbit_center - p.position;
        let dist = length(to_center);
        if dist > 0.001 {{
            // Centripetal force toward center
            let centripetal = normalize(to_center) * {strength};
            // Tangential direction (perpendicular in XZ plane)
            let tangent = vec3<f32>(-to_center.z, 0.0, to_center.x) / dist;
            // Adjust tangent velocity to maintain orbit
            let orbital_speed = sqrt({strength} * dist);
            let current_tangent_speed = dot(p.velocity, tangent);
            p.velocity += centripetal * uniforms.delta_time;
            p.velocity += tangent * (orbital_speed - current_tangent_speed) * 0.1 * uniforms.delta_time;
        }}
    }}"#,
                cx = center.x, cy = center.y, cz = center.z,
                strength = strength
            ),

            Rule::Curl { scale, strength } => format!(
                r#"    // Curl noise (divergence-free flow)
    {{
        let curl_pos = p.position * {scale};
        let eps = 0.01;
        // Compute curl via finite differences of noise field
        let dx = vec3<f32>(eps, 0.0, 0.0);
        let dy = vec3<f32>(0.0, eps, 0.0);
        let dz = vec3<f32>(0.0, 0.0, eps);
        // Sample noise at offset positions
        let n_py = noise3(curl_pos + dy + vec3<f32>(0.0, 0.0, 100.0));
        let n_my = noise3(curl_pos - dy + vec3<f32>(0.0, 0.0, 100.0));
        let n_pz = noise3(curl_pos + dz + vec3<f32>(0.0, 100.0, 0.0));
        let n_mz = noise3(curl_pos - dz + vec3<f32>(0.0, 100.0, 0.0));
        let n_px = noise3(curl_pos + dx + vec3<f32>(100.0, 0.0, 0.0));
        let n_mx = noise3(curl_pos - dx + vec3<f32>(100.0, 0.0, 0.0));
        // Curl = (dFz/dy - dFy/dz, dFx/dz - dFz/dx, dFy/dx - dFx/dy)
        let curl = vec3<f32>(
            (n_py - n_my) - (n_pz - n_mz),
            (n_pz - n_mz) - (n_px - n_mx),
            (n_px - n_mx) - (n_py - n_my)
        ) / (2.0 * eps);
        p.velocity += curl * {strength} * uniforms.delta_time;
    }}"#,
                scale = scale, strength = strength
            ),

            Rule::Wander { strength, frequency } => format!(
                r#"    // Wander (random movement)
    {{
        let wander_seed = index * 1103515245u + u32(uniforms.time * {frequency});
        let hx = (wander_seed ^ (wander_seed >> 15u)) * 0x45d9f3bu;
        let hy = ((wander_seed + 1u) ^ ((wander_seed + 1u) >> 15u)) * 0x45d9f3bu;
        let hz = ((wander_seed + 2u) ^ ((wander_seed + 2u) >> 15u)) * 0x45d9f3bu;
        let wander_force = vec3<f32>(
            f32(hx & 0xFFFFu) / 32768.0 - 1.0,
            f32(hy & 0xFFFFu) / 32768.0 - 1.0,
            f32(hz & 0xFFFFu) / 32768.0 - 1.0
        );
        p.velocity += wander_force * {strength} * uniforms.delta_time;
    }}"#
            ),

            Rule::SpeedLimit { min, max } => format!(
                r#"    // Speed limit
    {{
        let speed = length(p.velocity);
        if speed > 0.0001 {{
            let clamped_speed = clamp(speed, {min:.6}, {max:.6});
            p.velocity = normalize(p.velocity) * clamped_speed;
        }}
    }}"#
            ),

            Rule::Custom(code) => format!("    // Custom rule\n{}", code),

            Rule::Sync {
                phase_field,
                frequency,
                field,
                emit_amount,
                coupling,
                detection_threshold,
                on_fire,
            } => {
                let on_fire_code = on_fire.as_ref().map(|c| c.as_str()).unwrap_or("");
                format!(
                    r#"    // Oscillator synchronization (Kuramoto model)
    {{
        let tau = 6.28318;

        // Advance phase by frequency
        p.{phase_field} += {frequency} * uniforms.delta_time * tau;

        // Read field to detect nearby firing
        let detected = field_read({field}u, p.position);

        // If we detect activity, nudge our phase (coupling)
        if detected > {detection_threshold} {{
            // sin(phase/2) peaks at phase=π, meaning particles respond most
            // strongly when halfway through their cycle
            let phase_response = sin(p.{phase_field} * 0.5);
            p.{phase_field} += {coupling} * phase_response * detected * uniforms.delta_time * tau;
        }}

        // Check if it's time to fire
        if p.{phase_field} >= tau {{
            // Reset phase (keep remainder for stability)
            p.{phase_field} = p.{phase_field} - tau;

            // Emit to field
            field_write({field}u, p.position, {emit_amount});

            // Run custom on_fire callback
{on_fire_code}
        }}
    }}"#
                )
            }

            Rule::BondSprings { bonds, stiffness, damping, rest_length, max_stretch } => {
                let no_bond = u32::MAX;
                let max_stretch_code = if let Some(max_s) = max_stretch {
                    format!(
                        r#"
                let stretch_ratio = dist / {rest_length};
                if stretch_ratio > {max_s} {{
                    stretch = stretch + (stretch_ratio - {max_s}) * {rest_length} * 10.0;
                }}"#,
                        rest_length = rest_length,
                        max_s = max_s
                    )
                } else {
                    String::new()
                };

                let mut bond_code = String::new();
                for bond_field in bonds {
                    bond_code.push_str(&format!(
                        r#"
        // Bond: {bond_field}
        if p.{bond_field} != {no_bond}u {{
            let other = particles[p.{bond_field}];
            let delta = other.position - p.position;
            let dist = length(delta);
            if dist > 0.0001 {{
                let dir = delta / dist;
                var stretch = dist - {rest_length};{max_stretch_code}
                bond_force += dir * stretch * {stiffness};
                let rel_vel = dot(other.velocity - p.velocity, dir);
                bond_force += dir * rel_vel * {damping};
            }}
        }}
"#,
                        bond_field = bond_field,
                        no_bond = no_bond,
                        rest_length = rest_length,
                        stiffness = stiffness,
                        damping = damping,
                        max_stretch_code = max_stretch_code,
                    ));
                }

                format!(
                    r#"    // Bond springs
    {{
        var bond_force = vec3<f32>(0.0);
        let dt = uniforms.delta_time;
{bond_code}
        p.velocity += bond_force * dt;
    }}"#,
                    bond_code = bond_code
                )
            }

            Rule::ChainSprings { stiffness, damping, rest_length, max_stretch } => {
                let max_stretch_code = if let Some(max_s) = max_stretch {
                    format!(
                        r#"
                    let stretch_ratio = dist / {rest_length};
                    if stretch_ratio > {max_s} {{
                        stretch = stretch + (stretch_ratio - {max_s}) * {rest_length} * 10.0;
                    }}"#
                    )
                } else {
                    String::new()
                };

                format!(
                    r#"    // Chain springs (index-based)
    {{
        var chain_force = vec3<f32>(0.0);
        let dt = uniforms.delta_time;
        let num_particles = arrayLength(&particles);

        // Bond to previous particle (index - 1)
        if index > 0u {{
            let other = particles[index - 1u];
            let delta = other.position - p.position;
            let dist = length(delta);
            if dist > 0.0001 {{
                let dir = delta / dist;
                var stretch = dist - {rest_length};{max_stretch_code}
                chain_force += dir * stretch * {stiffness};
                let rel_vel = dot(other.velocity - p.velocity, dir);
                chain_force += dir * rel_vel * {damping};
            }}
        }}

        // Bond to next particle (index + 1)
        if index < num_particles - 1u {{
            let other = particles[index + 1u];
            let delta = other.position - p.position;
            let dist = length(delta);
            if dist > 0.0001 {{
                let dir = delta / dist;
                var stretch = dist - {rest_length};{max_stretch_code}
                chain_force += dir * stretch * {stiffness};
                let rel_vel = dot(other.velocity - p.velocity, dir);
                chain_force += dir * rel_vel * {damping};
            }}
        }}

        p.velocity += chain_force * dt;
    }}"#,
                    rest_length = rest_length,
                    stiffness = stiffness,
                    damping = damping,
                    max_stretch_code = max_stretch_code,
                )
            }

            Rule::RadialSprings { hub_stiffness, ring_stiffness, damping, hub_length, ring_length } => {
                format!(
                    r#"    // Radial springs (hub + ring)
    {{
        var radial_force = vec3<f32>(0.0);
        let dt = uniforms.delta_time;
        let num_particles = arrayLength(&particles);

        if index > 0u {{
            // Connect to center hub (particle 0)
            let hub = particles[0u];
            let delta = hub.position - p.position;
            let dist = length(delta);
            if dist > 0.0001 {{
                let dir = delta / dist;
                let stretch = dist - {hub_length};
                radial_force += dir * stretch * {hub_stiffness};
                let rel_vel = dot(hub.velocity - p.velocity, dir);
                radial_force += dir * rel_vel * {damping};
            }}

            // Connect to ring neighbors (wrapping)
            let ring_size = num_particles - 1u;
            let ring_idx = index - 1u;  // 0-based ring index

            // Previous in ring
            let prev_ring = (ring_idx + ring_size - 1u) % ring_size;
            let prev_idx = prev_ring + 1u;
            {{
                let other = particles[prev_idx];
                let delta = other.position - p.position;
                let dist = length(delta);
                if dist > 0.0001 {{
                    let dir = delta / dist;
                    let stretch = dist - {ring_length};
                    radial_force += dir * stretch * {ring_stiffness};
                    let rel_vel = dot(other.velocity - p.velocity, dir);
                    radial_force += dir * rel_vel * {damping};
                }}
            }}

            // Next in ring
            let next_ring = (ring_idx + 1u) % ring_size;
            let next_idx = next_ring + 1u;
            {{
                let other = particles[next_idx];
                let delta = other.position - p.position;
                let dist = length(delta);
                if dist > 0.0001 {{
                    let dir = delta / dist;
                    let stretch = dist - {ring_length};
                    radial_force += dir * stretch * {ring_stiffness};
                    let rel_vel = dot(other.velocity - p.velocity, dir);
                    radial_force += dir * rel_vel * {damping};
                }}
            }}
        }}

        p.velocity += radial_force * dt;
    }}"#,
                    hub_stiffness = hub_stiffness,
                    ring_stiffness = ring_stiffness,
                    damping = damping,
                    hub_length = hub_length,
                    ring_length = ring_length,
                )
            }

            Rule::Buoyancy { surface_y, density } => {
                format!(
                    r#"    // Buoyancy
    {{
        let depth = {surface_y} - p.position.y;
        if depth > 0.0 {{
            // Upward force proportional to depth and density
            let buoyancy_force = depth * {density} * 10.0;
            p.velocity.y += buoyancy_force * uniforms.delta_time;

            // Water resistance (drag when submerged)
            p.velocity *= 1.0 - (0.5 * uniforms.delta_time);
        }}
    }}"#,
                    surface_y = surface_y,
                    density = density,
                )
            }

            Rule::Friction { ground_y, strength, threshold } => {
                format!(
                    r#"    // Ground friction
    {{
        let height_above_ground = p.position.y - {ground_y};
        if height_above_ground < {threshold} {{
            // Friction increases as we get closer to ground
            let friction_factor = 1.0 - (height_above_ground / {threshold});
            let friction = {strength} * friction_factor;

            // Apply friction to horizontal velocity
            p.velocity.x *= 1.0 - (friction * uniforms.delta_time);
            p.velocity.z *= 1.0 - (friction * uniforms.delta_time);

            // Prevent sinking below ground
            if p.position.y < {ground_y} {{
                p.position.y = {ground_y};
                p.velocity.y = max(p.velocity.y, 0.0);
            }}
        }}
    }}"#,
                    ground_y = ground_y,
                    strength = strength,
                    threshold = threshold,
                )
            }

            Rule::Wind { direction, strength, turbulence } => {
                // Normalize direction
                let len = (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z).sqrt();
                let (dx, dy, dz) = if len > 0.0001 {
                    (direction.x / len, direction.y / len, direction.z / len)
                } else {
                    (1.0, 0.0, 0.0)
                };

                // Generate different code paths based on whether turbulence is enabled
                if *turbulence > 0.0 {
                    format!(
                        r#"    // Wind with turbulence
    {{
        let wind_dir = vec3<f32>({dx:.6}, {dy:.6}, {dz:.6});
        var wind_strength = {strength:.6};

        let turb_pos = p.position * 3.0 + uniforms.time * 2.0;
        let turb = noise3(turb_pos) * {turbulence:.6};
        wind_strength = wind_strength * (1.0 + turb);

        // Also vary direction slightly
        let turb_dir = vec3<f32>(
            noise3(turb_pos + vec3<f32>(100.0, 0.0, 0.0)),
            noise3(turb_pos + vec3<f32>(0.0, 100.0, 0.0)),
            noise3(turb_pos + vec3<f32>(0.0, 0.0, 100.0))
        ) * {turbulence:.6} * 0.5;
        p.velocity += (wind_dir + turb_dir) * wind_strength * uniforms.delta_time;
    }}"#,
                        dx = dx,
                        dy = dy,
                        dz = dz,
                        strength = strength,
                        turbulence = turbulence,
                    )
                } else {
                    format!(
                        r#"    // Wind (steady)
    {{
        let wind_dir = vec3<f32>({dx:.6}, {dy:.6}, {dz:.6});
        p.velocity += wind_dir * {strength:.6} * uniforms.delta_time;
    }}"#,
                        dx = dx,
                        dy = dy,
                        dz = dz,
                        strength = strength,
                    )
                }
            }

            Rule::Current { field, strength } => {
                format!(
                    r#"    // Current (follow field gradient)
    {{
        let gradient = field_{field}_gradient(p.position);
        p.velocity += gradient * {strength} * uniforms.delta_time;
    }}"#,
                    field = field,
                    strength = strength,
                )
            }

            Rule::RespawnBelow { threshold_y, spawn_y, reset_velocity } => {
                let velocity_reset = if *reset_velocity {
                    "p.velocity = vec3<f32>(0.0);"
                } else {
                    "p.velocity.y = 0.0;"  // Just stop falling
                };

                format!(
                    r#"    // Respawn below threshold
    if p.position.y < {threshold_y} {{
        // Randomize X and Z position on respawn
        let seed = index + u32(uniforms.time * 1000.0);
        let hash1 = (seed * 1103515245u + 12345u);
        let hash2 = (hash1 * 1103515245u + 12345u);
        let rx = f32(hash1 % 10000u) / 10000.0 * 2.0 - 1.0;
        let rz = f32(hash2 % 10000u) / 10000.0 * 2.0 - 1.0;

        p.position.x = rx * 0.8;  // Spread across bounds
        p.position.y = {spawn_y};
        p.position.z = rz * 0.8;
        {velocity_reset}
    }}"#,
                    threshold_y = threshold_y,
                    spawn_y = spawn_y,
                    velocity_reset = velocity_reset,
                )
            }

            Rule::Age => "    // Age\n    p.age += uniforms.delta_time;".to_string(),

            Rule::Lifetime(seconds) => format!(
                r#"    // Lifetime
    if p.age > {seconds} {{
        p.alive = 0u;
    }}"#
            ),

            Rule::FadeOut(duration) => format!(
                r#"    // Fade out
    {{
        let fade = clamp(1.0 - p.age / {duration}, 0.0, 1.0);
        p.color *= fade;
    }}"#
            ),

            Rule::ShrinkOut(duration) => format!(
                r#"    // Shrink out
    p.scale = clamp(1.0 - p.age / {duration}, 0.0, 1.0);"#
            ),

            Rule::ColorOverLife { start, end, duration } => format!(
                r#"    // Color over life
    {{
        let t = clamp(p.age / {duration}, 0.0, 1.0);
        p.color = mix(vec3<f32>({}, {}, {}), vec3<f32>({}, {}, {}), t);
    }}"#,
                start.x, start.y, start.z, end.x, end.y, end.z
            ),

            Rule::PointGravity { point, strength, softening } => format!(
                r#"    // Point gravity (inverse-square)
    {{
        let to_point = vec3<f32>({}, {}, {}) - p.position;
        let dist_sq = dot(to_point, to_point) + {softening} * {softening};
        let dist = sqrt(dist_sq);
        if dist > 0.001 {{
            let force = {strength} / dist_sq;
            p.velocity += (to_point / dist) * force * uniforms.delta_time;
        }}
    }}"#,
                point.x, point.y, point.z
            ),

            Rule::Spring { anchor, stiffness, damping } => format!(
                r#"    // Spring (Hooke's law)
    {{
        let anchor_pos = vec3<f32>({}, {}, {});
        let displacement = anchor_pos - p.position;
        let spring_force = displacement * {stiffness};
        let damping_force = -p.velocity * {damping};
        p.velocity += (spring_force + damping_force) * uniforms.delta_time;
    }}"#,
                anchor.x, anchor.y, anchor.z
            ),

            Rule::Radial { point, strength, radius, falloff } => {
                let softening = 0.01_f32;
                let radius_check = if *radius > 0.0 {
                    format!("dist < {radius} && ")
                } else {
                    String::new()
                };
                let falloff_expr = match falloff {
                    Falloff::Constant => "1.0".to_string(),
                    Falloff::Linear => format!("(1.0 - dist / {radius})"),
                    Falloff::Inverse => format!("(1.0 / (dist + {softening}))"),
                    Falloff::InverseSquare => format!("(1.0 / (dist * dist + {softening} * {softening}))"),
                    Falloff::Smooth => format!("(1.0 - smoothstep(0.0, {radius}, dist))"),
                };
                format!(
                    r#"    // Radial force (strength={strength}, falloff={falloff:?})
    {{
        let radial_center = vec3<f32>({px}, {py}, {pz});
        let to_particle = p.position - radial_center;
        let dist = length(to_particle);
        if {radius_check}dist > 0.001 {{
            let falloff = {falloff_expr};
            let dir = to_particle / dist;
            p.velocity += dir * {strength} * falloff * uniforms.delta_time;
        }}
    }}"#,
                    px = point.x, py = point.y, pz = point.z,
                    strength = strength,
                    falloff = falloff,
                    radius_check = radius_check,
                    falloff_expr = falloff_expr
                )
            }

            Rule::Shockwave { origin, speed, width, strength, repeat } => {
                let time_expr = if *repeat > 0.0 {
                    format!("(uniforms.time % {repeat})")
                } else {
                    "uniforms.time".to_string()
                };
                format!(
                    r#"    // Shockwave (speed={speed}, repeat={repeat})
    {{
        let wave_center = vec3<f32>({ox}, {oy}, {oz});
        let to_particle = p.position - wave_center;
        let dist = length(to_particle);
        let wave_time = {time_expr};
        let wave_radius = wave_time * {speed};
        let wave_dist = abs(dist - wave_radius);
        if wave_dist < {width} && dist > 0.001 {{
            // Smooth falloff within wavefront
            let wave_strength = (1.0 - wave_dist / {width}) * {strength};
            let dir = to_particle / dist;
            p.velocity += dir * wave_strength * uniforms.delta_time;
        }}
    }}"#,
                    ox = origin.x, oy = origin.y, oz = origin.z,
                    speed = speed, width = width, strength = strength, repeat = repeat,
                    time_expr = time_expr
                )
            }

            Rule::Pulse { point, strength, frequency, radius } => {
                let radius_check = if *radius > 0.0 {
                    format!("dist < {} && ", radius)
                } else {
                    String::new()
                };
                format!(
                    r#"    // Pulse (breathing force, freq={frequency}Hz)
    {{
        let pulse_center = vec3<f32>({px}, {py}, {pz});
        let to_particle = p.position - pulse_center;
        let dist = length(to_particle);
        if {radius_check}dist > 0.001 {{
            // Sine oscillation: positive = expand, negative = contract
            let pulse_factor = sin(uniforms.time * {frequency} * 6.283185);
            let dir = to_particle / dist;
            p.velocity += dir * {strength} * pulse_factor * uniforms.delta_time;
        }}
    }}"#,
                    px = point.x, py = point.y, pz = point.z,
                    strength = strength, frequency = frequency,
                    radius_check = radius_check
                )
            }

            Rule::Oscillate { axis, amplitude, frequency, spatial_scale } => {
                let axis_len = (axis.x * axis.x + axis.y * axis.y + axis.z * axis.z).sqrt();
                let (ax, ay, az) = if axis_len > 0.0001 {
                    (axis.x / axis_len, axis.y / axis_len, axis.z / axis_len)
                } else {
                    (0.0, 1.0, 0.0)
                };
                if *spatial_scale > 0.0 {
                    format!(
                        r#"    // Oscillate (traveling wave)
    {{
        let osc_axis = vec3<f32>({ax}, {ay}, {az});
        // Compute distance from oscillation axis for radial waves
        let along_axis = osc_axis * dot(p.position, osc_axis);
        let perpendicular = p.position - along_axis;
        let radial_dist = length(perpendicular);
        let phase = uniforms.time * {frequency} * 6.283185 - radial_dist * {spatial_scale};
        let wave = sin(phase) * {amplitude};
        p.velocity += osc_axis * wave * uniforms.delta_time;
    }}"#
                    )
                } else {
                    format!(
                        r#"    // Oscillate (uniform)
    {{
        let osc_axis = vec3<f32>({ax}, {ay}, {az});
        let phase = uniforms.time * {frequency} * 6.283185;
        let wave = sin(phase) * {amplitude};
        p.velocity += osc_axis * wave * uniforms.delta_time;
    }}"#
                    )
                }
            }

            Rule::PositionNoise { scale, strength, speed } => format!(
                r#"    // Position noise (jitter)
    {{
        let noise_pos = p.position * {scale} + uniforms.time * {speed};
        let jitter = vec3<f32>(
            noise3(noise_pos),
            noise3(noise_pos + vec3<f32>(31.7, 0.0, 0.0)),
            noise3(noise_pos + vec3<f32>(0.0, 47.3, 0.0))
        );
        p.position += jitter * {strength};
    }}"#
            ),

            Rule::ColorBySpeed { slow_color, fast_color, max_speed } => format!(
                r#"    // Color by speed
    {{
        let speed = length(p.velocity);
        let t = clamp(speed / {max_speed}, 0.0, 1.0);
        p.color = mix(vec3<f32>({}, {}, {}), vec3<f32>({}, {}, {}), t);
    }}"#,
                slow_color.x, slow_color.y, slow_color.z,
                fast_color.x, fast_color.y, fast_color.z
            ),

            Rule::ColorByAge { young_color, old_color, max_age } => format!(
                r#"    // Color by age
    {{
        let t = clamp(p.age / {max_age}, 0.0, 1.0);
        p.color = mix(vec3<f32>({}, {}, {}), vec3<f32>({}, {}, {}), t);
    }}"#,
                young_color.x, young_color.y, young_color.z,
                old_color.x, old_color.y, old_color.z
            ),

            Rule::ScaleBySpeed { min_scale, max_scale, max_speed } => format!(
                r#"    // Scale by speed
    {{
        let speed = length(p.velocity);
        let t = clamp(speed / {max_speed}, 0.0, 1.0);
        p.scale = mix({min_scale}, {max_scale}, t);
    }}"#
            ),

            Rule::Maybe { probability, action } => format!(
                r#"    // Maybe (probabilistic)
    {{
        // Hash-based random using particle index and time
        let hash_seed = f32(index) * 12.9898 + uniforms.time * 78.233;
        let rand = fract(sin(hash_seed) * 43758.5453);
        if rand < {probability} {{
{action}
        }}
    }}"#
            ),

            Rule::Trigger { condition, action } => format!(
                r#"    // Trigger (conditional)
    if {condition} {{
{action}
    }}"#
            ),

            Rule::Periodic { interval, phase_field, action } => {
                let phase_offset = match phase_field {
                    Some(field) => format!("p.{field}"),
                    None => "0.0".to_string(),
                };
                format!(
                    r#"    // Periodic (time-based)
    {{
        let phase_offset = {phase_offset};
        let adjusted_time = uniforms.time + phase_offset * {interval};
        let prev_time = adjusted_time - uniforms.delta_time;
        let current_cycle = floor(adjusted_time / {interval});
        let prev_cycle = floor(prev_time / {interval});
        if current_cycle != prev_cycle {{
{action}
        }}
    }}"#
                )
            }

            Rule::Gradient { field, strength, ascending } => {
                let sign = if *ascending { 1.0 } else { -1.0 };
                format!(
                    r#"    // Gradient (chemotaxis)
    {{
        let eps = 0.02;  // Sample offset
        let here = field_read({field}u, p.position);
        let dx = field_read({field}u, p.position + vec3<f32>(eps, 0.0, 0.0)) - field_read({field}u, p.position - vec3<f32>(eps, 0.0, 0.0));
        let dy = field_read({field}u, p.position + vec3<f32>(0.0, eps, 0.0)) - field_read({field}u, p.position - vec3<f32>(0.0, eps, 0.0));
        let dz = field_read({field}u, p.position + vec3<f32>(0.0, 0.0, eps)) - field_read({field}u, p.position - vec3<f32>(0.0, 0.0, eps));
        let grad = vec3<f32>(dx, dy, dz) / (2.0 * eps);
        let grad_len = length(grad);
        if grad_len > 0.0001 {{
            p.velocity += normalize(grad) * {strength} * {sign:.1} * uniforms.delta_time;
        }}
    }}"#
                )
            }

            Rule::Lerp { field, target, rate } => format!(
                r#"    // Lerp (smooth interpolation)
    p.{field} = mix(p.{field}, {target}, clamp({rate} * uniforms.delta_time, 0.0, 1.0));"#
            ),

            Rule::Die { condition, field } => format!(
                r#"    // Die (conditional death)
    if {condition} {{
        p.{field} = false;
    }}"#
            ),

            Rule::State { field, transitions } => {
                let mut code = String::from("    // State machine transitions\n");
                for (from, to, condition) in transitions {
                    code.push_str(&format!(
                        "    if p.{field} == {from}u && {condition} {{ p.{field} = {to}u; }}\n"
                    ));
                }
                code
            }

            Rule::Agent {
                state_field,
                prev_state_field,
                state_timer_field,
                states,
            } => {
                let mut code = String::from("    // Agent state machine\n    {\n");

                // Check if state changed (for entry/exit actions)
                code.push_str(&format!(
                    "        let state_changed = p.{state_field} != p.{prev_state_field};\n"
                ));

                // Handle state timer reset on state change
                if let Some(timer_field) = state_timer_field {
                    code.push_str(&format!(
                        "        if state_changed {{ p.{timer_field} = 0.0; }}\n"
                    ));
                }

                // Generate exit actions (run when leaving a state)
                let has_exit_actions = states.iter().any(|s| s.on_exit.is_some());
                if has_exit_actions {
                    code.push_str("\n        // Exit actions (previous state)\n");
                    code.push_str("        if state_changed {\n");
                    for state in states.iter() {
                        if let Some(exit_code) = &state.on_exit {
                            let state_id = state.id;
                            let name_comment = state
                                .name
                                .as_ref()
                                .map(|n| format!(" // {}", n))
                                .unwrap_or_default();
                            code.push_str(&format!(
                                "            if p.{prev_state_field} == {state_id}u {{{name_comment}\n"
                            ));
                            for line in exit_code.lines() {
                                code.push_str(&format!("                {}\n", line.trim()));
                            }
                            code.push_str("            }\n");
                        }
                    }
                    code.push_str("        }\n");
                }

                // Generate entry actions (run when entering a state)
                let has_entry_actions = states.iter().any(|s| s.on_enter.is_some());
                if has_entry_actions {
                    code.push_str("\n        // Entry actions (current state)\n");
                    code.push_str("        if state_changed {\n");
                    for state in states.iter() {
                        if let Some(enter_code) = &state.on_enter {
                            let state_id = state.id;
                            let name_comment = state
                                .name
                                .as_ref()
                                .map(|n| format!(" // {}", n))
                                .unwrap_or_default();
                            code.push_str(&format!(
                                "            if p.{state_field} == {state_id}u {{{name_comment}\n"
                            ));
                            for line in enter_code.lines() {
                                code.push_str(&format!("                {}\n", line.trim()));
                            }
                            code.push_str("            }\n");
                        }
                    }
                    code.push_str("        }\n");
                }

                // Update prev_state AFTER entry/exit actions (so transitions can trigger them next frame)
                code.push_str(&format!(
                    "\n        // Mark state change as processed\n        p.{prev_state_field} = p.{state_field};\n"
                ));

                // Generate update actions (run every frame for current state)
                let has_update_actions = states.iter().any(|s| s.on_update.is_some());
                if has_update_actions {
                    code.push_str("\n        // Update actions (current state)\n");
                    for state in states.iter() {
                        if let Some(update_code) = &state.on_update {
                            let state_id = state.id;
                            let name_comment = state
                                .name
                                .as_ref()
                                .map(|n| format!(" // {}", n))
                                .unwrap_or_default();
                            code.push_str(&format!(
                                "        if p.{state_field} == {state_id}u {{{name_comment}\n"
                            ));
                            for line in update_code.lines() {
                                code.push_str(&format!("            {}\n", line.trim()));
                            }
                            code.push_str("        }\n");
                        }
                    }
                }

                // Increment state timer
                if let Some(timer_field) = state_timer_field {
                    code.push_str(&format!(
                        "\n        // Increment state timer\n        p.{timer_field} += uniforms.delta_time;\n"
                    ));
                }

                // Generate transitions (sorted by priority, highest first)
                code.push_str("\n        // State transitions\n");
                for state in states.iter() {
                    if state.transitions.is_empty() {
                        continue;
                    }

                    let state_id = state.id;
                    let name_comment = state
                        .name
                        .as_ref()
                        .map(|n| format!(" // from {}", n))
                        .unwrap_or_default();

                    // Sort transitions by priority (highest first)
                    let mut sorted_transitions = state.transitions.clone();
                    sorted_transitions.sort_by(|a, b| b.priority.cmp(&a.priority));

                    code.push_str(&format!(
                        "        if p.{state_field} == {state_id}u {{{name_comment}\n"
                    ));

                    for (i, transition) in sorted_transitions.iter().enumerate() {
                        let condition = &transition.condition;
                        let to_state = transition.to;
                        if i == 0 {
                            code.push_str(&format!(
                                "            if {condition} {{\n                p.{state_field} = {to_state}u;\n            }}"
                            ));
                        } else {
                            code.push_str(&format!(
                                " else if {condition} {{\n                p.{state_field} = {to_state}u;\n            }}"
                            ));
                        }
                    }

                    // Close the if-else chain with newline
                    if !sorted_transitions.is_empty() {
                        code.push_str("\n");
                    }
                    code.push_str("        }\n");
                }

                code.push_str("    }\n");
                code
            }

            Rule::Grow { rate, min, max } => format!(
                r#"    // Grow (scale over time)
    p.scale = clamp(p.scale + {rate} * uniforms.delta_time, {min}, {max});"#
            ),

            Rule::Decay { field, rate } => format!(
                r#"    // Decay (multiplicative)
    p.{field} *= pow({rate}, uniforms.delta_time);"#
            ),

            Rule::DensityBuoyancy { density_field, medium_density, strength } => format!(
                r#"    // Density-based buoyancy
    {{
        let buoyancy_force = ({medium_density} - p.{density_field}) * {strength};
        p.velocity.y += buoyancy_force * uniforms.delta_time;
    }}"#
            ),

            Rule::Mass { field } => format!(
                r#"    // Mass scaling (F=ma → a=F/m)
    {{
        let inv_mass = 1.0 / max(p.{field}, 0.001);
        // Scale this frame's velocity change by inverse mass
        // Note: this assumes velocity changes since last frame are forces
        p.velocity *= inv_mass;
    }}"#
            ),

            Rule::Tween { field, from, to, duration, timer_field } => format!(
                r#"    // Tween animation
    {{
        let t = clamp(p.{timer_field} / {duration}, 0.0, 1.0);
        p.{field} = mix({from}, {to}, t);
    }}"#
            ),

            Rule::Threshold { input_field, output_field, threshold, above, below } => format!(
                r#"    // Threshold (step function)
    if p.{input_field} >= {threshold} {{
        p.{output_field} = {above};
    }} else {{
        p.{output_field} = {below};
    }}"#
            ),

            Rule::Gate { condition, action } => format!(
                r#"    // Gate (conditional action)
    if {condition} {{
        {action}
    }}"#
            ),

            Rule::Noise { field, amplitude, frequency, time_scale } => format!(
                r#"    // Procedural noise
    {{
        let noise_pos = p.position * {frequency} + vec3<f32>(uniforms.time * {time_scale});
        p.{field} += noise3(noise_pos) * {amplitude};
    }}"#
            ),

            Rule::Remap { field, in_min, in_max, out_min, out_max } => format!(
                r#"    // Remap range
    {{
        let t = (p.{field} - {in_min:.6}) / ({in_max:.6} - {in_min:.6});
        p.{field} = {out_min:.6} + t * ({out_max:.6} - {out_min:.6});
    }}"#
            ),

            Rule::Clamp { field, min, max } => format!(
                "    // Clamp\n    p.{field} = clamp(p.{field}, {min:.6}, {max:.6});"
            ),

            Rule::Smooth { field, target, rate } => format!(
                "    // Smooth toward target\n    p.{field} = mix(p.{field}, {target:.6}, {rate:.6} * uniforms.delta_time);"
            ),

            Rule::Quantize { field, step } => format!(
                "    // Quantize to steps\n    p.{field} = floor(p.{field} / {step:.6}) * {step:.6};"
            ),

            Rule::Modulo { field, min, max } => format!(
                r#"    // Modulo wrap
    {{
        let range = {max:.6} - {min:.6};
        p.{field} = {min:.6} + (((p.{field} - {min:.6}) % range) + range) % range;
    }}"#
            ),

            Rule::Copy { from, to, scale, offset } => format!(
                "    // Copy field\n    p.{to} = p.{from} * {scale:.6} + {offset:.6};"
            ),

            // Logic gates
            Rule::And { a, b, output } => format!(
                "    // AND (min)\n    p.{output} = min(p.{a}, p.{b});"
            ),

            Rule::Or { a, b, output } => format!(
                "    // OR (max)\n    p.{output} = max(p.{a}, p.{b});"
            ),

            Rule::Not { input, output, max } => format!(
                "    // NOT (invert)\n    p.{output} = {max:.6} - p.{input};"
            ),

            Rule::Xor { a, b, output } => format!(
                "    // XOR (abs difference)\n    p.{output} = abs(p.{a} - p.{b});"
            ),

            Rule::Hysteresis { input, output, low_threshold, high_threshold, on_value, off_value } => format!(
                r#"    // Hysteresis (Schmitt trigger)
    if p.{input} > {high_threshold:.6} {{
        p.{output} = {on_value:.6};
    }} else if p.{input} < {low_threshold:.6} {{
        p.{output} = {off_value:.6};
    }}"#
            ),

            Rule::Latch { output, set_condition, reset_condition, set_value, reset_value } => format!(
                r#"    // Latch (SR flip-flop)
    if {reset_condition} {{
        p.{output} = {reset_value:.6};
    }} else if {set_condition} {{
        p.{output} = {set_value:.6};
    }}"#
            ),

            Rule::Edge { input, prev_field, output, threshold, rising, falling } => {
                let rising_check = if *rising {
                    format!("(p.{prev_field} < {threshold:.6} && p.{input} >= {threshold:.6})")
                } else {
                    "false".to_string()
                };
                let falling_check = if *falling {
                    format!("(p.{prev_field} >= {threshold:.6} && p.{input} < {threshold:.6})")
                } else {
                    "false".to_string()
                };
                format!(
                    r#"    // Edge detector
    if {rising_check} || {falling_check} {{
        p.{output} = 1.0;
    }} else {{
        p.{output} = 0.0;
    }}
    p.{prev_field} = p.{input};"#
                )
            }

            Rule::Select { condition, then_field, else_field, output } => format!(
                r#"    // Select (ternary)
    if {condition} {{
        p.{output} = p.{then_field};
    }} else {{
        p.{output} = p.{else_field};
    }}"#
            ),

            Rule::Blend { a, b, weight, output } => format!(
                "    // Blend by weight\n    p.{output} = mix(p.{a}, p.{b}, p.{weight});"
            ),

            // Neighbor rules generate code through to_neighbor_wgsl
            Rule::Collide { .. }
            | Rule::OnCollision { .. }
            | Rule::NBodyGravity { .. }
            | Rule::Viscosity { .. }
            | Rule::Pressure { .. }
            | Rule::Magnetism { .. }
            | Rule::SurfaceTension { .. }
            | Rule::Avoid { .. }
            | Rule::Separate { .. }
            | Rule::Cohere { .. }
            | Rule::Align { .. }
            | Rule::Flock { .. }
            | Rule::Typed { .. }
            | Rule::Convert { .. }
            | Rule::Chase { .. }
            | Rule::Evade { .. }
            | Rule::Diffuse { .. }
            | Rule::NeighborCustom(_) => String::new(),
        }
    }

    /// Generate WGSL code for neighbor-based rules (inside neighbor loop).
    pub fn to_neighbor_wgsl(&self) -> String {
        match self {
            Rule::Collide { radius, restitution } => format!(
                r#"            // Elastic collision
            if neighbor_dist < {radius} && neighbor_dist > 0.0001 {{
                // Relative velocity along collision normal (positive = approaching)
                let rel_vel = dot(neighbor_vel - p.velocity, neighbor_dir);

                // Only respond if particles are approaching
                if rel_vel > 0.0 {{
                    // Impulse for elastic collision (assumes equal masses)
                    // For two equal masses: each gets half the momentum exchange
                    let impulse = (1.0 + {restitution}) * rel_vel * 0.5;
                    p.velocity += neighbor_dir * impulse;
                }}

                // Position correction to resolve overlap
                let overlap = {radius} - neighbor_dist;
                p.velocity += neighbor_dir * overlap * 2.0;
            }}"#
            ),

            Rule::OnCollision { radius, response } => format!(
                r#"            // Custom collision response
            if neighbor_dist < {radius} && neighbor_dist > 0.0001 {{
                // Precompute useful collision variables
                let overlap = {radius} - neighbor_dist;
                let rel_vel = dot(neighbor_vel - p.velocity, neighbor_dir);

                // User-defined collision response
{response}
            }}"#
            ),

            Rule::NBodyGravity { strength, softening, radius } => format!(
                r#"            // N-body gravity
            if neighbor_dist < {radius} {{
                let dist_sq = neighbor_dist * neighbor_dist + {softening} * {softening};
                let force = {strength} / dist_sq;
                // Attract toward neighbor (opposite of neighbor_dir)
                p.velocity -= neighbor_dir * force * uniforms.delta_time;
            }}"#
            ),

            Rule::Viscosity { radius, strength: _ } => format!(
                r#"            // Viscosity (accumulate for averaging)
            if neighbor_dist < {radius} {{
                let weight = 1.0 - neighbor_dist / {radius};
                viscosity_sum += neighbor_vel * weight;
                viscosity_weight += weight;
            }}"#
            ),

            Rule::Pressure { radius, strength, target_density: _ } => format!(
                r#"            // Pressure (accumulate density and force)
            if neighbor_dist < {radius} && neighbor_dist > 0.001 {{
                let weight = 1.0 - neighbor_dist / {radius};
                pressure_density += weight;
                // Accumulate weighted push direction
                pressure_force += neighbor_dir * weight * {strength};
            }}"#
            ),

            Rule::Magnetism { radius, strength, same_repel } => {
                let same_sign = if *same_repel { "1.0" } else { "-1.0" };
                format!(
                    r#"            // Magnetism
            if neighbor_dist < {radius} && neighbor_dist > 0.001 {{
                let same_type = select(-1.0, 1.0, p.particle_type == other.particle_type);
                let force_dir = same_type * {same_sign}; // +1 = repel, -1 = attract
                let falloff = 1.0 - neighbor_dist / {radius};
                p.velocity += neighbor_dir * force_dir * falloff * {strength} * uniforms.delta_time;
            }}"#
                )
            }

            Rule::SurfaceTension { radius, .. } => format!(
                r#"            // Surface tension (accumulate neighbor info)
            if neighbor_dist < {radius} {{
                surface_neighbor_count += 1.0;
                surface_center_sum += neighbor_pos;
            }}"#
            ),

            Rule::Avoid { radius, .. } => format!(
                r#"            // Avoid (accumulate weighted avoidance)
            if neighbor_dist < {radius} && neighbor_dist > 0.001 {{
                let weight = 1.0 - neighbor_dist / {radius};
                avoid_sum += neighbor_dir * weight;
                avoid_count += 1.0;
            }}"#
            ),

            Rule::Separate { radius, strength } => format!(
                r#"            // Separation
            if neighbor_dist < {radius} && neighbor_dist > 0.0001 {{
                let force = ({radius} - neighbor_dist) / {radius};
                p.velocity += neighbor_dir * force * {strength} * uniforms.delta_time;
            }}"#
            ),

            Rule::Cohere { radius, strength: _ } => format!(
                r#"            // Cohesion (accumulate for averaging)
            if neighbor_dist < {radius} {{
                cohesion_sum += neighbor_pos;
                cohesion_count += 1.0;
            }}"#
            ),

            Rule::Align { radius, strength: _ } => format!(
                r#"            // Alignment (accumulate for averaging)
            if neighbor_dist < {radius} {{
                alignment_sum += neighbor_vel;
                alignment_count += 1.0;
            }}"#
            ),

            Rule::Flock { radius, separation, .. } => format!(
                r#"            // Flock: separation + cohesion + alignment accumulation
            if neighbor_dist < {radius} {{
                // Separation (immediate)
                if neighbor_dist > 0.0001 {{
                    let sep_force = ({radius} - neighbor_dist) / {radius};
                    p.velocity += neighbor_dir * sep_force * {separation} * uniforms.delta_time;
                }}
                // Cohesion accumulation
                cohesion_sum += neighbor_pos;
                cohesion_count += 1.0;
                // Alignment accumulation
                alignment_sum += neighbor_vel;
                alignment_count += 1.0;
            }}"#
            ),

            Rule::Typed { self_type, other_type, rule } => {
                let inner = rule.to_neighbor_wgsl();
                if inner.is_empty() {
                    return String::new();
                }
                let other_check = match other_type {
                    Some(t) => format!(" && other.particle_type == {}u", t),
                    None => String::new(),
                };
                format!(
                    r#"            // Typed rule (self={}, other={:?})
            if p.particle_type == {}u{} {{
{}
            }}"#,
                    self_type, other_type, self_type, other_check, inner
                )
            }

            Rule::Convert { from_type, trigger_type, to_type, radius, probability } => {
                format!(
                    r#"            // Convert type {} -> {} (triggered by {})
            if p.particle_type == {from_type}u && other.particle_type == {trigger_type}u && neighbor_dist < {radius} {{
                // Hash-based random using particle indices and time
                let hash_input = index ^ (other_idx * 1103515245u) ^ u32(uniforms.time * 1000.0);
                let hash = (hash_input ^ (hash_input >> 16u)) * 0x45d9f3bu;
                let rand = f32(hash & 0xFFFFu) / 65535.0;
                if rand < {probability} {{
                    p.particle_type = {to_type}u;
                }}
            }}"#,
                    from_type, to_type, trigger_type
                )
            }

            Rule::Chase { self_type, target_type, radius, .. } => format!(
                r#"            // Chase: track nearest target
            if p.particle_type == {self_type}u && other.particle_type == {target_type}u && neighbor_dist < {radius} {{
                if neighbor_dist < chase_nearest_dist {{
                    chase_nearest_dist = neighbor_dist;
                    chase_nearest_pos = neighbor_pos;
                }}
            }}"#
            ),

            Rule::Evade { self_type, threat_type, radius, .. } => format!(
                r#"            // Evade: track nearest threat
            if p.particle_type == {self_type}u && other.particle_type == {threat_type}u && neighbor_dist < {radius} {{
                if neighbor_dist < evade_nearest_dist {{
                    evade_nearest_dist = neighbor_dist;
                    evade_nearest_pos = neighbor_pos;
                }}
            }}"#
            ),

            Rule::NeighborCustom(code) => format!(
                "            // Custom neighbor rule\n{}",
                code
            ),

            Rule::Diffuse { field, radius, .. } => format!(
                r#"            // Diffuse: accumulate neighbor values
            if neighbor_dist < {radius} {{
                diffuse_sum += other.{field};
                diffuse_count += 1.0;
            }}"#
            ),

            _ => String::new(),
        }
    }

    /// Generate post-neighbor-loop WGSL (for averaging rules).
    pub fn to_post_neighbor_wgsl(&self) -> String {
        match self {
            Rule::Cohere { strength, .. } => format!(
                r#"    // Apply cohesion
    if cohesion_count > 0.0 {{
        let center = cohesion_sum / cohesion_count;
        let to_center = center - p.position;
        p.velocity += normalize(to_center) * {strength} * uniforms.delta_time;
    }}"#
            ),

            Rule::Align { strength, .. } => format!(
                r#"    // Apply alignment
    if alignment_count > 0.0 {{
        let avg_vel = alignment_sum / alignment_count;
        p.velocity += (avg_vel - p.velocity) * {strength} * uniforms.delta_time;
    }}"#
            ),

            Rule::Flock { cohesion, alignment, .. } => format!(
                r#"    // Apply flock cohesion and alignment
    if cohesion_count > 0.0 {{
        // Cohesion: steer toward center
        let center = cohesion_sum / cohesion_count;
        let to_center = center - p.position;
        let center_dist = length(to_center);
        if center_dist > 0.001 {{
            p.velocity += normalize(to_center) * {cohesion} * uniforms.delta_time;
        }}
    }}
    if alignment_count > 0.0 {{
        // Alignment: match average velocity
        let avg_vel = alignment_sum / alignment_count;
        p.velocity += (avg_vel - p.velocity) * {alignment} * uniforms.delta_time;
    }}"#
            ),

            Rule::Typed { self_type, rule, .. } => {
                let inner = rule.to_post_neighbor_wgsl();
                if inner.is_empty() {
                    return String::new();
                }
                format!(
                    r#"    // Typed post-neighbor (self={})
    if p.particle_type == {}u {{
{}
    }}"#,
                    self_type, self_type, inner
                )
            }

            Rule::Chase { self_type, strength, .. } => format!(
                r#"    // Apply chase steering
    if p.particle_type == {self_type}u && chase_nearest_dist < 1000.0 {{
        let to_target = chase_nearest_pos - p.position;
        let dist = length(to_target);
        if dist > 0.001 {{
            p.velocity += normalize(to_target) * {strength} * uniforms.delta_time;
        }}
    }}"#
            ),

            Rule::Evade { self_type, strength, .. } => format!(
                r#"    // Apply evade steering
    if p.particle_type == {self_type}u && evade_nearest_dist < 1000.0 {{
        let away_from_threat = p.position - evade_nearest_pos;
        let dist = length(away_from_threat);
        if dist > 0.001 {{
            p.velocity += normalize(away_from_threat) * {strength} * uniforms.delta_time;
        }}
    }}"#
            ),

            Rule::Viscosity { strength, .. } => format!(
                r#"    // Apply viscosity
    if viscosity_weight > 0.0 {{
        let avg_vel = viscosity_sum / viscosity_weight;
        p.velocity = mix(p.velocity, avg_vel, {strength} * uniforms.delta_time);
    }}"#
            ),

            Rule::Pressure { target_density, .. } => format!(
                r#"    // Apply pressure
    if pressure_density > {target_density} {{
        let excess = pressure_density - {target_density};
        p.velocity += pressure_force * excess * uniforms.delta_time;
    }}"#
            ),

            Rule::SurfaceTension { strength, threshold, .. } => format!(
                r#"    // Apply surface tension
    if surface_neighbor_count > 0.0 && surface_neighbor_count < {threshold} {{
        let center = surface_center_sum / surface_neighbor_count;
        let to_center = center - p.position;
        let dist = length(to_center);
        if dist > 0.001 {{
            // Pull toward center of neighbors (surface tension effect)
            let tension = ({threshold} - surface_neighbor_count) / {threshold};
            p.velocity += normalize(to_center) * tension * {strength} * uniforms.delta_time;
        }}
    }}"#
            ),

            Rule::Avoid { strength, .. } => format!(
                r#"    // Apply avoidance steering
    if avoid_count > 0.0 {{
        let avg_avoid = avoid_sum / avoid_count;
        let avoid_len = length(avg_avoid);
        if avoid_len > 0.001 {{
            // Steer away smoothly
            p.velocity += normalize(avg_avoid) * {strength} * uniforms.delta_time;
        }}
    }}"#
            ),

            Rule::Diffuse { field, rate, .. } => format!(
                r#"    // Apply diffusion (blend toward neighbor average)
    if diffuse_count > 0.0 {{
        let avg_value = diffuse_sum / diffuse_count;
        p.{field} = mix(p.{field}, avg_value, {rate} * uniforms.delta_time);
    }}"#
            ),

            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wraps rule WGSL code in a minimal valid compute shader for validation.
    fn wrap_in_shader(rule_code: &str) -> String {
        format!(
            r#"
struct Particle {{
    position: vec3<f32>,
    velocity: vec3<f32>,
    color: vec3<f32>,
    particle_type: u32,
    alive: u32,
    age: f32,
    lifetime: f32,
    size: f32,
    _pad: f32,
}};

struct Uniforms {{
    delta_time: f32,
    time: f32,
    bounds: f32,
}};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

// Minimal noise function for rules that use it
fn noise3(p: vec3<f32>) -> f32 {{
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 45.164))) * 43758.5453);
}}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let index = global_id.x;
    var p = particles[index];

{rule_code}

    particles[index] = p;
}}
"#,
            rule_code = rule_code
        )
    }

    /// Validates WGSL code using naga.
    fn validate_wgsl(code: &str) -> Result<(), String> {
        let module = naga::front::wgsl::parse_str(code)
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

    // ========== Basic Physics Rules ==========

    #[test]
    fn test_gravity_wgsl() {
        let rule = Rule::Gravity(9.8);
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Gravity"));
        assert!(wgsl.contains("velocity.y"));
        assert!(wgsl.contains("9.8"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Gravity WGSL should be valid");
    }

    #[test]
    fn test_drag_wgsl() {
        let rule = Rule::Drag(1.5);
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Drag"));
        assert!(wgsl.contains("1.5"));
        assert!(wgsl.contains("velocity *="));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Drag WGSL should be valid");
    }

    #[test]
    fn test_acceleration_wgsl() {
        let rule = Rule::Acceleration(Vec3::new(1.0, 2.0, 3.0));
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Acceleration"));
        assert!(wgsl.contains("1"));
        assert!(wgsl.contains("2"));
        assert!(wgsl.contains("3"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Acceleration WGSL should be valid");
    }

    #[test]
    fn test_speed_limit_wgsl() {
        let rule = Rule::SpeedLimit { min: 0.5, max: 5.0 };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Speed limit"));
        assert!(wgsl.contains("0.5"));
        assert!(wgsl.contains("5"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("SpeedLimit WGSL should be valid");
    }

    #[test]
    fn test_wander_wgsl() {
        let rule = Rule::Wander {
            strength: 2.0,
            frequency: 10.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Wander"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Wander WGSL should be valid");
    }

    // ========== Boundary Rules ==========

    #[test]
    fn test_bounce_walls_wgsl() {
        let rule = Rule::BounceWalls;
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Bounce"));
        assert!(wgsl.contains("position.x"));
        assert!(wgsl.contains("position.y"));
        assert!(wgsl.contains("position.z"));
        assert!(wgsl.contains("abs(p.velocity"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("BounceWalls WGSL should be valid");
    }

    #[test]
    fn test_wrap_walls_wgsl() {
        let rule = Rule::WrapWalls;
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Wrap"));
        assert!(wgsl.contains("toroidal"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("WrapWalls WGSL should be valid");
    }

    // ========== Point Force Rules ==========

    #[test]
    fn test_attract_to_wgsl() {
        let rule = Rule::AttractTo {
            point: Vec3::new(0.0, 0.0, 0.0),
            strength: 5.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Attract"));
        assert!(wgsl.contains("normalize"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("AttractTo WGSL should be valid");
    }

    #[test]
    fn test_repel_from_wgsl() {
        let rule = Rule::RepelFrom {
            point: Vec3::new(0.0, 0.0, 0.0),
            strength: 5.0,
            radius: 1.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Repel"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("RepelFrom WGSL should be valid");
    }

    #[test]
    fn test_point_gravity_wgsl() {
        let rule = Rule::PointGravity {
            point: Vec3::ZERO,
            strength: 10.0,
            softening: 0.05,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Point gravity"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("PointGravity WGSL should be valid");
    }

    #[test]
    fn test_spring_wgsl() {
        let rule = Rule::Spring {
            anchor: Vec3::new(0.0, 1.0, 0.0),
            stiffness: 10.0,
            damping: 0.5,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Spring"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Spring WGSL should be valid");
    }

    // ========== Field Effect Rules ==========

    #[test]
    fn test_vortex_wgsl() {
        let rule = Rule::Vortex {
            center: Vec3::ZERO,
            axis: Vec3::Y,
            strength: 5.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Vortex"));
        assert!(wgsl.contains("cross"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Vortex WGSL should be valid");
    }

    #[test]
    fn test_turbulence_wgsl() {
        let rule = Rule::Turbulence {
            scale: 2.0,
            strength: 1.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Turbulence"));
        assert!(wgsl.contains("noise3"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Turbulence WGSL should be valid");
    }

    #[test]
    fn test_orbit_wgsl() {
        let rule = Rule::Orbit {
            center: Vec3::ZERO,
            strength: 5.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Orbit"));
        assert!(wgsl.contains("centripetal"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Orbit WGSL should be valid");
    }

    #[test]
    fn test_curl_wgsl() {
        let rule = Rule::Curl {
            scale: 3.0,
            strength: 1.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Curl"));
        assert!(wgsl.contains("divergence-free"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Curl WGSL should be valid");
    }

    // ========== Wave/Modulation Rules ==========

    #[test]
    fn test_oscillate_wgsl() {
        let rule = Rule::Oscillate {
            axis: Vec3::Y,
            amplitude: 0.5,
            frequency: 2.0,
            spatial_scale: 0.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Oscillate"));
        assert!(wgsl.contains("sin"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Oscillate WGSL should be valid");
    }

    #[test]
    fn test_position_noise_wgsl() {
        let rule = Rule::PositionNoise {
            scale: 5.0,
            strength: 0.1,
            speed: 2.0,
        };
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Position noise"));
        assert!(wgsl.contains("noise3"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("PositionNoise WGSL should be valid");
    }

    // ========== Lifecycle Rules ==========

    #[test]
    fn test_age_wgsl() {
        let rule = Rule::Age;
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("Age"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Age WGSL should be valid");
    }

    // ========== Custom Rules ==========

    #[test]
    fn test_custom_wgsl() {
        let rule = Rule::Custom("p.velocity.x += 1.0;".into());
        let wgsl = rule.to_wgsl(1.0);

        assert!(wgsl.contains("velocity.x += 1.0"));

        let shader = wrap_in_shader(&wgsl);
        validate_wgsl(&shader).expect("Custom WGSL should be valid");
    }

    // ========== Falloff ==========

    #[test]
    fn test_falloff_expressions() {
        // Test all falloff types generate valid WGSL expressions
        let falloffs = [
            Falloff::Constant,
            Falloff::Linear,
            Falloff::Inverse,
            Falloff::InverseSquare,
            Falloff::Smooth,
        ];

        for falloff in falloffs {
            let expr = falloff.to_wgsl_expr();
            assert!(!expr.is_empty(), "Falloff {:?} should have expression", falloff);
        }
    }

    // ========== Rule Properties ==========

    #[test]
    fn test_requires_neighbors() {
        // Rules that should need neighbors
        let neighbor_rules = [
            Rule::Separate { radius: 0.1, strength: 1.0 },
            Rule::Cohere { radius: 0.5, strength: 1.0 },
            Rule::Align { radius: 0.3, strength: 1.0 },
            Rule::Collide { radius: 0.1, restitution: 0.8 },
            Rule::NBodyGravity { radius: 1.0, strength: 1.0, softening: 0.01 },
        ];

        for rule in &neighbor_rules {
            assert!(
                rule.requires_neighbors(),
                "Rule {:?} should need neighbors",
                std::mem::discriminant(rule)
            );
        }

        // Rules that should NOT need neighbors
        let solo_rules = [
            Rule::Gravity(9.8),
            Rule::Drag(1.0),
            Rule::BounceWalls,
            Rule::WrapWalls,
        ];

        for rule in &solo_rules {
            assert!(
                !rule.requires_neighbors(),
                "Rule {:?} should not need neighbors",
                std::mem::discriminant(rule)
            );
        }
    }

    // ========== Bounds Substitution ==========

    #[test]
    fn test_bounds_substitution() {
        let rule = Rule::BounceWalls;

        // Test with bounds = 1.0
        let wgsl_1 = rule.to_wgsl(1.0);
        assert!(wgsl_1.contains("-1"));
        assert!(wgsl_1.contains("1")); // positive bound

        // Test with bounds = 2.5
        let wgsl_2 = rule.to_wgsl(2.5);
        assert!(wgsl_2.contains("-2.5"));
        assert!(wgsl_2.contains("2.5"));
    }
}
