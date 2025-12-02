//! Particle behavior rules.
//!
//! Rules define how particles behave each frame. They are applied in order
//! and translated into WGSL compute shader code at simulation startup.
//!
//! # Rule Categories
//!
//! - **Physics**: Basic forces like gravity and drag
//! - **Boundaries**: Wall collision and wrapping
//! - **Forces**: Point attractors and repellers
//! - **Movement**: Wandering and speed limits
//! - **Flocking**: Separation, cohesion, alignment (boids)
//! - **Types**: Type-filtered interactions, conversion, chase/evade
//! - **Custom**: Raw WGSL for anything else
//!
//! # Neighbor Rules
//!
//! Rules that query nearby particles (Separate, Cohere, Align, Collide,
//! Chase, Evade, Convert) require spatial hashing. Configure with:
//!
//! ```ignore
//! .with_spatial_config(cell_size, grid_resolution)
//! ```

use glam::Vec3;

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
    /// custom behavior, wrap built-in neighbor rules with [`Rule::Typed`].
    Custom(String),

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
}

impl Rule {
    /// Returns `true` if this rule requires spatial hashing.
    ///
    /// Neighbor-based rules (Collide, Separate, Cohere, Align, Convert,
    /// Chase, Evade) need spatial hashing enabled via `with_spatial_config()`.
    pub fn requires_neighbors(&self) -> bool {
        match self {
            Rule::Collide { .. }
            | Rule::Separate { .. }
            | Rule::Cohere { .. }
            | Rule::Align { .. }
            | Rule::Convert { .. }
            | Rule::Chase { .. }
            | Rule::Evade { .. } => true,
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

    /// Generate WGSL code for non-neighbor rules.
    pub(crate) fn to_wgsl(&self, bounds: f32) -> String {
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
            let clamped_speed = clamp(speed, {min}, {max});
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

            // Neighbor rules generate code through to_neighbor_wgsl
            Rule::Collide { .. }
            | Rule::Separate { .. }
            | Rule::Cohere { .. }
            | Rule::Align { .. }
            | Rule::Typed { .. }
            | Rule::Convert { .. }
            | Rule::Chase { .. }
            | Rule::Evade { .. } => String::new(),
        }
    }

    /// Generate WGSL code for neighbor-based rules (inside neighbor loop).
    pub(crate) fn to_neighbor_wgsl(&self) -> String {
        match self {
            Rule::Collide { radius, response } => format!(
                r#"            // Collision
            if neighbor_dist < {radius} && neighbor_dist > 0.0001 {{
                let overlap = {radius} - neighbor_dist;
                let push = neighbor_dir * (overlap * {response});
                p.velocity += push;
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

            _ => String::new(),
        }
    }

    /// Generate post-neighbor-loop WGSL (for averaging rules).
    pub(crate) fn to_post_neighbor_wgsl(&self) -> String {
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

            _ => String::new(),
        }
    }
}
