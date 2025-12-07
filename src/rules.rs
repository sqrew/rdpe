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

    /// Steering behavior: seek a target point.
    ///
    /// Applies a steering force toward a target, adjusting velocity smoothly
    /// rather than setting it directly. Core autonomous agent behavior.
    ///
    /// # Fields
    ///
    /// - `target` - Position to seek toward
    /// - `max_speed` - Maximum velocity magnitude
    /// - `max_force` - Maximum steering force (lower = smoother turns)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Seek {
    ///     target: Vec3::new(0.5, 0.0, 0.0),
    ///     max_speed: 2.0,
    ///     max_force: 0.5,
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// For dynamic targets (like mouse position), use `Rule::Custom` with
    /// uniforms or combine with fields.
    Seek {
        /// Target position to seek.
        target: Vec3,
        /// Maximum speed.
        max_speed: f32,
        /// Maximum steering force.
        max_force: f32,
    },

    /// Steering behavior: flee from a point.
    ///
    /// Opposite of Seek - applies steering force away from a point.
    /// The panic_radius controls when fleeing kicks in.
    ///
    /// # Fields
    ///
    /// - `target` - Position to flee from
    /// - `max_speed` - Maximum velocity magnitude
    /// - `max_force` - Maximum steering force
    /// - `panic_radius` - Distance at which fleeing activates (0 = always flee)
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Flee {
    ///     target: Vec3::ZERO,
    ///     max_speed: 3.0,
    ///     max_force: 1.0,
    ///     panic_radius: 0.5,  // Only flee when close
    /// }
    /// ```
    Flee {
        /// Position to flee from.
        target: Vec3,
        /// Maximum speed.
        max_speed: f32,
        /// Maximum steering force.
        max_force: f32,
        /// Radius within which to flee (0 = always flee).
        panic_radius: f32,
    },

    /// Steering behavior: arrive at a target with deceleration.
    ///
    /// Like Seek but slows down as it approaches the target, coming to a
    /// smooth stop. Essential for realistic goal-seeking behavior.
    ///
    /// # Fields
    ///
    /// - `target` - Position to arrive at
    /// - `max_speed` - Maximum velocity magnitude
    /// - `max_force` - Maximum steering force
    /// - `slowing_radius` - Distance at which deceleration begins
    ///
    /// # Example
    ///
    /// ```ignore
    /// Rule::Arrive {
    ///     target: Vec3::ZERO,
    ///     max_speed: 2.0,
    ///     max_force: 0.5,
    ///     slowing_radius: 0.3,  // Start slowing at 0.3 units away
    /// }
    /// ```
    Arrive {
        /// Target position to arrive at.
        target: Vec3,
        /// Maximum speed.
        max_speed: f32,
        /// Maximum steering force.
        max_force: f32,
        /// Distance at which to start slowing down.
        slowing_radius: f32,
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

    /// Lennard-Jones potential for molecular dynamics.
    ///
    /// **Requires spatial hashing.** Creates realistic molecular interactions:
    /// strong repulsion at close range (Pauli exclusion) and weak attraction
    /// at medium range (Van der Waals). Perfect for crystal formation,
    /// liquid simulations, and soft matter physics.
    ///
    /// # Fields
    ///
    /// - `epsilon` - Well depth (strength of attraction at equilibrium)
    /// - `sigma` - Zero-crossing distance (particle diameter)
    /// - `cutoff` - Maximum interaction range (typically 2.5 * sigma)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.3, 32)
    /// .with_rule(Rule::LennardJones {
    ///     epsilon: 1.0,          // Attraction strength
    ///     sigma: 0.1,            // Particle "size"
    ///     cutoff: 0.25,          // 2.5 * sigma
    /// })
    /// ```
    ///
    /// # Physics Note
    ///
    /// The potential is V(r) = 4ε[(σ/r)¹² - (σ/r)⁶]. Particles settle at
    /// r ≈ 1.12σ (the equilibrium distance). Combine with temperature-based
    /// velocity initialization for molecular dynamics simulations.
    LennardJones {
        /// Well depth (attraction strength at equilibrium).
        epsilon: f32,
        /// Zero-crossing distance (effective particle diameter).
        sigma: f32,
        /// Cutoff radius for interaction (typically 2.5 * sigma).
        cutoff: f32,
    },

    /// Diffusion-Limited Aggregation (DLA) for fractal growth.
    ///
    /// **Requires spatial hashing.** Particles perform random walks until
    /// they contact a "seed" particle (type 0 by default), at which point
    /// they stick and become part of the growing structure. Creates beautiful
    /// fractal patterns like snowflakes, coral, lightning, and mineral deposits.
    ///
    /// # Fields
    ///
    /// - `seed_type` - Particle type that acts as the seed/structure
    /// - `mobile_type` - Particle type that diffuses and sticks
    /// - `stick_radius` - Distance at which particles stick
    /// - `diffusion_strength` - Random walk intensity
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.1, 32)
    /// .with_rule(Rule::DLA {
    ///     seed_type: 0,
    ///     mobile_type: 1,
    ///     stick_radius: 0.05,
    ///     diffusion_strength: 0.5,
    /// })
    /// ```
    ///
    /// # Physics Note
    ///
    /// Real DLA requires very slow aggregation for proper fractal dimension.
    /// For visual effect, higher diffusion works fine. Initialize with one
    /// seed particle (type 0) at center and many mobile particles (type 1).
    DLA {
        /// Particle type that forms the structure (immobile once stuck).
        seed_type: u32,
        /// Particle type that diffuses until it sticks.
        mobile_type: u32,
        /// Contact radius for sticking.
        stick_radius: f32,
        /// Brownian motion intensity.
        diffusion_strength: f32,
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

    /// Refractory period / cooldown mechanic for particle fields.
    ///
    /// Models a "charge" that depletes when a trigger field is active and
    /// regenerates when inactive. Perfect for:
    /// - Bioluminescence (luciferin depletion/regeneration)
    /// - Neuron firing (refractory period)
    /// - Ability cooldowns
    /// - Energy systems
    ///
    /// When `trigger` field is above `active_threshold`:
    /// - Charge depletes at `depletion_rate * trigger_value`
    ///
    /// When `trigger` field is below `active_threshold`:
    /// - Charge regenerates at `regen_rate`
    ///
    /// # Fields
    ///
    /// - `trigger` - Field that triggers depletion (e.g., "glow")
    /// - `charge` - Field storing the charge level (e.g., "energy")
    /// - `active_threshold` - Trigger value above which charge depletes
    /// - `depletion_rate` - How fast charge depletes when active
    /// - `regen_rate` - How fast charge regenerates when inactive
    ///
    /// # Example: Bioluminescence
    ///
    /// ```ignore
    /// Rule::Refractory {
    ///     trigger: "glow".into(),
    ///     charge: "luciferin".into(),
    ///     active_threshold: 0.05,
    ///     depletion_rate: 0.03,
    ///     regen_rate: 0.008,
    /// }
    /// ```
    ///
    /// # Example: Neuron firing
    ///
    /// ```ignore
    /// Rule::Refractory {
    ///     trigger: "firing".into(),
    ///     charge: "membrane_potential".into(),
    ///     active_threshold: 0.1,
    ///     depletion_rate: 0.5,   // Fast depletion during firing
    ///     regen_rate: 0.02,      // Slow recovery
    /// }
    /// ```
    Refractory {
        /// Field that triggers depletion when high.
        trigger: String,
        /// Field storing the charge (0.0 to 1.0).
        charge: String,
        /// Threshold above which trigger causes depletion.
        active_threshold: f32,
        /// Depletion rate (multiplied by trigger value).
        depletion_rate: f32,
        /// Regeneration rate when trigger is low.
        regen_rate: f32,
    },

    /// Run custom WGSL code when a particle dies.
    ///
    /// Triggered on the frame when `p.alive` transitions from 1 to 0.
    /// Useful for death effects, recording final state, or triggering
    /// other systems.
    ///
    /// # Available Variables
    ///
    /// - `p` - The particle at the moment of death (read/write)
    /// - `index` - Particle index (`u32`)
    /// - `uniforms.time`, `uniforms.delta_time` - Time values
    ///
    /// # Example: Flash color on death
    ///
    /// ```ignore
    /// .with_rule(Rule::Lifetime(3.0))
    /// .with_rule(Rule::OnDeath {
    ///     action: r#"
    ///         p.color = vec3<f32>(1.0, 1.0, 1.0); // Flash white
    ///     "#.into(),
    /// })
    /// ```
    ///
    /// # Example: Deposit to field on death
    ///
    /// ```ignore
    /// .with_rule(Rule::OnDeath {
    ///     action: r#"
    ///         // Leave a "corpse" marker in the field
    ///         field_write(0u, p.position, 1.0);
    ///     "#.into(),
    /// })
    /// ```
    ///
    /// # Note
    ///
    /// The action runs after all other rules have executed, just before
    /// the particle is written back to the buffer. The particle is still
    /// "alive" in the sense that you can modify its fields, but `p.alive`
    /// will be 0.
    OnDeath {
        /// WGSL code to execute when particle dies.
        action: String,
    },

    /// Run custom WGSL code when a condition is true.
    ///
    /// A declarative wrapper around conditional logic. More readable than
    /// `Rule::Custom` with an if statement when the pattern is simple.
    ///
    /// # Fields
    ///
    /// - `condition` - WGSL boolean expression (e.g., `"p.energy < 0.1"`)
    /// - `action` - WGSL code to run when condition is true
    ///
    /// # Example: Low health warning
    ///
    /// ```ignore
    /// Rule::OnCondition {
    ///     condition: "p.health < 0.2".into(),
    ///     action: "p.color = vec3<f32>(1.0, 0.0, 0.0);".into(),
    /// }
    /// ```
    ///
    /// # Example: Speed boost when charged
    ///
    /// ```ignore
    /// Rule::OnCondition {
    ///     condition: "p.charge > 0.8".into(),
    ///     action: r#"
    ///         p.velocity *= 1.5;
    ///         p.color = vec3<f32>(1.0, 1.0, 0.0);
    ///     "#.into(),
    /// }
    /// ```
    OnCondition {
        /// WGSL boolean expression.
        condition: String,
        /// WGSL code to execute when condition is true.
        action: String,
    },

    /// Run custom WGSL code at regular time intervals.
    ///
    /// Triggers once per interval when the simulation time crosses an
    /// interval boundary. Useful for periodic effects like pulses,
    /// spawning, or state changes.
    ///
    /// # Fields
    ///
    /// - `interval` - Time between triggers in seconds
    /// - `action` - WGSL code to run at each interval
    ///
    /// # Example: Periodic pulse
    ///
    /// ```ignore
    /// Rule::OnInterval {
    ///     interval: 0.5,
    ///     action: "p.glow = 1.0;".into(),
    /// }
    /// ```
    ///
    /// # Example: Random color change every second
    ///
    /// ```ignore
    /// Rule::OnInterval {
    ///     interval: 1.0,
    ///     action: r#"
    ///         p.color = vec3<f32>(
    ///             rand_f32(index, 0u),
    ///             rand_f32(index, 1u),
    ///             rand_f32(index, 2u)
    ///         );
    ///     "#.into(),
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// The trigger detection uses `floor(time / interval)` comparison,
    /// so it fires once per particle when crossing each interval boundary.
    OnInterval {
        /// Time between triggers in seconds.
        interval: f32,
        /// WGSL code to execute at each interval.
        action: String,
    },

    /// Run custom WGSL code when a particle spawns.
    ///
    /// Triggered on the frame when `p.alive` transitions from 0 to 1
    /// (i.e., when an emitter spawns the particle). Useful for
    /// initialization effects, random starting values, or spawn bursts.
    ///
    /// # Available Variables
    ///
    /// - `p` - The newly spawned particle (read/write)
    /// - `index` - Particle index (`u32`)
    /// - `uniforms.time`, `uniforms.delta_time` - Time values
    ///
    /// # Example: Random color on spawn
    ///
    /// ```ignore
    /// .with_rule(Rule::OnSpawn {
    ///     action: r#"
    ///         p.color = vec3<f32>(
    ///             rand_f32(index, 0u),
    ///             rand_f32(index, 1u),
    ///             rand_f32(index, 2u)
    ///         );
    ///     "#.into(),
    /// })
    /// ```
    ///
    /// # Example: Spawn burst effect
    ///
    /// ```ignore
    /// .with_rule(Rule::OnSpawn {
    ///     action: r#"
    ///         p.glow = 1.0;
    ///         p.scale = 2.0;
    ///     "#.into(),
    /// })
    /// ```
    ///
    /// # Note
    ///
    /// This triggers for emitter-spawned particles, not for particles
    /// created at simulation start (those are initialized via the spawner).
    OnSpawn {
        /// WGSL code to execute when particle spawns.
        action: String,
    },

    /// Copy one particle field to another.
    ///
    /// Simple field assignment. Useful for tracking previous values,
    /// creating derived fields, or synchronizing state.
    ///
    /// # Fields
    ///
    /// - `from` - Source field name
    /// - `to` - Destination field name
    ///
    /// # Example: Track previous position
    ///
    /// ```ignore
    /// Rule::CopyField {
    ///     from: "position".into(),
    ///     to: "prev_position".into(),
    /// }
    /// ```
    ///
    /// # Example: Sync color to velocity-based color
    ///
    /// ```ignore
    /// Rule::CopyField {
    ///     from: "computed_color".into(),
    ///     to: "color".into(),
    /// }
    /// ```
    CopyField {
        /// Source field name.
        from: String,
        /// Destination field name.
        to: String,
    },

    /// Accumulate a value from neighbors into a target field.
    ///
    /// Gathers values from nearby particles and combines them using the
    /// specified operation. Useful for sensing neighbor properties like
    /// density, average temperature, maximum threat, etc.
    ///
    /// # Fields
    ///
    /// - `source` - Field to read from neighbors (f32)
    /// - `target` - Field to write the result to (f32)
    /// - `radius` - Neighbor detection radius
    /// - `operation` - How to combine values: "sum", "average", "max", "min"
    /// - `falloff` - Optional distance falloff for weighting
    ///
    /// # Example: Sense neighbor density
    ///
    /// ```ignore
    /// Rule::Accumulate {
    ///     source: "mass".into(),
    ///     target: "sensed_density".into(),
    ///     radius: 0.2,
    ///     operation: "sum".into(),
    ///     falloff: Some(Falloff::Linear),
    /// }
    /// ```
    ///
    /// # Example: Find hottest neighbor
    ///
    /// ```ignore
    /// Rule::Accumulate {
    ///     source: "temperature".into(),
    ///     target: "max_nearby_temp".into(),
    ///     radius: 0.15,
    ///     operation: "max".into(),
    ///     falloff: None,
    /// }
    /// ```
    ///
    /// # Example: Average neighbor energy
    ///
    /// ```ignore
    /// Rule::Accumulate {
    ///     source: "energy".into(),
    ///     target: "local_energy".into(),
    ///     radius: 0.1,
    ///     operation: "average".into(),
    ///     falloff: Some(Falloff::InverseSquare),
    /// }
    /// ```
    Accumulate {
        /// Field to read from neighbors (f32).
        source: String,
        /// Field to write result to (f32).
        target: String,
        /// Neighbor detection radius.
        radius: f32,
        /// How to combine values: "sum", "average", "max", "min".
        operation: String,
        /// Optional distance-based weighting.
        falloff: Option<Falloff>,
    },

    /// Deposit a particle value into a 3D field at the particle's position.
    ///
    /// Particles leave a trail or accumulate influence in a spatial field.
    /// Useful for pheromone trails, heat maps, density fields, etc.
    ///
    /// # Fields
    ///
    /// - `field_index` - Index of the 3D field to write to (0, 1, 2...)
    /// - `source` - Particle field to read value from (f32)
    /// - `amount` - Multiplier for the deposited value
    ///
    /// # Example: Pheromone trail
    ///
    /// ```ignore
    /// // Ants deposit pheromones as they walk
    /// Rule::Deposit {
    ///     field_index: 0,
    ///     source: "pheromone_strength".into(),
    ///     amount: 0.1,
    /// }
    /// ```
    ///
    /// # Example: Heat emission
    ///
    /// ```ignore
    /// // Hot particles warm the field around them
    /// Rule::Deposit {
    ///     field_index: 0,  // heat field
    ///     source: "temperature".into(),
    ///     amount: 0.05,
    /// }
    /// ```
    Deposit {
        /// Index of the 3D field (registered via with_field).
        field_index: u32,
        /// Particle field to read value from.
        source: String,
        /// Amount multiplier (scales the deposited value).
        amount: f32,
    },

    /// Read a value from a 3D field at the particle's position.
    ///
    /// Particles sample the field and store the result in a particle property.
    /// Use with field_gradient for chemotaxis-style behavior.
    ///
    /// # Fields
    ///
    /// - `field_index` - Index of the 3D field to read from
    /// - `target` - Particle field to store the value in (f32)
    ///
    /// # Example: Sense pheromones
    ///
    /// ```ignore
    /// // Read pheromone concentration into particle
    /// Rule::Sense {
    ///     field_index: 0,
    ///     target: "sensed_pheromone".into(),
    /// }
    /// ```
    Sense {
        /// Index of the 3D field to read from.
        field_index: u32,
        /// Particle field to store the sensed value.
        target: String,
    },

    /// Read and consume value from a 3D field (depletes the field).
    ///
    /// Particles extract resources from the field, reducing its value.
    /// The consumed amount is stored in a particle field.
    ///
    /// # Fields
    ///
    /// - `field_index` - Index of the 3D field to consume from
    /// - `target` - Particle field to store consumed amount
    /// - `rate` - Maximum consumption rate per frame
    ///
    /// # Example: Eating food
    ///
    /// ```ignore
    /// // Consume food from field, gain energy
    /// Rule::Consume {
    ///     field_index: 0,  // food field
    ///     target: "energy".into(),
    ///     rate: 0.1,
    /// }
    /// ```
    Consume {
        /// Index of the 3D field to consume from.
        field_index: u32,
        /// Particle field to store consumed amount.
        target: String,
        /// Maximum consumption rate per frame.
        rate: f32,
    },

    /// Broadcast a signal value to nearby particles.
    ///
    /// The particle writes its value to all neighbors within radius.
    /// Neighbors accumulate signals (additive). Useful for communication,
    /// alarm signals, or influence spreading.
    ///
    /// Note: Due to GPU parallelism, signals are accumulated additively.
    /// For "strongest signal wins", use Rule::Accumulate with operation: "max".
    ///
    /// # Fields
    ///
    /// - `source` - Particle field containing the signal to broadcast
    /// - `target` - Particle field on neighbors to receive the signal
    /// - `radius` - Broadcast radius
    /// - `strength` - Signal strength multiplier
    /// - `falloff` - Optional distance-based attenuation
    ///
    /// # Example: Alarm signal
    ///
    /// ```ignore
    /// // Frightened particles broadcast alarm to neighbors
    /// Rule::Signal {
    ///     source: "alarm".into(),
    ///     target: "received_alarm".into(),
    ///     radius: 0.5,
    ///     strength: 1.0,
    ///     falloff: Some(Falloff::InverseSquare),
    /// }
    /// ```
    Signal {
        /// Particle field containing the signal value.
        source: String,
        /// Particle field on neighbors to write to.
        target: String,
        /// Broadcast radius.
        radius: f32,
        /// Signal strength multiplier.
        strength: f32,
        /// Optional distance-based falloff.
        falloff: Option<Falloff>,
    },

    /// Absorb nearby particles of a matching type.
    ///
    /// When particles are within range, the absorber kills them and
    /// accumulates their properties (mass, energy, etc.).
    ///
    /// # Fields
    ///
    /// - `target_type` - Type of particles to absorb (None = any type)
    /// - `radius` - Absorption radius
    /// - `source_field` - Field to absorb from dying particles
    /// - `target_field` - Field to accumulate absorbed values into
    ///
    /// # Example: Predator eating prey
    ///
    /// ```ignore
    /// Rule::Absorb {
    ///     target_type: Some(Species::Prey.into()),
    ///     radius: 0.1,
    ///     source_field: "energy".into(),
    ///     target_field: "energy".into(),
    /// }
    /// ```
    ///
    /// # Example: Black hole absorbing mass
    ///
    /// ```ignore
    /// Rule::Typed {
    ///     self_type: ParticleType::BlackHole.into(),
    ///     rule: Box::new(Rule::Absorb {
    ///         target_type: None,  // absorb anything
    ///         radius: 0.05,
    ///         source_field: "mass".into(),
    ///         target_field: "mass".into(),
    ///     }),
    /// }
    /// ```
    Absorb {
        /// Type of particles to absorb (None = any type).
        target_type: Option<u32>,
        /// Absorption radius.
        radius: f32,
        /// Field to read from absorbed particles.
        source_field: String,
        /// Field to accumulate absorbed values into.
        target_field: String,
    },

    /// Switch between two rules based on a condition.
    ///
    /// Evaluates a WGSL condition and applies either the `then` rule
    /// or the `else` rule. Useful for state-dependent behavior.
    ///
    /// # Fields
    ///
    /// - `condition` - WGSL boolean expression
    /// - `then_rule` - Rule to apply when condition is true
    /// - `else_rule` - Optional rule to apply when condition is false
    ///
    /// # Example: Flee when low health
    ///
    /// ```ignore
    /// Rule::Switch {
    ///     condition: "p.health < 0.3".into(),
    ///     then_rule: Box::new(Rule::Evade { ... }),
    ///     else_rule: Some(Box::new(Rule::Chase { ... })),
    /// }
    /// ```
    ///
    /// # Example: Different behavior by type
    ///
    /// ```ignore
    /// Rule::Switch {
    ///     condition: "p.particle_type == 0u".into(),
    ///     then_rule: Box::new(Rule::Gravity(9.8)),
    ///     else_rule: Some(Box::new(Rule::Gravity(-5.0))),
    /// }
    /// ```
    Switch {
        /// WGSL boolean condition (has access to `p`, `uniforms`).
        condition: String,
        /// Rule to apply when condition is true.
        then_rule: Box<Rule>,
        /// Optional rule to apply when condition is false.
        else_rule: Option<Box<Rule>>,
    },

    /// Divide a particle when a condition is met.
    ///
    /// When the condition evaluates to true, the particle spawns offspring
    /// and optionally consumes resources (e.g., energy, mass). Uses the
    /// sub-emitter system for spawning.
    ///
    /// # Fields
    ///
    /// - `condition` - WGSL boolean expression (has access to `p`, `uniforms`)
    /// - `offspring_count` - Number of offspring to spawn (1-10)
    /// - `offspring_type` - Particle type for offspring (same as parent if None)
    /// - `resource_field` - Optional field to consume when splitting
    /// - `resource_cost` - Amount of resource consumed per split
    /// - `spread` - Velocity spread angle in radians (default: PI/4)
    /// - `speed` - Speed range for offspring velocity
    ///
    /// # Example: Cell Division
    ///
    /// ```ignore
    /// Rule::Split {
    ///     condition: "p.energy > 1.5".into(),
    ///     offspring_count: 2,
    ///     offspring_type: None,  // Same type as parent
    ///     resource_field: Some("energy".into()),
    ///     resource_cost: 0.8,
    ///     spread: std::f32::consts::PI / 4.0,
    ///     speed: 0.1..0.3,
    /// }
    /// ```
    ///
    /// # Example: Fragmentation
    ///
    /// ```ignore
    /// Rule::Split {
    ///     condition: "p.health < 0.1".into(),
    ///     offspring_count: 5,
    ///     offspring_type: Some(FragmentType.into()),
    ///     resource_field: None,
    ///     resource_cost: 0.0,
    ///     spread: std::f32::consts::TAU,  // Full sphere
    ///     speed: 0.5..1.5,
    /// }
    /// ```
    Split {
        /// WGSL boolean condition that triggers splitting.
        condition: String,
        /// Number of offspring particles to spawn.
        offspring_count: u32,
        /// Particle type for offspring (None = same as parent).
        offspring_type: Option<u32>,
        /// Optional particle field to consume when splitting.
        resource_field: Option<String>,
        /// Amount of resource consumed per split.
        resource_cost: f32,
        /// Spread angle for offspring velocity (radians).
        spread: f32,
        /// Speed range for offspring (min, max).
        speed_min: f32,
        speed_max: f32,
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
    /// Returns `true` if this is an OnDeath rule.
    pub fn is_on_death(&self) -> bool {
        matches!(self, Rule::OnDeath { .. })
    }

    /// Generate WGSL code for OnDeath handling.
    ///
    /// Returns the code to run when `was_alive == 1u && p.alive == 0u`.
    pub fn to_on_death_wgsl(&self) -> String {
        match self {
            Rule::OnDeath { action } => format!(
                "        // OnDeath action\n{}",
                action
            ),
            _ => String::new(),
        }
    }

    /// Returns `true` if this is an OnSpawn rule.
    pub fn is_on_spawn(&self) -> bool {
        matches!(self, Rule::OnSpawn { .. })
    }

    /// Generate WGSL code for OnSpawn handling.
    ///
    /// Returns the code to run when `was_alive == 0u && p.alive == 1u`.
    pub fn to_on_spawn_wgsl(&self) -> String {
        match self {
            Rule::OnSpawn { action } => format!(
                "        // OnSpawn action\n{}",
                action
            ),
            _ => String::new(),
        }
    }

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
            | Rule::LennardJones { .. }
            | Rule::DLA { .. }
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
            | Rule::Accumulate { .. }
            | Rule::Signal { .. }
            | Rule::Absorb { .. }
            | Rule::NeighborCustom(_) => true,
            Rule::Typed { rule, .. } => rule.requires_neighbors(),
            Rule::Switch { then_rule, else_rule, .. } => {
                then_rule.requires_neighbors()
                    || else_rule.as_ref().map(|r| r.requires_neighbors()).unwrap_or(false)
            }
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

    /// Returns `true` if this rule uses accumulate accumulators.
    pub(crate) fn needs_accumulate_accumulator(&self) -> bool {
        match self {
            Rule::Accumulate { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_accumulate_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses signal accumulators.
    pub(crate) fn needs_signal_accumulator(&self) -> bool {
        match self {
            Rule::Signal { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_signal_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses absorb accumulators.
    pub(crate) fn needs_absorb_accumulator(&self) -> bool {
        match self {
            Rule::Absorb { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_absorb_accumulator(),
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

            Rule::Seek { target, max_speed, max_force } => format!(
                r#"    // Seek: steering toward target
    {{
        let seek_target = vec3<f32>({tx}, {ty}, {tz});
        let desired = seek_target - p.position;
        let dist = length(desired);
        if dist > 0.001 {{
            // Desired velocity at max speed toward target
            let desired_vel = normalize(desired) * {max_speed};
            // Steering = desired - current velocity
            var steering = desired_vel - p.velocity;
            let steer_mag = length(steering);
            if steer_mag > {max_force} {{
                steering = normalize(steering) * {max_force};
            }}
            p.velocity += steering * uniforms.delta_time;
        }}
    }}"#,
                tx = target.x, ty = target.y, tz = target.z,
                max_speed = max_speed, max_force = max_force
            ),

            Rule::Flee { target, max_speed, max_force, panic_radius } => format!(
                r#"    // Flee: steering away from target
    {{
        let flee_target = vec3<f32>({tx}, {ty}, {tz});
        let away = p.position - flee_target;
        let dist = length(away);
        let should_flee = {panic_radius} <= 0.0 || dist < {panic_radius};
        if should_flee && dist > 0.001 {{
            // Desired velocity at max speed away from target
            let desired_vel = normalize(away) * {max_speed};
            // Steering = desired - current velocity
            var steering = desired_vel - p.velocity;
            let steer_mag = length(steering);
            if steer_mag > {max_force} {{
                steering = normalize(steering) * {max_force};
            }}
            p.velocity += steering * uniforms.delta_time;
        }}
    }}"#,
                tx = target.x, ty = target.y, tz = target.z,
                max_speed = max_speed, max_force = max_force, panic_radius = panic_radius
            ),

            Rule::Arrive { target, max_speed, max_force, slowing_radius } => format!(
                r#"    // Arrive: seek with deceleration
    {{
        let arrive_target = vec3<f32>({tx}, {ty}, {tz});
        let desired = arrive_target - p.position;
        let dist = length(desired);
        if dist > 0.001 {{
            // Scale desired speed based on distance
            var target_speed = {max_speed};
            if dist < {slowing_radius} {{
                target_speed = {max_speed} * (dist / {slowing_radius});
            }}
            let desired_vel = normalize(desired) * target_speed;
            var steering = desired_vel - p.velocity;
            let steer_mag = length(steering);
            if steer_mag > {max_force} {{
                steering = normalize(steering) * {max_force};
            }}
            p.velocity += steering * uniforms.delta_time;
        }}
    }}"#,
                tx = target.x, ty = target.y, tz = target.z,
                max_speed = max_speed, max_force = max_force, slowing_radius = slowing_radius
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

            Rule::OnCondition { condition, action } => format!(
                r#"    // OnCondition
    if {condition} {{
{action}
    }}"#
            ),

            Rule::OnInterval { interval, action } => format!(
                r#"    // OnInterval ({interval}s)
    {{
        let prev_intervals = floor((uniforms.time - uniforms.delta_time) / {interval});
        let curr_intervals = floor(uniforms.time / {interval});
        if curr_intervals > prev_intervals {{
{action}
        }}
    }}"#
            ),

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

            Rule::Refractory {
                trigger,
                charge,
                active_threshold,
                depletion_rate,
                regen_rate,
            } => format!(
                r#"    // Refractory: charge depletion/regeneration
    if p.{trigger} > {active_threshold} {{
        // Trigger is active - deplete charge
        p.{charge} = max(p.{charge} - p.{trigger} * {depletion_rate}, 0.0);
    }} else {{
        // Trigger inactive - regenerate charge
        p.{charge} = min(p.{charge} + {regen_rate}, 1.0);
    }}"#
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
                        code.push('\n');
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

            Rule::CopyField { from, to } => format!(
                "    // Copy field\n    p.{to} = p.{from};"
            ),

            // Neighbor rules generate code through to_neighbor_wgsl
            Rule::Collide { .. }
            | Rule::OnCollision { .. }
            | Rule::NBodyGravity { .. }
            | Rule::LennardJones { .. }
            | Rule::DLA { .. }
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
            | Rule::Accumulate { .. }
            | Rule::Signal { .. }
            | Rule::Absorb { .. }
            | Rule::NeighborCustom(_)
            | Rule::OnDeath { .. }
            | Rule::OnSpawn { .. } => String::new(), // OnDeath/OnSpawn handled specially

            Rule::Deposit { field_index, source, amount } => format!(
                r#"    // Deposit: write particle value to field
    field_write({field_index}u, p.position, p.{source} * {amount});"#
            ),

            Rule::Sense { field_index, target } => format!(
                r#"    // Sense: read field value into particle
    p.{target} = field_read({field_index}u, p.position);"#
            ),

            Rule::Consume { field_index, target, rate } => format!(
                r#"    // Consume: read field value and deplete
    let consumed_val = min(field_read({field_index}u, p.position), {rate});
    p.{target} += consumed_val;
    field_write({field_index}u, p.position, -consumed_val);"#
            ),

            Rule::Switch { condition, then_rule, else_rule } => {
                let then_code = then_rule.to_wgsl(bounds);
                let else_code = else_rule.as_ref().map(|r| r.to_wgsl(bounds)).unwrap_or_default();
                if else_code.is_empty() {
                    format!(
                        r#"    // Switch: conditional rule
    if {condition} {{
{then_code}
    }}"#
                    )
                } else {
                    format!(
                        r#"    // Switch: conditional rule
    if {condition} {{
{then_code}
    }} else {{
{else_code}
    }}"#
                    )
                }
            },

            Rule::Split {
                condition,
                offspring_count,
                offspring_type,
                resource_field,
                resource_cost,
                spread,
                speed_min,
                speed_max,
            } => {
                let resource_check = if let Some(field) = resource_field {
                    format!(" && p.{field} >= {resource_cost}")
                } else {
                    String::new()
                };

                let resource_deduct = if let Some(field) = resource_field {
                    format!("\n        p.{field} -= {resource_cost};")
                } else {
                    String::new()
                };

                let child_type = offspring_type
                    .map(|t| format!("{t}u"))
                    .unwrap_or_else(|| "p.particle_type".to_string());

                format!(
                    r#"    // Split: spawn offspring when condition met
    if ({condition}){resource_check} {{
        // Record split event for sub-emitter processing
        let split_idx = atomicAdd(&death_count, 1u);
        if split_idx < arrayLength(&death_buffer) {{
            death_buffer[split_idx].position = p.position;
            death_buffer[split_idx].velocity = p.velocity;
            death_buffer[split_idx].color = p.color;
            death_buffer[split_idx].parent_type = {child_type};
            // Store spawn parameters in padding fields
            // offspring_count={offspring_count}, spread={spread}, speed_min={speed_min}, speed_max={speed_max}
        }}{resource_deduct}
    }}"#
                )
            },
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

            Rule::LennardJones { epsilon, sigma, cutoff } => format!(
                r#"            // Lennard-Jones potential
            if neighbor_dist < {cutoff} && neighbor_dist > 0.0001 {{
                // LJ potential: V(r) = 4ε[(σ/r)^12 - (σ/r)^6]
                // Force: F(r) = 24ε/r * [2(σ/r)^12 - (σ/r)^6]
                let sr = {sigma} / neighbor_dist;
                let sr6 = sr * sr * sr * sr * sr * sr;
                let sr12 = sr6 * sr6;
                // Force magnitude (positive = repulsive, negative = attractive)
                let force_mag = 24.0 * {epsilon} / neighbor_dist * (2.0 * sr12 - sr6);
                // Apply force along neighbor direction (positive pushes away)
                p.velocity += neighbor_dir * force_mag * uniforms.delta_time;
            }}"#
            ),

            Rule::DLA { seed_type, mobile_type, stick_radius, diffusion_strength } => format!(
                r#"            // Diffusion-Limited Aggregation
            // Mobile particles stick to seed particles on contact
            if p.particle_type == {mobile_type}u && other.particle_type == {seed_type}u {{
                if neighbor_dist < {stick_radius} {{
                    // Stick: become part of the structure
                    p.particle_type = {seed_type}u;
                    p.velocity = vec3<f32>(0.0, 0.0, 0.0);
                }}
            }}
            // Apply diffusion (random walk) to mobile particles
            if p.particle_type == {mobile_type}u {{
                let diff_seed = index * 1103515245u + u32(uniforms.time * 1000.0);
                let hx = (diff_seed ^ (diff_seed >> 15u)) * 0x45d9f3bu;
                let hy = ((diff_seed + 1u) ^ ((diff_seed + 1u) >> 15u)) * 0x45d9f3bu;
                let hz = ((diff_seed + 2u) ^ ((diff_seed + 2u) >> 15u)) * 0x45d9f3bu;
                let diff_force = vec3<f32>(
                    f32(hx & 0xFFFFu) / 32768.0 - 1.0,
                    f32(hy & 0xFFFFu) / 32768.0 - 1.0,
                    f32(hz & 0xFFFFu) / 32768.0 - 1.0
                );
                p.velocity += diff_force * {diffusion_strength} * uniforms.delta_time;
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

            Rule::Accumulate { source, radius, falloff, operation, .. } => {
                let weight_expr = if let Some(f) = falloff {
                    // Falloff expressions expect `dist` and `radius` in scope
                    format!(
                        "let dist = neighbor_dist;\n                let radius = {radius};\n                let acc_weight = {};",
                        f.to_wgsl_expr()
                    )
                } else {
                    "let acc_weight = 1.0;".to_string()
                };

                let update_expr = match operation.as_str() {
                    "max" => format!("accumulate_value = max(accumulate_value, other.{source} * acc_weight);"),
                    "min" => format!("accumulate_value = min(accumulate_value, other.{source} * acc_weight);"),
                    _ => format!("accumulate_sum += other.{source} * acc_weight;\n                accumulate_weight += acc_weight;"),
                };

                format!(
                    r#"            // Accumulate: gather from neighbors
            if neighbor_dist < {radius} {{
                {weight_expr}
                {update_expr}
            }}"#
                )
            },

            Rule::Signal { source, target: _, radius, strength, falloff } => {
                let falloff_expr = if let Some(f) = falloff {
                    format!(
                        "let dist = neighbor_dist;\n                let radius = {radius};\n                let signal_strength = {} * {strength};",
                        f.to_wgsl_expr()
                    )
                } else {
                    format!("let signal_strength = {strength};")
                };

                format!(
                    r#"            // Signal: receive broadcast from neighbors
            if neighbor_dist < {radius} {{
                {falloff_expr}
                signal_sum += other.{source} * signal_strength;
                signal_count += 1.0;
            }}"#
                )
            },

            Rule::Absorb { target_type, radius, source_field, target_field: _ } => {
                let type_check = if let Some(t) = target_type {
                    format!("other.particle_type == {t}u && ")
                } else {
                    String::new()
                };

                format!(
                    r#"            // Absorb: consume nearby particles
            if {type_check}neighbor_dist < {radius} && other.alive == 1u {{
                absorb_sum += other.{source_field};
                // Mark neighbor for absorption (will be killed)
                if !absorb_found {{
                    absorb_found = true;
                    absorb_target_idx = other_idx;
                }}
            }}"#
                )
            },

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

            Rule::Accumulate { target, operation, .. } => {
                let assign_expr = match operation.as_str() {
                    "max" => format!("p.{target} = accumulate_value;"),
                    "min" => format!("p.{target} = accumulate_value;"),
                    "average" => format!(
                        "if accumulate_weight > 0.0 {{\n        p.{target} = accumulate_sum / accumulate_weight;\n    }}"
                    ),
                    // "sum" and default
                    _ => format!("p.{target} = accumulate_sum;"),
                };

                format!(
                    r#"    // Apply accumulated value
    {assign_expr}"#
                )
            },

            Rule::Signal { target, .. } => format!(
                r#"    // Apply received signal
    if signal_count > 0.0 {{
        p.{target} += signal_sum / signal_count;
    }}"#
            ),

            Rule::Absorb { target_field, .. } => format!(
                r#"    // Apply absorption result
    if absorb_found {{
        p.{target_field} += absorb_sum;
        // Kill the absorbed particle (must be done via storage access)
        // Note: Actual kill happens via particles_out[absorb_target_idx].alive = 0u;
    }}"#
            ),

            _ => String::new(),
        }
    }

    /// Get a human-readable display name for this rule.
    pub fn display_name(&self) -> &'static str {
        match self {
            Rule::Gravity(_) => "Gravity",
            Rule::BounceWalls => "Bounce Walls",
            Rule::WrapWalls => "Wrap Walls",
            Rule::Drag(_) => "Drag",
            Rule::Acceleration(_) => "Acceleration",
            Rule::AttractTo { .. } => "Attract To",
            Rule::RepelFrom { .. } => "Repel From",
            Rule::Seek { .. } => "Seek",
            Rule::Flee { .. } => "Flee",
            Rule::Arrive { .. } => "Arrive",
            Rule::Vortex { .. } => "Vortex",
            Rule::Turbulence { .. } => "Turbulence",
            Rule::Orbit { .. } => "Orbit",
            Rule::Curl { .. } => "Curl",
            Rule::PointGravity { .. } => "Point Gravity",
            Rule::Spring { .. } => "Spring",
            Rule::Radial { .. } => "Radial",
            Rule::Shockwave { .. } => "Shockwave",
            Rule::Pulse { .. } => "Pulse",
            Rule::Oscillate { .. } => "Oscillate",
            Rule::PositionNoise { .. } => "Position Noise",
            Rule::SpeedLimit { .. } => "Speed Limit",
            Rule::Wander { .. } => "Wander",
            Rule::Collide { .. } => "Collide",
            Rule::Separate { .. } => "Separate",
            Rule::Cohere { .. } => "Cohere",
            Rule::Align { .. } => "Align",
            Rule::Avoid { .. } => "Avoid",
            Rule::NBodyGravity { .. } => "N-Body Gravity",
            Rule::LennardJones { .. } => "Lennard-Jones",
            Rule::DLA { .. } => "DLA",
            Rule::Viscosity { .. } => "Viscosity",
            Rule::Pressure { .. } => "Pressure",
            Rule::Magnetism { .. } => "Magnetism",
            Rule::SurfaceTension { .. } => "Surface Tension",
            Rule::Typed { .. } => "Typed",
            Rule::Convert { .. } => "Convert",
            Rule::Chase { .. } => "Chase",
            Rule::Evade { .. } => "Evade",
            Rule::Age => "Age",
            Rule::Lifetime(_) => "Lifetime",
            Rule::FadeOut { .. } => "Fade Out",
            Rule::ShrinkOut { .. } => "Shrink Out",
            Rule::ColorOverLife { .. } => "Color Over Life",
            Rule::ColorBySpeed { .. } => "Color By Speed",
            Rule::ColorByAge { .. } => "Color By Age",
            Rule::ScaleBySpeed { .. } => "Scale By Speed",
            Rule::Custom(_) => "Custom",
            Rule::NeighborCustom(_) => "Neighbor Custom",
            Rule::OnDeath { .. } => "On Death",
            Rule::OnSpawn { .. } => "On Spawn",
            Rule::OnCollision { .. } => "On Collision",
            Rule::State { .. } => "State",
            Rule::Agent { .. } => "Agent",
            Rule::Signal { .. } => "Signal",
            Rule::Absorb { .. } => "Absorb",
            // Catch-all for any other variants
            _ => "Rule",
        }
    }

    /// Extract editable parameters from this rule with a unique prefix.
    ///
    /// Returns Vec of (parameter_name, value) pairs for runtime editing.
    /// The prefix ensures unique names when multiple rules of the same type exist.
    pub fn params(&self, index: usize) -> Vec<(String, crate::uniforms::UniformValue)> {
        use crate::uniforms::UniformValue;
        let prefix = format!("rule_{}", index);

        match self {
            Rule::Gravity(strength) => vec![
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Drag(coefficient) => vec![
                (format!("{}_coefficient", prefix), UniformValue::F32(*coefficient)),
            ],
            Rule::Acceleration(acc) => vec![
                (format!("{}_acceleration", prefix), UniformValue::Vec3(*acc)),
            ],
            Rule::AttractTo { point, strength } => vec![
                (format!("{}_point", prefix), UniformValue::Vec3(*point)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::RepelFrom { point, strength, radius } => vec![
                (format!("{}_point", prefix), UniformValue::Vec3(*point)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
            ],
            Rule::Seek { target, max_speed, max_force } => vec![
                (format!("{}_target", prefix), UniformValue::Vec3(*target)),
                (format!("{}_max_speed", prefix), UniformValue::F32(*max_speed)),
                (format!("{}_max_force", prefix), UniformValue::F32(*max_force)),
            ],
            Rule::Flee { target, max_speed, max_force, panic_radius } => vec![
                (format!("{}_target", prefix), UniformValue::Vec3(*target)),
                (format!("{}_max_speed", prefix), UniformValue::F32(*max_speed)),
                (format!("{}_max_force", prefix), UniformValue::F32(*max_force)),
                (format!("{}_panic_radius", prefix), UniformValue::F32(*panic_radius)),
            ],
            Rule::Arrive { target, max_speed, max_force, slowing_radius } => vec![
                (format!("{}_target", prefix), UniformValue::Vec3(*target)),
                (format!("{}_max_speed", prefix), UniformValue::F32(*max_speed)),
                (format!("{}_max_force", prefix), UniformValue::F32(*max_force)),
                (format!("{}_slowing_radius", prefix), UniformValue::F32(*slowing_radius)),
            ],
            Rule::Vortex { center, axis, strength } => vec![
                (format!("{}_center", prefix), UniformValue::Vec3(*center)),
                (format!("{}_axis", prefix), UniformValue::Vec3(*axis)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Turbulence { scale, strength } => vec![
                (format!("{}_scale", prefix), UniformValue::F32(*scale)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Orbit { center, strength } => vec![
                (format!("{}_center", prefix), UniformValue::Vec3(*center)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Curl { scale, strength } => vec![
                (format!("{}_scale", prefix), UniformValue::F32(*scale)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::PointGravity { point, strength, softening } => vec![
                (format!("{}_point", prefix), UniformValue::Vec3(*point)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_softening", prefix), UniformValue::F32(*softening)),
            ],
            Rule::Spring { anchor, stiffness, damping } => vec![
                (format!("{}_anchor", prefix), UniformValue::Vec3(*anchor)),
                (format!("{}_stiffness", prefix), UniformValue::F32(*stiffness)),
                (format!("{}_damping", prefix), UniformValue::F32(*damping)),
            ],
            Rule::Pulse { point, strength, frequency, radius } => vec![
                (format!("{}_point", prefix), UniformValue::Vec3(*point)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_frequency", prefix), UniformValue::F32(*frequency)),
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
            ],
            Rule::Oscillate { axis, amplitude, frequency, spatial_scale } => vec![
                (format!("{}_axis", prefix), UniformValue::Vec3(*axis)),
                (format!("{}_amplitude", prefix), UniformValue::F32(*amplitude)),
                (format!("{}_frequency", prefix), UniformValue::F32(*frequency)),
                (format!("{}_spatial_scale", prefix), UniformValue::F32(*spatial_scale)),
            ],
            Rule::SpeedLimit { min, max } => vec![
                (format!("{}_min", prefix), UniformValue::F32(*min)),
                (format!("{}_max", prefix), UniformValue::F32(*max)),
            ],
            Rule::Wander { strength, frequency } => vec![
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_frequency", prefix), UniformValue::F32(*frequency)),
            ],
            Rule::Separate { radius, strength } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Cohere { radius, strength } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Align { radius, strength } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Collide { radius, restitution } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_restitution", prefix), UniformValue::F32(*restitution)),
            ],
            Rule::Chase { self_type, target_type, radius, strength } => vec![
                (format!("{}_self_type", prefix), UniformValue::U32(*self_type)),
                (format!("{}_target_type", prefix), UniformValue::U32(*target_type)),
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Evade { self_type, threat_type, radius, strength } => vec![
                (format!("{}_self_type", prefix), UniformValue::U32(*self_type)),
                (format!("{}_threat_type", prefix), UniformValue::U32(*threat_type)),
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Lifetime(duration) => vec![
                (format!("{}_duration", prefix), UniformValue::F32(*duration)),
            ],
            Rule::Radial { point, strength, radius, .. } => vec![
                (format!("{}_point", prefix), UniformValue::Vec3(*point)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
            ],
            Rule::Shockwave { origin, speed, width, strength, repeat } => vec![
                (format!("{}_origin", prefix), UniformValue::Vec3(*origin)),
                (format!("{}_speed", prefix), UniformValue::F32(*speed)),
                (format!("{}_width", prefix), UniformValue::F32(*width)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_repeat", prefix), UniformValue::F32(*repeat)),
            ],
            Rule::PositionNoise { scale, strength, speed } => vec![
                (format!("{}_scale", prefix), UniformValue::F32(*scale)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_speed", prefix), UniformValue::F32(*speed)),
            ],
            Rule::NBodyGravity { strength, softening, radius } => vec![
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_softening", prefix), UniformValue::F32(*softening)),
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
            ],
            Rule::LennardJones { epsilon, sigma, cutoff } => vec![
                (format!("{}_epsilon", prefix), UniformValue::F32(*epsilon)),
                (format!("{}_sigma", prefix), UniformValue::F32(*sigma)),
                (format!("{}_cutoff", prefix), UniformValue::F32(*cutoff)),
            ],
            Rule::DLA { seed_type, mobile_type, stick_radius, diffusion_strength } => vec![
                (format!("{}_seed_type", prefix), UniformValue::U32(*seed_type)),
                (format!("{}_mobile_type", prefix), UniformValue::U32(*mobile_type)),
                (format!("{}_stick_radius", prefix), UniformValue::F32(*stick_radius)),
                (format!("{}_diffusion_strength", prefix), UniformValue::F32(*diffusion_strength)),
            ],
            Rule::Viscosity { radius, strength } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Pressure { radius, strength, target_density } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_target_density", prefix), UniformValue::F32(*target_density)),
            ],
            Rule::Magnetism { radius, strength, .. } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::SurfaceTension { radius, strength, threshold } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_threshold", prefix), UniformValue::F32(*threshold)),
            ],
            Rule::Avoid { radius, strength } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::Buoyancy { surface_y, density } => vec![
                (format!("{}_surface_y", prefix), UniformValue::F32(*surface_y)),
                (format!("{}_density", prefix), UniformValue::F32(*density)),
            ],
            Rule::Friction { ground_y, strength, threshold } => vec![
                (format!("{}_ground_y", prefix), UniformValue::F32(*ground_y)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_threshold", prefix), UniformValue::F32(*threshold)),
            ],
            Rule::Wind { direction, strength, turbulence } => vec![
                (format!("{}_direction", prefix), UniformValue::Vec3(*direction)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
                (format!("{}_turbulence", prefix), UniformValue::F32(*turbulence)),
            ],
            Rule::RespawnBelow { threshold_y, spawn_y, .. } => vec![
                (format!("{}_threshold_y", prefix), UniformValue::F32(*threshold_y)),
                (format!("{}_spawn_y", prefix), UniformValue::F32(*spawn_y)),
            ],
            Rule::Convert { from_type, trigger_type, to_type, radius, probability } => vec![
                (format!("{}_from_type", prefix), UniformValue::U32(*from_type)),
                (format!("{}_trigger_type", prefix), UniformValue::U32(*trigger_type)),
                (format!("{}_to_type", prefix), UniformValue::U32(*to_type)),
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_probability", prefix), UniformValue::F32(*probability)),
            ],
            Rule::FadeOut(duration) => vec![
                (format!("{}_duration", prefix), UniformValue::F32(*duration)),
            ],
            Rule::ShrinkOut(duration) => vec![
                (format!("{}_duration", prefix), UniformValue::F32(*duration)),
            ],
            Rule::ColorOverLife { start, end, duration } => vec![
                (format!("{}_start", prefix), UniformValue::Vec3(*start)),
                (format!("{}_end", prefix), UniformValue::Vec3(*end)),
                (format!("{}_duration", prefix), UniformValue::F32(*duration)),
            ],
            Rule::ColorBySpeed { slow_color, fast_color, max_speed } => vec![
                (format!("{}_slow_color", prefix), UniformValue::Vec3(*slow_color)),
                (format!("{}_fast_color", prefix), UniformValue::Vec3(*fast_color)),
                (format!("{}_max_speed", prefix), UniformValue::F32(*max_speed)),
            ],
            Rule::ColorByAge { young_color, old_color, max_age } => vec![
                (format!("{}_young_color", prefix), UniformValue::Vec3(*young_color)),
                (format!("{}_old_color", prefix), UniformValue::Vec3(*old_color)),
                (format!("{}_max_age", prefix), UniformValue::F32(*max_age)),
            ],
            Rule::ScaleBySpeed { min_scale, max_scale, max_speed } => vec![
                (format!("{}_min_scale", prefix), UniformValue::F32(*min_scale)),
                (format!("{}_max_scale", prefix), UniformValue::F32(*max_scale)),
                (format!("{}_max_speed", prefix), UniformValue::F32(*max_speed)),
            ],
            Rule::Flock { radius, separation, cohesion, alignment } => vec![
                (format!("{}_radius", prefix), UniformValue::F32(*radius)),
                (format!("{}_separation", prefix), UniformValue::F32(*separation)),
                (format!("{}_cohesion", prefix), UniformValue::F32(*cohesion)),
                (format!("{}_alignment", prefix), UniformValue::F32(*alignment)),
            ],
            Rule::Grow { rate, min, max } => vec![
                (format!("{}_rate", prefix), UniformValue::F32(*rate)),
                (format!("{}_min", prefix), UniformValue::F32(*min)),
                (format!("{}_max", prefix), UniformValue::F32(*max)),
            ],
            Rule::Gradient { field, strength, .. } => vec![
                (format!("{}_field", prefix), UniformValue::U32(*field)),
                (format!("{}_strength", prefix), UniformValue::F32(*strength)),
            ],
            Rule::ChainSprings { stiffness, damping, rest_length, .. } => vec![
                (format!("{}_stiffness", prefix), UniformValue::F32(*stiffness)),
                (format!("{}_damping", prefix), UniformValue::F32(*damping)),
                (format!("{}_rest_length", prefix), UniformValue::F32(*rest_length)),
            ],
            Rule::RadialSprings { hub_stiffness, ring_stiffness, damping, hub_length, ring_length } => vec![
                (format!("{}_hub_stiffness", prefix), UniformValue::F32(*hub_stiffness)),
                (format!("{}_ring_stiffness", prefix), UniformValue::F32(*ring_stiffness)),
                (format!("{}_damping", prefix), UniformValue::F32(*damping)),
                (format!("{}_hub_length", prefix), UniformValue::F32(*hub_length)),
                (format!("{}_ring_length", prefix), UniformValue::F32(*ring_length)),
            ],
            // Rules with no numeric params or complex types (strings, closures, etc.)
            // Use catch-all to handle any remaining rules not explicitly matched
            _ => vec![],
        }
    }

    /// Generate WGSL code that reads parameters from the uniforms struct.
    ///
    /// When rule inspector is enabled, rule parameters are stored as custom uniforms
    /// and can be modified at runtime through the inspector UI.
    /// For rules without dynamic support, falls back to static to_wgsl().
    pub fn to_wgsl_dynamic(&self, index: usize, bounds: f32) -> String {
        let prefix = format!("rule_{}", index);

        match self {
            Rule::Gravity(_) => format!(
                "    // Gravity (dynamic)\n    p.velocity.y -= uniforms.{prefix}_strength * uniforms.delta_time;"
            ),
            Rule::Drag(_) => format!(
                "    // Drag (dynamic)\n    p.velocity *= 1.0 - (uniforms.{prefix}_coefficient * uniforms.delta_time);"
            ),
            Rule::Acceleration(_) => format!(
                "    // Acceleration (dynamic)\n    p.velocity += uniforms.{prefix}_acceleration * uniforms.delta_time;"
            ),
            Rule::AttractTo { .. } => format!(
                r#"    // Attract to point (dynamic)
    {{
        let attract_dir = uniforms.{prefix}_point - p.position;
        let dist = length(attract_dir);
        if dist > 0.001 {{
            p.velocity += normalize(attract_dir) * uniforms.{prefix}_strength * uniforms.delta_time;
        }}
    }}"#
            ),
            Rule::RepelFrom { .. } => format!(
                r#"    // Repel from point (dynamic)
    {{
        let repel_dir = p.position - uniforms.{prefix}_point;
        let dist = length(repel_dir);
        if dist < uniforms.{prefix}_radius && dist > 0.001 {{
            let force = (uniforms.{prefix}_radius - dist) / uniforms.{prefix}_radius * uniforms.{prefix}_strength;
            p.velocity += normalize(repel_dir) * force * uniforms.delta_time;
        }}
    }}"#
            ),
            Rule::Vortex { .. } => format!(
                r#"    // Vortex (dynamic)
    {{
        let to_particle = p.position - uniforms.{prefix}_center;
        let axis_norm = normalize(uniforms.{prefix}_axis);
        let proj = dot(to_particle, axis_norm) * axis_norm;
        let radial = to_particle - proj;
        let dist = length(radial);
        if dist > 0.001 {{
            let tangent = cross(axis_norm, normalize(radial));
            p.velocity += tangent * uniforms.{prefix}_strength * uniforms.delta_time;
        }}
    }}"#
            ),
            Rule::Turbulence { .. } => format!(
                r#"    // Turbulence (dynamic)
    {{
        let noise_pos = p.position * uniforms.{prefix}_scale + vec3<f32>(uniforms.time * 0.5);
        let force = vec3<f32>(
            noise3(noise_pos),
            noise3(noise_pos + vec3<f32>(100.0, 0.0, 0.0)),
            noise3(noise_pos + vec3<f32>(0.0, 100.0, 0.0))
        );
        p.velocity += force * uniforms.{prefix}_strength * uniforms.delta_time;
    }}"#
            ),
            Rule::Orbit { .. } => format!(
                r#"    // Orbit (dynamic)
    {{
        let to_center = uniforms.{prefix}_center - p.position;
        let dist = length(to_center);
        if dist > 0.001 {{
            let tangent = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), to_center));
            p.velocity += tangent * uniforms.{prefix}_strength * uniforms.delta_time;
            p.velocity += normalize(to_center) * uniforms.{prefix}_strength * 0.1 * uniforms.delta_time;
        }}
    }}"#
            ),
            Rule::PointGravity { .. } => format!(
                r#"    // Point gravity (dynamic)
    {{
        let to_point = uniforms.{prefix}_point - p.position;
        let dist_sq = dot(to_point, to_point) + uniforms.{prefix}_softening * uniforms.{prefix}_softening;
        let force = uniforms.{prefix}_strength / dist_sq;
        p.velocity += normalize(to_point) * force * uniforms.delta_time;
    }}"#
            ),
            Rule::Spring { .. } => format!(
                r#"    // Spring (dynamic)
    {{
        let displacement = uniforms.{prefix}_anchor - p.position;
        let spring_force = displacement * uniforms.{prefix}_stiffness;
        let damping_force = -p.velocity * uniforms.{prefix}_damping;
        p.velocity += (spring_force + damping_force) * uniforms.delta_time;
    }}"#
            ),
            Rule::Pulse { .. } => format!(
                r#"    // Pulse (dynamic)
    {{
        let to_particle = p.position - uniforms.{prefix}_point;
        let dist = length(to_particle);
        if uniforms.{prefix}_radius <= 0.0 || dist < uniforms.{prefix}_radius {{
            let pulse = sin(uniforms.time * uniforms.{prefix}_frequency * 6.28318) * uniforms.{prefix}_strength;
            if dist > 0.001 {{
                p.velocity += normalize(to_particle) * pulse * uniforms.delta_time;
            }}
        }}
    }}"#
            ),
            Rule::SpeedLimit { .. } => format!(
                r#"    // Speed limit (dynamic)
    {{
        let speed = length(p.velocity);
        if speed > uniforms.{prefix}_max {{
            p.velocity = normalize(p.velocity) * uniforms.{prefix}_max;
        }} else if speed < uniforms.{prefix}_min && speed > 0.001 {{
            p.velocity = normalize(p.velocity) * uniforms.{prefix}_min;
        }}
    }}"#
            ),
            Rule::Lifetime(_) => format!(
                r#"    // Lifetime (dynamic)
    if p.age >= uniforms.{prefix}_duration {{
        p.alive = 0u;
    }}"#
            ),
            Rule::Curl { .. } => format!(
                r#"    // Curl noise (dynamic)
    {{
        let curl_pos = p.position * uniforms.{prefix}_scale + vec3<f32>(uniforms.time * 0.3);
        let eps = 0.01;
        let dx = noise3(curl_pos + vec3<f32>(eps, 0.0, 0.0)) - noise3(curl_pos - vec3<f32>(eps, 0.0, 0.0));
        let dy = noise3(curl_pos + vec3<f32>(0.0, eps, 0.0)) - noise3(curl_pos - vec3<f32>(0.0, eps, 0.0));
        let dz = noise3(curl_pos + vec3<f32>(0.0, 0.0, eps)) - noise3(curl_pos - vec3<f32>(0.0, 0.0, eps));
        let curl = vec3<f32>(dy - dz, dz - dx, dx - dy) / (2.0 * eps);
        p.velocity += curl * uniforms.{prefix}_strength * uniforms.delta_time;
    }}"#
            ),
            Rule::Oscillate { .. } => format!(
                r#"    // Oscillate (dynamic)
    {{
        let phase = uniforms.time * uniforms.{prefix}_frequency * 6.28318;
        var wave = sin(phase);
        if uniforms.{prefix}_spatial_scale > 0.0 {{
            let dist = length(p.position.xz);
            wave = sin(phase - dist * uniforms.{prefix}_spatial_scale);
        }}
        p.velocity += normalize(uniforms.{prefix}_axis) * wave * uniforms.{prefix}_amplitude * uniforms.delta_time;
    }}"#
            ),
            Rule::Wander { .. } => format!(
                r#"    // Wander (dynamic)
    {{
        let seed = f32(index) * 0.1 + uniforms.time * uniforms.{prefix}_frequency;
        let wander_dir = vec3<f32>(
            sin(seed * 1.1),
            sin(seed * 0.7 + 2.0),
            sin(seed * 0.9 + 4.0)
        );
        p.velocity += wander_dir * uniforms.{prefix}_strength * uniforms.delta_time;
    }}"#
            ),
            Rule::Seek { .. } => format!(
                r#"    // Seek (dynamic)
    {{
        let desired = normalize(uniforms.{prefix}_target - p.position) * uniforms.{prefix}_max_speed;
        var steer = desired - p.velocity;
        let steer_mag = length(steer);
        if steer_mag > uniforms.{prefix}_max_force {{
            steer = normalize(steer) * uniforms.{prefix}_max_force;
        }}
        p.velocity += steer * uniforms.delta_time;
    }}"#
            ),
            Rule::Flee { .. } => format!(
                r#"    // Flee (dynamic)
    {{
        let to_target = uniforms.{prefix}_target - p.position;
        let dist = length(to_target);
        if uniforms.{prefix}_panic_radius <= 0.0 || dist < uniforms.{prefix}_panic_radius {{
            let desired = normalize(-to_target) * uniforms.{prefix}_max_speed;
            var steer = desired - p.velocity;
            let steer_mag = length(steer);
            if steer_mag > uniforms.{prefix}_max_force {{
                steer = normalize(steer) * uniforms.{prefix}_max_force;
            }}
            p.velocity += steer * uniforms.delta_time;
        }}
    }}"#
            ),
            Rule::Arrive { .. } => format!(
                r#"    // Arrive (dynamic)
    {{
        let to_target = uniforms.{prefix}_target - p.position;
        let dist = length(to_target);
        var desired_speed = uniforms.{prefix}_max_speed;
        if dist < uniforms.{prefix}_slowing_radius {{
            desired_speed = uniforms.{prefix}_max_speed * (dist / uniforms.{prefix}_slowing_radius);
        }}
        let desired = normalize(to_target) * desired_speed;
        var steer = desired - p.velocity;
        let steer_mag = length(steer);
        if steer_mag > uniforms.{prefix}_max_force {{
            steer = normalize(steer) * uniforms.{prefix}_max_force;
        }}
        p.velocity += steer * uniforms.delta_time;
    }}"#
            ),
            Rule::Wind { .. } => format!(
                r#"    // Wind (dynamic)
    {{
        let wind_dir = normalize(uniforms.{prefix}_direction);
        var force = wind_dir * uniforms.{prefix}_strength;
        if uniforms.{prefix}_turbulence > 0.0 {{
            let noise_pos = p.position * 2.0 + vec3<f32>(uniforms.time);
            force += vec3<f32>(
                noise3(noise_pos),
                noise3(noise_pos + vec3<f32>(100.0, 0.0, 0.0)),
                noise3(noise_pos + vec3<f32>(0.0, 100.0, 0.0))
            ) * uniforms.{prefix}_turbulence * uniforms.{prefix}_strength;
        }}
        p.velocity += force * uniforms.delta_time;
    }}"#
            ),
            Rule::Buoyancy { .. } => format!(
                r#"    // Buoyancy (dynamic)
    {{
        if p.position.y < uniforms.{prefix}_surface_y {{
            let depth = uniforms.{prefix}_surface_y - p.position.y;
            p.velocity.y += depth * uniforms.{prefix}_density * uniforms.delta_time;
        }}
    }}"#
            ),
            Rule::Friction { .. } => format!(
                r#"    // Friction (dynamic)
    {{
        if p.position.y < uniforms.{prefix}_ground_y + uniforms.{prefix}_threshold {{
            let friction = uniforms.{prefix}_strength * uniforms.delta_time;
            p.velocity.x *= 1.0 - friction;
            p.velocity.z *= 1.0 - friction;
        }}
    }}"#
            ),
            Rule::RespawnBelow { reset_velocity, .. } => {
                let reset_code = if *reset_velocity {
                    "p.velocity = vec3<f32>(0.0);"
                } else {
                    ""
                };
                format!(
                    r#"    // Respawn below (dynamic)
    if p.position.y < uniforms.{prefix}_threshold_y {{
        p.position.y = uniforms.{prefix}_spawn_y;
        {reset_code}
    }}"#
                )
            },
            Rule::FadeOut(_) => format!(
                r#"    // Fade out (dynamic)
    {{
        let fade = max(0.0, 1.0 - p.age / uniforms.{prefix}_duration);
        p.color *= fade;
    }}"#
            ),
            Rule::ShrinkOut(_) => format!(
                r#"    // Shrink out (dynamic)
    {{
        p.scale = max(0.0, 1.0 - p.age / uniforms.{prefix}_duration);
    }}"#
            ),
            Rule::ColorOverLife { .. } => format!(
                r#"    // Color over life (dynamic)
    {{
        let t = clamp(p.age / uniforms.{prefix}_duration, 0.0, 1.0);
        p.color = mix(uniforms.{prefix}_start, uniforms.{prefix}_end, t);
    }}"#
            ),
            Rule::ColorBySpeed { .. } => format!(
                r#"    // Color by speed (dynamic)
    {{
        let speed = length(p.velocity);
        let t = clamp(speed / uniforms.{prefix}_max_speed, 0.0, 1.0);
        p.color = mix(uniforms.{prefix}_slow_color, uniforms.{prefix}_fast_color, t);
    }}"#
            ),
            Rule::ColorByAge { .. } => format!(
                r#"    // Color by age (dynamic)
    {{
        let t = clamp(p.age / uniforms.{prefix}_max_age, 0.0, 1.0);
        p.color = mix(uniforms.{prefix}_young_color, uniforms.{prefix}_old_color, t);
    }}"#
            ),
            Rule::ScaleBySpeed { .. } => format!(
                r#"    // Scale by speed (dynamic)
    {{
        let speed = length(p.velocity);
        let t = clamp(speed / uniforms.{prefix}_max_speed, 0.0, 1.0);
        p.scale = mix(uniforms.{prefix}_min_scale, uniforms.{prefix}_max_scale, t);
    }}"#
            ),
            Rule::Grow { .. } => format!(
                r#"    // Grow (dynamic)
    {{
        p.scale = clamp(p.scale + uniforms.{prefix}_rate * uniforms.delta_time, uniforms.{prefix}_min, uniforms.{prefix}_max);
    }}"#
            ),
            Rule::PositionNoise { .. } => format!(
                r#"    // Position noise (dynamic)
    {{
        let noise_pos = p.position * uniforms.{prefix}_scale + vec3<f32>(uniforms.time * uniforms.{prefix}_speed);
        let jitter = vec3<f32>(
            noise3(noise_pos) - 0.5,
            noise3(noise_pos + vec3<f32>(100.0, 0.0, 0.0)) - 0.5,
            noise3(noise_pos + vec3<f32>(0.0, 100.0, 0.0)) - 0.5
        ) * 2.0;
        p.position += jitter * uniforms.{prefix}_strength;
    }}"#
            ),
            Rule::Shockwave { .. } => format!(
                r#"    // Shockwave (dynamic)
    {{
        var t = uniforms.time;
        if uniforms.{prefix}_repeat > 0.0 {{
            t = fract(t / uniforms.{prefix}_repeat) * uniforms.{prefix}_repeat;
        }}
        let wave_radius = t * uniforms.{prefix}_speed;
        let to_particle = p.position - uniforms.{prefix}_origin;
        let dist = length(to_particle);
        let wave_dist = abs(dist - wave_radius);
        if wave_dist < uniforms.{prefix}_width {{
            let force = (1.0 - wave_dist / uniforms.{prefix}_width) * uniforms.{prefix}_strength;
            if dist > 0.001 {{
                p.velocity += normalize(to_particle) * force * uniforms.delta_time;
            }}
        }}
    }}"#
            ),
            Rule::Radial { falloff, .. } => {
                let falloff_code = match falloff {
                    Falloff::Constant => "let falloff = 1.0;",
                    Falloff::Linear => "let falloff = 1.0 - dist / uniforms.{prefix}_radius;",
                    Falloff::Inverse => "let falloff = 1.0 / (dist + 0.01);",
                    Falloff::InverseSquare => "let falloff = 1.0 / (dist * dist + 0.01);",
                    Falloff::Smooth => "let t = dist / uniforms.{prefix}_radius; let falloff = 1.0 - t * t * (3.0 - 2.0 * t);",
                };
                format!(
                    r#"    // Radial (dynamic)
    {{
        let to_particle = p.position - uniforms.{prefix}_point;
        let dist = length(to_particle);
        if uniforms.{prefix}_radius <= 0.0 || dist < uniforms.{prefix}_radius {{
            {falloff_code}
            if dist > 0.001 {{
                p.velocity += normalize(to_particle) * uniforms.{prefix}_strength * falloff * uniforms.delta_time;
            }}
        }}
    }}"#
                )
            },
            // Fall back to static for complex rules or rules without dynamic support
            _ => self.to_wgsl(bounds),
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
