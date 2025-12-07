//! # Hydraulic Erosion Simulation
//!
//! Water droplets flow downhill over implicit terrain, demonstrating
//! gradient-following behavior and sediment transport visualization.
//!
//! ## What This Demonstrates
//!
//! - Particles following terrain gradients (flowing downhill)
//! - Using a field as a 2D heightmap
//! - Color changes based on sediment load
//! - Particle respawning for continuous rain
//!
//! Run with: `cargo run --example erosion --release`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct WaterDroplet {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Amount of sediment being carried (affects color)
    sediment: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_droplets = 20000;

    // Pre-generate initial positions - rain from above
    let positions: Vec<Vec3> = (0..num_droplets)
        .map(|_| {
            Vec3::new(
                rng.gen_range(-0.9..0.9),
                0.0, // Will be set to terrain height
                rng.gen_range(-0.9..0.9),
            )
        })
        .collect();

    Simulation::<WaterDroplet>::new()
        .with_particle_count(num_droplets as u32)
        .with_bounds(1.0)
        .with_spawner(move |ctx| WaterDroplet {
            position: positions[ctx.index as usize],
            velocity: Vec3::ZERO,
            color: Vec3::new(0.3, 0.6, 1.0), // Blue water
            sediment: 0.0,
        })
        // Water flow over implicit terrain (procedural hills)
        .with_rule(Rule::Custom(
            r#"
            let dt = uniforms.delta_time;

            // Implicit terrain function: central peak with some ridges
            // height(x,z) = exp(-r²) + ridges
            let x = p.position.x;
            let z = p.position.z;
            let r2 = x*x + z*z;

            // Central mountain
            let central_height = exp(-r2 * 3.0) * 0.5;

            // Add some ridges using sin waves
            let ridge1 = sin(x * 8.0) * sin(z * 8.0) * 0.1 * exp(-r2 * 2.0);
            let ridge2 = sin(x * 5.0 + 1.0) * sin(z * 7.0 + 2.0) * 0.08;

            let terrain_height = central_height + ridge1 + ridge2 + 0.1;

            // Sample terrain at offset positions to get gradient
            let eps = 0.02;
            let x_plus = x + eps;
            let x_minus = x - eps;
            let z_plus = z + eps;
            let z_minus = z - eps;

            let h_xp = exp(-(x_plus*x_plus + z*z) * 3.0) * 0.5
                     + sin(x_plus * 8.0) * sin(z * 8.0) * 0.1 * exp(-(x_plus*x_plus + z*z) * 2.0)
                     + sin(x_plus * 5.0 + 1.0) * sin(z * 7.0 + 2.0) * 0.08;

            let h_xm = exp(-(x_minus*x_minus + z*z) * 3.0) * 0.5
                     + sin(x_minus * 8.0) * sin(z * 8.0) * 0.1 * exp(-(x_minus*x_minus + z*z) * 2.0)
                     + sin(x_minus * 5.0 + 1.0) * sin(z * 7.0 + 2.0) * 0.08;

            let h_zp = exp(-(x*x + z_plus*z_plus) * 3.0) * 0.5
                     + sin(x * 8.0) * sin(z_plus * 8.0) * 0.1 * exp(-(x*x + z_plus*z_plus) * 2.0)
                     + sin(x * 5.0 + 1.0) * sin(z_plus * 7.0 + 2.0) * 0.08;

            let h_zm = exp(-(x*x + z_minus*z_minus) * 3.0) * 0.5
                     + sin(x * 8.0) * sin(z_minus * 8.0) * 0.1 * exp(-(x*x + z_minus*z_minus) * 2.0)
                     + sin(x * 5.0 + 1.0) * sin(z_minus * 7.0 + 2.0) * 0.08;

            // Gradient (points uphill)
            let grad_x = (h_xp - h_xm) / (2.0 * eps);
            let grad_z = (h_zp - h_zm) / (2.0 * eps);

            // Flow downhill (negative gradient)
            let flow_force = vec3<f32>(-grad_x, 0.0, -grad_z) * 2.0;
            p.velocity += flow_force * dt;

            // Add slight randomness for more natural flow
            let seed = index + u32(uniforms.time * 100.0);
            let jitter = rand_sphere(seed) * 0.1;
            p.velocity.x += jitter.x * dt;
            p.velocity.z += jitter.z * dt;

            // Damping
            p.velocity *= 0.97;

            // Keep on terrain surface
            p.position.y = terrain_height;

            // Sediment pickup when moving fast (erosion visualization)
            let speed = length(p.velocity);
            if speed > 0.2 {
                p.sediment = min(p.sediment + speed * dt * 0.5, 1.0);
            } else {
                // Deposit sediment when slow
                p.sediment = max(p.sediment - dt * 0.3, 0.0);
            }

            // Color based on sediment: blue (clean) -> brown (muddy)
            let sed = p.sediment;
            p.color = mix(
                vec3<f32>(0.3, 0.6, 1.0),   // Clean water
                vec3<f32>(0.6, 0.4, 0.2),   // Muddy water
                sed
            );

            // Respawn if out of bounds or stuck at bottom
            let dist_from_center = sqrt(x*x + z*z);
            if dist_from_center > 0.95 || (speed < 0.05 && terrain_height < 0.15) {
                // Respawn near the peak
                let spawn_seed = index + u32(uniforms.time * 1000.0);
                let angle = rand(spawn_seed) * 6.28318;
                let radius = rand(spawn_seed + 1u) * 0.3;
                p.position.x = cos(angle) * radius;
                p.position.z = sin(angle) * radius;
                p.velocity = vec3<f32>(0.0, 0.0, 0.0);
                p.sediment = 0.0;
            }
            "#
            .into(),
        ))
        // Visuals
        .with_visuals(|v| {
            v.background(Vec3::new(0.2, 0.3, 0.2)); // Earthy green
            v.blend_mode(BlendMode::Additive);
        })
        .run();
}
