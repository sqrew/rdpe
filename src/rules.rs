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

    /// Particle-particle collision response.
    ///
    /// **Requires spatial hashing.** Particles within `radius` of each other
    /// are pushed apart. Creates solid, non-overlapping particles.
    ///
    /// # Fields
    ///
    /// - `radius` - Collision distance (particle "size")
    /// - `response` - Push strength (0.5 = gentle, 1.0+ = bouncy)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_spatial_config(0.1, 32)
    /// .with_rule(Rule::Collide {
    ///     radius: 0.05,          // Particle diameter
    ///     response: 0.5,         // Gentle collision
    /// })
    /// ```
    Collide {
        /// Collision distance.
        radius: f32,
        /// Push strength.
        response: f32,
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
    ///     rule: Box::new(Rule::Collide { radius: 0.05, response: 0.5 }),
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
            | Rule::NBodyGravity { .. }
            | Rule::Viscosity { .. }
            | Rule::Pressure { .. }
            | Rule::Magnetism { .. }
            | Rule::SurfaceTension { .. }
            | Rule::Avoid { .. }
            | Rule::Separate { .. }
            | Rule::Cohere { .. }
            | Rule::Align { .. }
            | Rule::Convert { .. }
            | Rule::Chase { .. }
            | Rule::Evade { .. }
            | Rule::NeighborCustom(_) => true,
            Rule::Typed { rule, .. } => rule.requires_neighbors(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses cohesion accumulators.
    pub(crate) fn needs_cohesion_accumulator(&self) -> bool {
        match self {
            Rule::Cohere { .. } => true,
            Rule::Typed { rule, .. } => rule.needs_cohesion_accumulator(),
            _ => false,
        }
    }

    /// Returns `true` if this rule uses alignment accumulators.
    pub(crate) fn needs_alignment_accumulator(&self) -> bool {
        match self {
            Rule::Align { .. } => true,
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

            // Neighbor rules generate code through to_neighbor_wgsl
            Rule::Collide { .. }
            | Rule::NBodyGravity { .. }
            | Rule::Viscosity { .. }
            | Rule::Pressure { .. }
            | Rule::Magnetism { .. }
            | Rule::SurfaceTension { .. }
            | Rule::Avoid { .. }
            | Rule::Separate { .. }
            | Rule::Cohere { .. }
            | Rule::Align { .. }
            | Rule::Typed { .. }
            | Rule::Convert { .. }
            | Rule::Chase { .. }
            | Rule::Evade { .. }
            | Rule::NeighborCustom(_) => String::new(),
        }
    }

    /// Generate WGSL code for neighbor-based rules (inside neighbor loop).
    pub fn to_neighbor_wgsl(&self) -> String {
        match self {
            Rule::Collide { radius, response } => format!(
                r#"            // Collision
            if neighbor_dist < {radius} && neighbor_dist > 0.0001 {{
                let overlap = {radius} - neighbor_dist;
                let push = neighbor_dir * (overlap * {response});
                p.velocity += push;
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
            Rule::Collide { radius: 0.1, response: 0.5 },
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
