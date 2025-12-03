//! Particle emitters for runtime spawning.
//!
//! Emitters continuously spawn new particles into dead slots in the particle pool.
//! Multiple emitter types are available for different spawning patterns.
//!
//! # Emitter Types
//!
//! | Type | Description |
//! |------|-------------|
//! | [`Emitter::Point`] | Single point, omnidirectional or directional |
//! | [`Emitter::Burst`] | One-time explosion of particles |
//! | [`Emitter::Cone`] | Directional cone emission |
//! | [`Emitter::Sphere`] | Spawn on sphere surface |
//! | [`Emitter::Box`] | Spawn within a box volume |
//!
//! # Velocity Control
//!
//! Most emitters support velocity configuration:
//! - `direction` - The primary direction of emission (normalized)
//! - `speed` - Base speed of emitted particles
//! - `spread` - Angular spread in radians (0 = laser, PI = hemisphere)
//!
//! # Example
//!
//! ```ignore
//! // Fountain shooting upward
//! .with_emitter(Emitter::Cone {
//!     position: Vec3::new(0.0, -0.5, 0.0),
//!     direction: Vec3::Y,
//!     speed: 2.0,
//!     spread: 0.3,
//!     rate: 1000.0,
//! })
//! ```

use glam::Vec3;

/// Particle emitter configuration.
///
/// Emitters respawn dead particles at a configurable rate. They run
/// as part of the compute shader, finding dead particle slots and
/// reinitializing them.
///
/// # Example
///
/// ```ignore
/// Simulation::<Spark>::new()
///     .with_particle_count(10_000)
///     .with_emitter(Emitter::Point {
///         position: Vec3::ZERO,
///         rate: 500.0,
///     })
///     .with_rule(Rule::Age)
///     .with_rule(Rule::Lifetime(2.0))
///     .with_rule(Rule::Gravity(9.8))
///     .run();
/// ```
#[derive(Clone, Debug)]
pub enum Emitter {
    /// Emit particles from a single point with random directions.
    ///
    /// Simple omnidirectional emitter - particles fly out in all directions.
    ///
    /// # Fields
    ///
    /// - `position` - Spawn location
    /// - `rate` - Particles per second
    /// - `speed` - Initial speed of particles (default behavior if 0: random 0-0.5)
    Point {
        /// Spawn position.
        position: Vec3,
        /// Emission rate (particles per second).
        rate: f32,
        /// Initial speed of particles. If 0, uses random velocity.
        speed: f32,
    },

    /// One-time burst of particles (explosion effect).
    ///
    /// Spawns `count` particles in a single frame, then stops.
    /// Useful for explosions, impacts, or other instantaneous effects.
    ///
    /// # Fields
    ///
    /// - `position` - Explosion center
    /// - `count` - Number of particles to spawn
    /// - `speed` - Initial outward speed
    /// - `triggered` - Set to `true` to fire (resets after burst)
    Burst {
        /// Explosion center.
        position: Vec3,
        /// Number of particles to spawn.
        count: u32,
        /// Initial outward speed.
        speed: f32,
    },

    /// Directional cone emitter.
    ///
    /// Emits particles in a cone shape, useful for fountains, jets, thrusters.
    ///
    /// # Fields
    ///
    /// - `position` - Spawn location
    /// - `direction` - Primary emission direction (will be normalized)
    /// - `speed` - Initial particle speed
    /// - `spread` - Cone half-angle in radians (0 = laser, PI/2 = hemisphere)
    /// - `rate` - Particles per second
    Cone {
        /// Spawn position.
        position: Vec3,
        /// Primary emission direction.
        direction: Vec3,
        /// Initial speed of particles.
        speed: f32,
        /// Cone half-angle in radians.
        spread: f32,
        /// Emission rate (particles per second).
        rate: f32,
    },

    /// Spawn particles on a sphere surface.
    ///
    /// Particles spawn on the surface and move outward (or inward if speed < 0).
    ///
    /// # Fields
    ///
    /// - `center` - Sphere center
    /// - `radius` - Sphere radius
    /// - `speed` - Outward speed (negative = inward)
    /// - `rate` - Particles per second
    Sphere {
        /// Sphere center.
        center: Vec3,
        /// Sphere radius.
        radius: f32,
        /// Outward speed (negative for inward).
        speed: f32,
        /// Emission rate (particles per second).
        rate: f32,
    },

    /// Spawn particles within a box volume.
    ///
    /// Particles spawn at random positions within the box with optional initial velocity.
    ///
    /// # Fields
    ///
    /// - `min` - Minimum corner of the box
    /// - `max` - Maximum corner of the box
    /// - `velocity` - Initial velocity for all spawned particles
    /// - `rate` - Particles per second
    Box {
        /// Minimum corner of the box.
        min: Vec3,
        /// Maximum corner of the box.
        max: Vec3,
        /// Initial velocity.
        velocity: Vec3,
        /// Emission rate (particles per second).
        rate: f32,
    },
}

impl Emitter {
    /// Get the emission rate in particles per second.
    ///
    /// For `Burst` emitters, returns the count as a one-time rate.
    pub fn rate(&self) -> f32 {
        match self {
            Emitter::Point { rate, .. } => *rate,
            Emitter::Burst { count, .. } => *count as f32,
            Emitter::Cone { rate, .. } => *rate,
            Emitter::Sphere { rate, .. } => *rate,
            Emitter::Box { rate, .. } => *rate,
        }
    }

    /// Generate WGSL code for emitter logic.
    ///
    /// This code runs at the start of the compute shader for each particle.
    /// Dead particles have a chance to be respawned based on the rate.
    pub(crate) fn to_wgsl(&self, emitter_index: usize) -> String {
        match self {
            Emitter::Point { position, rate, speed } => {
                let speed_code = if *speed > 0.0 {
                    format!(
                        r#"let vel_dir = normalize(vec3<f32>(vx, vy, vz));
            p.velocity = vel_dir * {speed};"#
                    )
                } else {
                    "p.velocity = vec3<f32>(vx, vy, vz) * 0.5;".to_string()
                };

                format!(
                    r#"    // Point emitter {emitter_index} at ({}, {}, {})
    if p.alive == 0u {{
        let spawn_hash = (index * 1103515245u + u32(uniforms.time * 10000.0) + {emitter_index}u * 7919u) ^ (index >> 3u);
        let spawn_chance = f32(spawn_hash & 0xFFFFu) / 65535.0;
        let spawn_rate = {rate} * uniforms.delta_time / f32(num_particles);

        if spawn_chance < spawn_rate {{
            p.alive = 1u;
            p.age = 0.0;
            p.scale = 1.0;
            p.particle_type = 0u;
            p.position = vec3<f32>({}, {}, {});

            // Random direction
            let vhash = spawn_hash * 0x45d9f3bu;
            let vx = f32((vhash >> 0u) & 0xFFu) / 128.0 - 1.0;
            let vy = f32((vhash >> 8u) & 0xFFu) / 128.0 - 1.0;
            let vz = f32((vhash >> 16u) & 0xFFu) / 128.0 - 1.0;
            {speed_code}
        }}
    }}"#,
                    position.x, position.y, position.z,
                    position.x, position.y, position.z,
                )
            }

            Emitter::Burst { position, count, speed } => {
                format!(
                    r#"    // Burst emitter {emitter_index} at ({}, {}, {})
    // Fires once at time ~0, spawning first {count} particles
    if index < {count}u && uniforms.time < 0.1 {{
        p.alive = 1u;
        p.age = 0.0;
        p.scale = 1.0;
        p.particle_type = 0u;
        p.position = vec3<f32>({}, {}, {});

        // Random outward direction (uniform on sphere)
        let vhash = index * 2654435761u;
        let theta = f32((vhash >> 0u) & 0xFFFFu) / 65535.0 * 6.28318;
        let phi = acos(f32((vhash >> 16u) & 0xFFFFu) / 65535.0 * 2.0 - 1.0);
        let dir = vec3<f32>(
            sin(phi) * cos(theta),
            sin(phi) * sin(theta),
            cos(phi)
        );
        p.velocity = dir * {speed};
    }}"#,
                    position.x, position.y, position.z,
                    position.x, position.y, position.z,
                )
            }

            Emitter::Cone { position, direction, speed, spread, rate } => {
                let dir = direction.normalize();
                format!(
                    r#"    // Cone emitter {emitter_index} at ({}, {}, {}) dir ({}, {}, {})
    if p.alive == 0u {{
        let spawn_hash = (index * 1103515245u + u32(uniforms.time * 10000.0) + {emitter_index}u * 7919u) ^ (index >> 3u);
        let spawn_chance = f32(spawn_hash & 0xFFFFu) / 65535.0;
        let spawn_rate = {rate} * uniforms.delta_time / f32(num_particles);

        if spawn_chance < spawn_rate {{
            p.alive = 1u;
            p.age = 0.0;
            p.scale = 1.0;
            p.particle_type = 0u;
            p.position = vec3<f32>({}, {}, {});

            // Cone direction with spread
            let base_dir = vec3<f32>({}, {}, {});
            let vhash = spawn_hash * 0x45d9f3bu;

            // Random angle within spread cone
            let rand_angle = f32((vhash >> 0u) & 0xFFFFu) / 65535.0 * 6.28318;
            let rand_spread = f32((vhash >> 16u) & 0xFFFFu) / 65535.0 * {spread};

            // Create perpendicular vectors for rotation
            let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(base_dir.y) > 0.9);
            let right = normalize(cross(up, base_dir));
            let forward = cross(base_dir, right);

            // Apply spread
            let spread_x = sin(rand_spread) * cos(rand_angle);
            let spread_y = sin(rand_spread) * sin(rand_angle);
            let spread_z = cos(rand_spread);
            let dir = normalize(right * spread_x + forward * spread_y + base_dir * spread_z);

            p.velocity = dir * {speed};
        }}
    }}"#,
                    position.x, position.y, position.z,
                    dir.x, dir.y, dir.z,
                    position.x, position.y, position.z,
                    dir.x, dir.y, dir.z,
                )
            }

            Emitter::Sphere { center, radius, speed, rate } => {
                format!(
                    r#"    // Sphere emitter {emitter_index} center ({}, {}, {}) radius {}
    if p.alive == 0u {{
        let spawn_hash = (index * 1103515245u + u32(uniforms.time * 10000.0) + {emitter_index}u * 7919u) ^ (index >> 3u);
        let spawn_chance = f32(spawn_hash & 0xFFFFu) / 65535.0;
        let spawn_rate = {rate} * uniforms.delta_time / f32(num_particles);

        if spawn_chance < spawn_rate {{
            p.alive = 1u;
            p.age = 0.0;
            p.scale = 1.0;
            p.particle_type = 0u;

            // Random point on sphere surface
            let vhash = spawn_hash * 0x45d9f3bu;
            let theta = f32((vhash >> 0u) & 0xFFFFu) / 65535.0 * 6.28318;
            let phi = acos(f32((vhash >> 16u) & 0xFFFFu) / 65535.0 * 2.0 - 1.0);
            let dir = vec3<f32>(
                sin(phi) * cos(theta),
                sin(phi) * sin(theta),
                cos(phi)
            );

            p.position = vec3<f32>({}, {}, {}) + dir * {radius};
            p.velocity = dir * {speed};
        }}
    }}"#,
                    center.x, center.y, center.z, radius,
                    center.x, center.y, center.z,
                )
            }

            Emitter::Box { min, max, velocity, rate } => {
                format!(
                    r#"    // Box emitter {emitter_index} from ({}, {}, {}) to ({}, {}, {})
    if p.alive == 0u {{
        let spawn_hash = (index * 1103515245u + u32(uniforms.time * 10000.0) + {emitter_index}u * 7919u) ^ (index >> 3u);
        let spawn_chance = f32(spawn_hash & 0xFFFFu) / 65535.0;
        let spawn_rate = {rate} * uniforms.delta_time / f32(num_particles);

        if spawn_chance < spawn_rate {{
            p.alive = 1u;
            p.age = 0.0;
            p.scale = 1.0;
            p.particle_type = 0u;

            // Random position within box
            let vhash = spawn_hash * 0x45d9f3bu;
            let rx = f32((vhash >> 0u) & 0xFFu) / 255.0;
            let ry = f32((vhash >> 8u) & 0xFFu) / 255.0;
            let rz = f32((vhash >> 16u) & 0xFFu) / 255.0;

            p.position = vec3<f32>(
                mix({}, {}, rx),
                mix({}, {}, ry),
                mix({}, {}, rz)
            );
            p.velocity = vec3<f32>({}, {}, {});
        }}
    }}"#,
                    min.x, min.y, min.z,
                    max.x, max.y, max.z,
                    min.x, max.x,
                    min.y, max.y,
                    min.z, max.z,
                    velocity.x, velocity.y, velocity.z,
                )
            }
        }
    }
}
