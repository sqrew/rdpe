//! # Cosmic Jellyfish
//!
//! A strange, pulsating entity made of particles that flow like tentacles,
//! breathe with a cosmic heartbeat, and shimmer with interdimensional colors.
//!
//! This showcases the creative potential of combining:
//! - Custom WGSL functions for complex behaviors
//! - Time-based uniforms for rhythmic effects
//! - Type interactions for emergent structure
//! - Multiple forces creating organic motion
//!
//! Run with: `cargo run --example cosmic_jellyfish`

use rand::Rng;
use rdpe::prelude::*;

// Three "organs" of the jellyfish
#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
enum Organ {
    Core = 0,      // Central pulsing mass
    Tentacle = 1,  // Flowing tendrils
    Spark = 2,     // Bioluminescent particles
}

impl From<Organ> for u32 {
    fn from(o: Organ) -> u32 {
        o as u32
    }
}

#[derive(Particle, Clone)]
struct Cell {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Cell> = (0..20_000)
        .map(|i| {
            let organ = if i < 2000 {
                Organ::Core
            } else if i < 15000 {
                Organ::Tentacle
            } else {
                Organ::Spark
            };

            // Core particles start in a tight ball
            // Tentacles start below, streaming down
            // Sparks scattered around
            let (pos, vel) = match organ {
                Organ::Core => {
                    let theta = rng.gen_range(0.0..std::f32::consts::TAU);
                    let phi = rng.gen_range(0.0..std::f32::consts::PI);
                    let r = rng.gen_range(0.0..0.15);
                    (
                        Vec3::new(
                            r * phi.sin() * theta.cos(),
                            r * phi.cos() + 0.3,
                            r * phi.sin() * theta.sin(),
                        ),
                        Vec3::ZERO,
                    )
                }
                Organ::Tentacle => {
                    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                    let r = rng.gen_range(0.05..0.2);
                    let y = rng.gen_range(-0.5..0.2);
                    (
                        Vec3::new(angle.cos() * r, y, angle.sin() * r),
                        Vec3::new(0.0, -0.1, 0.0),
                    )
                }
                Organ::Spark => (
                    Vec3::new(
                        rng.gen_range(-0.4..0.4),
                        rng.gen_range(-0.3..0.5),
                        rng.gen_range(-0.4..0.4),
                    ),
                    Vec3::ZERO,
                ),
            };

            Cell {
                position: pos,
                velocity: vel,
                particle_type: organ.into(),
                color: Vec3::new(1.0, 1.0, 1.0), // Will be set by shader
            }
        })
        .collect();

    Simulation::<Cell>::new()
        .with_particle_count(20_000)
        .with_particle_size(0.012)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())

        // === Custom Functions ===

        // Organic pulsing based on distance and time
        .with_function(r#"
            fn cosmic_pulse(pos: vec3<f32>, time: f32, freq: f32, phase: f32) -> f32 {
                let dist = length(pos);
                return sin(time * freq + dist * 10.0 + phase) * 0.5 + 0.5;
            }
        "#)

        // Tentacle flow - spiraling downward motion
        .with_function(r#"
            fn tentacle_flow(pos: vec3<f32>, time: f32) -> vec3<f32> {
                let angle = atan2(pos.z, pos.x);
                let r = length(vec2<f32>(pos.x, pos.z));

                // Spiral component
                let spiral = vec3<f32>(
                    -sin(angle + time * 0.5) * 0.3,
                    -0.5 - pos.y * 0.3,
                    cos(angle + time * 0.5) * 0.3
                );

                // Wave component
                let wave = sin(pos.y * 8.0 - time * 3.0) * 0.2;

                return spiral + vec3<f32>(wave * cos(angle), 0.0, wave * sin(angle));
            }
        "#)

        // Interdimensional color based on position and phase
        .with_function(r#"
            fn cosmic_color(pos: vec3<f32>, time: f32, ptype: u32) -> vec3<f32> {
                let pulse = sin(time * 2.0 + length(pos) * 5.0) * 0.5 + 0.5;

                if ptype == 0u {
                    // Core: deep purple to bright cyan pulse
                    let hue = 0.75 + pulse * 0.15;
                    return hsv_to_rgb(hue, 0.8, 0.7 + pulse * 0.3);
                } else if ptype == 1u {
                    // Tentacles: flowing blue to magenta gradient
                    let depth = (pos.y + 1.0) * 0.5;
                    let hue = 0.55 + depth * 0.25 + sin(time + pos.y * 3.0) * 0.1;
                    return hsv_to_rgb(hue, 0.7, 0.5 + pulse * 0.3);
                } else {
                    // Sparks: bright shifting colors
                    let hue = fract(time * 0.1 + length(pos));
                    return hsv_to_rgb(hue, 0.6, 0.9);
                }
            }
        "#)

        // === Core Behavior ===
        // Core particles pulse and orbit
        .with_rule(Rule::Custom(r#"
            if p.particle_type == 0u {
                // Pulsing expansion/contraction
                let pulse = cosmic_pulse(p.position, uniforms.time, 2.0, 0.0);
                let to_center = vec3<f32>(0.0, 0.3, 0.0) - p.position;
                let dist = length(to_center);

                // Breathe in and out
                let breath_force = select(1.5, -0.8, pulse > 0.5);
                if dist > 0.01 {
                    p.velocity += normalize(to_center) * breath_force * uniforms.delta_time;
                }

                // Gentle orbit
                p.velocity.x += -p.position.z * 0.5 * uniforms.delta_time;
                p.velocity.z += p.position.x * 0.5 * uniforms.delta_time;
            }
        "#.into()))

        // === Tentacle Behavior ===
        // Tentacles flow and undulate
        .with_rule(Rule::Custom(r#"
            if p.particle_type == 1u {
                let flow = tentacle_flow(p.position, uniforms.time);
                p.velocity += flow * uniforms.delta_time;

                // Pull toward center axis
                let to_axis = vec3<f32>(-p.position.x, 0.0, -p.position.z);
                p.velocity += to_axis * 0.3 * uniforms.delta_time;

                // Respawn if too far down
                if p.position.y < -1.0 {
                    // Teleport back to near core
                    let angle = noise3(p.position * 100.0) * 6.28;
                    p.position = vec3<f32>(cos(angle) * 0.1, 0.2, sin(angle) * 0.1);
                    p.velocity = vec3<f32>(0.0, -0.2, 0.0);
                }
            }
        "#.into()))

        // === Spark Behavior ===
        // Sparks orbit and are attracted to both core and tentacles
        .with_rule(Rule::Custom(r#"
            if p.particle_type == 2u {
                // Orbit around Y axis
                p.velocity.x += -p.position.z * 2.0 * uniforms.delta_time;
                p.velocity.z += p.position.x * 2.0 * uniforms.delta_time;

                // Attracted to core
                let to_core = vec3<f32>(0.0, 0.3, 0.0) - p.position;
                p.velocity += normalize(to_core) * 0.5 * uniforms.delta_time;

                // Random jitter
                let jitter = vec3<f32>(
                    noise3(p.position * 10.0 + uniforms.time),
                    noise3(p.position * 10.0 + uniforms.time + 100.0),
                    noise3(p.position * 10.0 + uniforms.time + 200.0)
                );
                p.velocity += jitter * 0.5 * uniforms.delta_time;
            }
        "#.into()))

        // === Color Update ===
        .with_rule(Rule::Custom(r#"
            p.color = cosmic_color(p.position, uniforms.time, p.particle_type);
        "#.into()))

        // === Physics ===
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.5 })
        .with_rule(Rule::WrapWalls)
        .run();
}
