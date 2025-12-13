//! # Galaxy Formation
//!
//! Stars orbiting a central mass, forming spiral arm structures
//! through gravitational dynamics and initial angular momentum.
//!
//! ## What This Demonstrates
//!
//! - Central gravitational attractor
//! - Initial tangential velocity creates orbits
//! - Particle interactions create spiral density waves
//! - Color based on orbital velocity (blue=fast, red=slow)
//! - Varying star masses with visual scale
//!
//! ## Physics
//!
//! - Strong central gravity (supermassive black hole / dark matter halo)
//! - Stars have initial circular velocity based on distance
//! - Slight perturbations create spiral arm structures
//! - Drag simulates dynamical friction
//! - Mass-weighted n-body gravity between nearby stars
//!
//! Run with: `cargo run --example galaxy --release`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Star {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    mass: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_stars = 30000;

    // Generate stars in a disk with initial orbital velocity
    let stars: Vec<_> = (0..num_stars)
        .map(|_| {
            // Disk distribution - more stars near center
            let r = rng.gen_range(0.05_f32..0.9).powf(0.5); // sqrt for uniform area density
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);

            // Slight thickness in z
            let z = rng.gen_range(-0.02..0.02) * (1.0 - r); // Thinner at edges

            let x = r * theta.cos();
            let y = r * theta.sin();

            // Orbital velocity - circular orbit around center
            // v = sqrt(GM/r), we'll use GM = 0.5
            let orbital_speed = (0.5 / r.max(0.1)).sqrt() * 0.8;

            // Tangential direction (perpendicular to radius)
            let vx = -theta.sin() * orbital_speed;
            let vy = theta.cos() * orbital_speed;

            // Add some random perturbation to create spiral structure
            let perturb = rng.gen_range(-0.05..0.05);

            // Mass follows a power law - most stars small, few large
            // Using exponential distribution for more realistic IMF
            let mass = rng.gen_range(0.0_f32..1.0).powf(3.0) * 4.0 + 0.5; // 0.5 to 4.5

            (
                Vec3::new(x, y, z),
                Vec3::new(vx + perturb, vy + perturb, 0.0),
                mass,
            )
        })
        .collect();

    Simulation::<Star>::new()
        .with_particle_count(num_stars as u32)
        .with_bounds(10.0)
        .with_spawner(move |ctx| {
            let (pos, vel, mass) = stars[ctx.index as usize];
            Star {
                position: pos,
                velocity: vel,
                color: Vec3::ONE, // Will be set by shader
                mass,
            }
        })
        // Spatial hashing for neighbor queries
        .with_spatial_config(0.2, 32)
        // Mass-weighted n-body gravity between nearby stars
        .with_rule(Rule::NeighborCustom(
            r#"
            // Mass-weighted gravitational attraction
            let dist_sq = neighbor_dist * neighbor_dist + 0.03 * 0.03; // softening
            let force = other.mass * 0.015 / dist_sq;
            // Attract toward neighbor (opposite of neighbor_dir)
            p.velocity -= neighbor_dir * force * uniforms.delta_time;
            "#
            .into(),
        ))
        // Very light drag (dynamical friction)
        .with_rule(Rule::Drag(0.1))
        // Central gravity + coloring + scale by mass
        .with_rule(Rule::Custom(
            r#"
            // Scale by mass (visual size)
            p.scale = sqrt(p.mass) * 0.5;

            // Central gravitational attractor
            let to_center = -p.position;
            let dist = length(to_center);
            let dir = to_center / max(dist, 0.01);

            // Gravitational acceleration: a = GM/r^2
            // Softened to prevent singularity and limit max acceleration
            let gm = 0.3;
            let softening = 0.1;
            let accel = gm / (dist * dist + softening * softening);
            // Cap maximum acceleration to prevent slingshots
            let accel_capped = min(accel, 3.0);
            p.velocity += dir * accel_capped * uniforms.delta_time;

            // Color based on velocity (orbital speed)
            let speed = length(p.velocity);

            // Fast inner stars: blue-white
            // Slow outer stars: red-orange
            let t = clamp(speed * 1.5, 0.0, 1.0);

            if t > 0.6 {
                let blend = (t - 0.6) / 0.4;
                p.color = mix(vec3<f32>(0.8, 0.8, 1.0), vec3<f32>(0.9, 0.95, 1.0), blend);
            } else if t > 0.3 {
                let blend = (t - 0.3) / 0.3;
                p.color = mix(vec3<f32>(1.0, 0.7, 0.3), vec3<f32>(0.8, 0.8, 1.0), blend);
            } else {
                let blend = t / 0.3;
                p.color = mix(vec3<f32>(1.0, 0.3, 0.1), vec3<f32>(1.0, 0.7, 0.3), blend);
            }

            // Brighten massive stars
            let mass_bright = 0.7 + p.mass * 0.15;
            p.color *= mass_bright;
            "#
            .into(),
        ))
        .with_visuals(|v| {
            v.background(Vec3::new(0.0, 0.0, 0.02)); // Deep space
            v.blend_mode(BlendMode::Additive);
        })
        .with_rule(Rule::BounceWalls)
        .with_rule_inspector()
        .with_particle_inspector()
        .run().expect("Simulation failed");
}
