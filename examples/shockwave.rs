//! Shockwave demonstration.
//!
//! Shows expanding shockwaves that push particles outward.
//! Particles also breathe in and out with a pulse effect.

use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
}

fn main() {
    let count = 30_000;

    Simulation::<Particle>::new()
        .with_particle_count(count)
        .with_bounds(1.5)
        .with_particle_size(0.012)
        .with_spawner(|i, total| {
            // Spawn in a spherical shell
            let phi = (i as f32 / total as f32) * std::f32::consts::PI * 2.0 * 100.0;
            let theta = ((i as f32 * 0.618033988749895) % 1.0) * std::f32::consts::PI;
            let r = 0.3 + (i as f32 * 0.381966) % 0.4;

            let x = r * theta.sin() * phi.cos();
            let y = r * theta.cos();
            let z = r * theta.sin() * phi.sin();

            Particle {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
            }
        })
        // Color by distance - Ocean palette for watery ripple effect
        .with_visuals(|v| {
            v.palette(Palette::Ocean, ColorMapping::Distance { max_dist: 1.5 });
            v.blend_mode(BlendMode::Additive);
        })
        // Repeating shockwave every 3 seconds
        .with_rule(Rule::Shockwave {
            origin: Vec3::ZERO,
            speed: 0.8,
            width: 0.25,
            strength: 4.0,
            repeat: 3.0,
        })
        // Gentle breathing pulse
        .with_rule(Rule::Pulse {
            point: Vec3::ZERO,
            strength: 0.5,
            frequency: 0.3,
            radius: 0.0,  // Unlimited
        })
        // Soft attraction back to center
        .with_rule(Rule::Radial {
            point: Vec3::ZERO,
            strength: -0.8,  // Negative = inward
            radius: 2.0,
            falloff: Falloff::Linear,
        })
        // Drag to stabilize
        .with_rule(Rule::Drag(1.5))
        // Speed limit
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.5 })
        // Soft bounce
        .with_rule(Rule::BounceWalls)
        .run();
}
