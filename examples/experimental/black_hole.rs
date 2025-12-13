//! # Black Hole
//!
//! An accretion disk of particles spiraling into oblivion, with relativistic
//! jets shooting out from the poles. Watch matter fall past the event horizon
//! and get spaghettified.
//!
//! Features:
//! - Gravitational pull with inverse-square falloff
//! - Accretion disk orbital mechanics
//! - Polar jets ejecting high-energy particles
//! - Color based on velocity (doppler-ish effect)
//! - Particles "die" when crossing the event horizon
//!
//! Run with: `cargo run --example black_hole`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
#[allow(dead_code)] // Variants reserved for future use
enum Matter {
    Disk = 0,      // Orbiting accretion disk
    Infalling = 1, // Falling into the hole
    Jet = 2,       // Ejected polar jets
}

impl From<Matter> for u32 {
    fn from(m: Matter) -> u32 {
        m as u32
    }
}

#[derive(Particle, Clone)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Particle> = (0..25_000)
        .map(|i| {
            let matter_type = if i < 20000 {
                Matter::Disk
            } else {
                Matter::Jet
            };

            match matter_type {
                Matter::Disk => {
                    // Start in a disk around the black hole
                    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                    let r = rng.gen_range(0.2..0.9);
                    let height = rng.gen_range(-0.02..0.02) * r; // Thinner near center

                    // Orbital velocity (approximately circular)
                    let orbital_speed = (0.8f32 / r).sqrt();
                    let vel = Vec3::new(
                        -angle.sin() * orbital_speed,
                        0.0,
                        angle.cos() * orbital_speed,
                    );

                    Particle {
                        position: Vec3::new(angle.cos() * r, height, angle.sin() * r),
                        velocity: vel,
                        particle_type: Matter::Disk.into(),
                        color: Vec3::new(1.0, 0.5, 0.2),
                    }
                }
                Matter::Jet => {
                    // Jets start near the poles
                    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                    let r = rng.gen_range(0.0..0.05);
                    let y_sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };

                    Particle {
                        position: Vec3::new(
                            angle.cos() * r,
                            y_sign * rng.gen_range(0.05..0.1),
                            angle.sin() * r,
                        ),
                        velocity: Vec3::new(0.0, y_sign * 2.0, 0.0),
                        particle_type: Matter::Jet.into(),
                        color: Vec3::new(0.5, 0.8, 1.0),
                    }
                }
                _ => unreachable!(),
            }
        })
        .collect();

    Simulation::<Particle>::new()
        .with_particle_count(25_000)
        .with_particle_size(0.008)
        .with_bounds(2.0)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())

        // === Gravitational Functions ===
        .with_function(r#"
            fn gravity_force(pos: vec3<f32>, mass: f32) -> vec3<f32> {
                let to_center = -pos;
                let dist = length(to_center);
                if dist < 0.05 {
                    return vec3<f32>(0.0); // Inside event horizon
                }
                let strength = mass / (dist * dist);
                return normalize(to_center) * strength;
            }
        "#)

        // Relativistic color shift (fake but pretty)
        .with_function(r#"
            fn doppler_color(vel: vec3<f32>, base_temp: f32) -> vec3<f32> {
                let speed = length(vel);
                // Approaching = blue shift, receding = red shift
                let radial_vel = dot(normalize(vel), vec3<f32>(0.0, 0.0, 1.0));
                let temp = base_temp + radial_vel * 2.0 + speed * 0.5;

                // Black body-ish color
                if temp < 0.3 {
                    return vec3<f32>(0.8, 0.2, 0.1); // Cool red
                } else if temp < 0.6 {
                    return vec3<f32>(1.0, 0.6, 0.2); // Orange
                } else if temp < 0.9 {
                    return vec3<f32>(1.0, 0.9, 0.5); // Yellow-white
                } else {
                    return vec3<f32>(0.7, 0.85, 1.0); // Blue-white hot
                }
            }
        "#)

        // === Disk Particle Behavior ===
        .with_rule(Rule::Custom(r#"
            if p.particle_type == 0u {
                // Gravitational pull - weaker to keep disk stable longer
                let gravity = gravity_force(p.position, 1.5);
                p.velocity += gravity * uniforms.delta_time;

                let r = length(p.position.xz);

                // Maintain orbital velocity - adds tangential speed to keep orbiting
                let orbital_speed = sqrt(1.5 / max(r, 0.1));
                let current_tangent = vec3<f32>(-p.position.z, 0.0, p.position.x) / max(r, 0.01);
                let tangent_vel = dot(p.velocity, current_tangent);
                if tangent_vel < orbital_speed * 0.9 {
                    p.velocity += current_tangent * (orbital_speed - tangent_vel) * 0.5 * uniforms.delta_time;
                }

                // Very slow inward drift (accretion)
                if r > 0.15 {
                    let inward = -normalize(vec3<f32>(p.position.x, 0.0, p.position.z)) * 0.02;
                    p.velocity += inward * uniforms.delta_time;
                }

                // Flatten to disk plane
                p.velocity.y -= p.position.y * 3.0 * uniforms.delta_time;

                // Check for event horizon crossing - smaller threshold
                let dist = length(p.position);
                if dist < 0.06 {
                    // Spaghettification!
                    p.velocity += normalize(-p.position) * 3.0 * uniforms.delta_time;
                    p.particle_type = 1u;
                }

                // Color based on temperature (velocity)
                let temp = length(p.velocity) * 0.3 + (1.0 - r) * 0.5;
                p.color = doppler_color(p.velocity, temp);
            }
        "#.into()))

        // === Infalling Particles ===
        .with_rule(Rule::Custom(r#"
            if p.particle_type == 1u {
                // Strong gravity, stretching
                let gravity = gravity_force(p.position, 8.0);
                p.velocity += gravity * uniforms.delta_time;

                // Stretch toward center (spaghettification)
                let to_center = normalize(-p.position);
                p.velocity += to_center * 3.0 * uniforms.delta_time;

                // Fade to black as approaching singularity
                let dist = length(p.position);
                let fade = smoothstep(0.0, 0.08, dist);
                p.color = vec3<f32>(1.0, 0.3, 0.1) * fade;

                // "Die" at singularity (respawn as jet)
                if dist < 0.02 {
                    // Respawn as jet particle
                    let y_sign = select(-1.0, 1.0, noise3(p.position * 100.0) > 0.0);
                    p.position = vec3<f32>(0.0, y_sign * 0.05, 0.0);
                    p.velocity = vec3<f32>(
                        noise3(p.position * 50.0) * 0.3,
                        y_sign * 3.0,
                        noise3(p.position * 50.0 + 100.0) * 0.3
                    );
                    p.particle_type = 2u;
                }
            }
        "#.into()))

        // === Jet Particles ===
        .with_rule(Rule::Custom(r#"
            if p.particle_type == 2u {
                // Collimated outward acceleration
                let y_sign = sign(p.position.y);
                p.velocity.y += y_sign * 4.0 * uniforms.delta_time;

                // Slight collimation toward axis
                p.velocity.x -= p.position.x * 2.0 * uniforms.delta_time;
                p.velocity.z -= p.position.z * 2.0 * uniforms.delta_time;

                // Hot blue-white color, fading with distance
                let dist_from_pole = abs(p.position.y);
                let brightness = 1.0 - smoothstep(0.0, 1.5, dist_from_pole);
                p.color = vec3<f32>(0.6, 0.8, 1.0) * brightness + vec3<f32>(0.2, 0.1, 0.4) * (1.0 - brightness);

                // Respawn when too far
                if dist_from_pole > 1.5 {
                    // Back to disk
                    let angle = noise3(p.position * 10.0) * 6.28;
                    let r = 0.4 + noise3(p.position * 10.0 + 50.0) * 0.4;
                    p.position = vec3<f32>(cos(angle) * r, 0.0, sin(angle) * r);
                    let orbital_speed = sqrt(0.8 / r);
                    p.velocity = vec3<f32>(-sin(angle) * orbital_speed, 0.0, cos(angle) * orbital_speed);
                    p.particle_type = 0u;
                }
            }
        "#.into()))

        .with_rule(Rule::Drag(0.3))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 5.0 })
        .with_rule(Rule::WrapWalls)
        .run().expect("Simulation failed");
}
