//! Emitter configuration types for runtime particle spawning.
//!
//! Emitters continuously spawn new particles into dead slots in the particle pool.

use serde::{Deserialize, Serialize};

/// Emitter configuration for runtime particle spawning.
///
/// Emitters respawn dead particles at a configurable rate. They run
/// as part of the compute shader, finding dead particle slots and
/// reinitializing them.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EmitterConfig {
    /// Emit particles from a single point with random directions.
    Point {
        /// Spawn position [x, y, z].
        position: [f32; 3],
        /// Emission rate (particles per second).
        rate: f32,
        /// Initial speed of particles. If 0, uses random velocity.
        #[serde(default)]
        speed: f32,
    },

    /// One-time burst of particles (explosion effect).
    Burst {
        /// Explosion center [x, y, z].
        position: [f32; 3],
        /// Number of particles to spawn.
        count: u32,
        /// Initial outward speed.
        speed: f32,
    },

    /// Directional cone emitter.
    Cone {
        /// Spawn position [x, y, z].
        position: [f32; 3],
        /// Primary emission direction [x, y, z] (will be normalized).
        direction: [f32; 3],
        /// Initial speed of particles.
        speed: f32,
        /// Cone half-angle in radians (0 = laser, PI/2 = hemisphere).
        spread: f32,
        /// Emission rate (particles per second).
        rate: f32,
    },

    /// Spawn particles on a sphere surface.
    Sphere {
        /// Sphere center [x, y, z].
        center: [f32; 3],
        /// Sphere radius.
        radius: f32,
        /// Outward speed (negative for inward).
        speed: f32,
        /// Emission rate (particles per second).
        rate: f32,
    },

    /// Spawn particles within a box volume.
    Box {
        /// Minimum corner of the box [x, y, z].
        min: [f32; 3],
        /// Maximum corner of the box [x, y, z].
        max: [f32; 3],
        /// Initial velocity [x, y, z].
        velocity: [f32; 3],
        /// Emission rate (particles per second).
        rate: f32,
    },
}

impl EmitterConfig {
    /// Get the emission rate in particles per second.
    pub fn rate(&self) -> f32 {
        match self {
            EmitterConfig::Point { rate, .. } => *rate,
            EmitterConfig::Burst { count, .. } => *count as f32,
            EmitterConfig::Cone { rate, .. } => *rate,
            EmitterConfig::Sphere { rate, .. } => *rate,
            EmitterConfig::Box { rate, .. } => *rate,
        }
    }

    /// Get a human-readable name for this emitter type.
    pub fn type_name(&self) -> &'static str {
        match self {
            EmitterConfig::Point { .. } => "Point",
            EmitterConfig::Burst { .. } => "Burst",
            EmitterConfig::Cone { .. } => "Cone",
            EmitterConfig::Sphere { .. } => "Sphere",
            EmitterConfig::Box { .. } => "Box",
        }
    }

    /// Get available emitter type names.
    pub fn type_variants() -> &'static [&'static str] {
        &["Point", "Burst", "Cone", "Sphere", "Box"]
    }

    /// Create a default emitter of the given type.
    pub fn default_of_type(type_name: &str) -> Self {
        match type_name {
            "Point" => EmitterConfig::Point {
                position: [0.0, 0.0, 0.0],
                rate: 100.0,
                speed: 0.5,
            },
            "Burst" => EmitterConfig::Burst {
                position: [0.0, 0.0, 0.0],
                count: 100,
                speed: 1.0,
            },
            "Cone" => EmitterConfig::Cone {
                position: [0.0, -0.5, 0.0],
                direction: [0.0, 1.0, 0.0],
                speed: 1.0,
                spread: 0.3,
                rate: 200.0,
            },
            "Sphere" => EmitterConfig::Sphere {
                center: [0.0, 0.0, 0.0],
                radius: 0.5,
                speed: 0.5,
                rate: 100.0,
            },
            "Box" => EmitterConfig::Box {
                min: [-0.5, -0.5, -0.5],
                max: [0.5, 0.5, 0.5],
                velocity: [0.0, 0.0, 0.0],
                rate: 100.0,
            },
            _ => EmitterConfig::Point {
                position: [0.0, 0.0, 0.0],
                rate: 100.0,
                speed: 0.5,
            },
        }
    }

    /// Generate WGSL code for this emitter.
    pub fn to_wgsl(&self, emitter_index: usize) -> String {
        match self {
            EmitterConfig::Point { position, rate, speed } => {
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
                    position[0], position[1], position[2],
                    position[0], position[1], position[2],
                )
            }

            EmitterConfig::Burst { position, count, speed } => {
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
                    position[0], position[1], position[2],
                    position[0], position[1], position[2],
                )
            }

            EmitterConfig::Cone { position, direction, speed, spread, rate } => {
                // Normalize direction
                let len = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
                let dir = if len > 0.0 {
                    [direction[0] / len, direction[1] / len, direction[2] / len]
                } else {
                    [0.0, 1.0, 0.0]
                };

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
                    position[0], position[1], position[2],
                    dir[0], dir[1], dir[2],
                    position[0], position[1], position[2],
                    dir[0], dir[1], dir[2],
                )
            }

            EmitterConfig::Sphere { center, radius, speed, rate } => {
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
                    center[0], center[1], center[2], radius,
                    center[0], center[1], center[2],
                )
            }

            EmitterConfig::Box { min, max, velocity, rate } => {
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
                    min[0], min[1], min[2],
                    max[0], max[1], max[2],
                    min[0], max[0],
                    min[1], max[1],
                    min[2], max[2],
                    velocity[0], velocity[1], velocity[2],
                )
            }
        }
    }
}

impl Default for EmitterConfig {
    fn default() -> Self {
        EmitterConfig::Point {
            position: [0.0, 0.0, 0.0],
            rate: 100.0,
            speed: 0.5,
        }
    }
}
