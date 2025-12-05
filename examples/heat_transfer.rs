//! # Heat Transfer Simulation
//!
//! Particles as discrete thermal masses exchanging heat with a volumetric gas field.
//!
//! ## What This Demonstrates
//!
//! - Particles with individual temperatures
//! - Heat exchange between particles and a 3D temperature field
//! - Field diffusion simulates heat conduction through gas
//! - Field decay simulates heat loss to environment
//! - Temperature-based coloring (blue = cold, red = hot)
//!
//! ## Physics
//!
//! Each frame:
//! 1. Hot particles deposit heat into the field (warming the gas)
//! 2. Cold particles absorb heat from the field (cooling the gas)
//! 3. Field diffuses (heat spreads through the gas)
//! 4. Field decays (heat loss to ambient environment)
//! 5. Particles equilibrate toward local field temperature
//!
//! Run with: `cargo run --example heat_transfer`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct ThermalMass {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Particle temperature (0.0 = cold, 1.0 = hot)
    temperature: f32,
    /// 1 = heat source (maintains temp), 0 = passive (equilibrates)
    is_source: u32,
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_particles = 50000;

    // Pre-generate particles: some are heat sources, rest are passive
    let particles: Vec<_> = (0..num_particles)
        .map(|i| {
            let pos = Vec3::new(
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-0.8..0.8),
            );

            // First 5% are hot sources (maintain temperature)
            // Next 5% are cold sinks (maintain cold)
            // Rest are passive (equilibrate with field)
            let is_hot_source = i < num_particles / 20;
            let is_cold_sink = i >= num_particles / 20 && i < num_particles / 10;
            let is_source = is_hot_source || is_cold_sink;

            let temperature = if is_hot_source {
                1.0 // Constant hot
            } else if is_cold_sink {
                0.0 // Constant cold
            } else {
                rng.gen_range(0.3..0.5) // Passive particles start lukewarm
            };

            // Random initial velocity (sources move slower)
            let speed = if is_source { 0.15 } else { 0.4 };
            let vel = Vec3::new(
                rng.gen_range(-speed..speed),
                rng.gen_range(-speed..speed),
                rng.gen_range(-speed..speed),
            );

            (pos, vel, temperature, if is_source { 1u32 } else { 0u32 })
        })
        .collect();

    Simulation::<ThermalMass>::new()
        .with_particle_count(num_particles as u32)
        .with_bounds(1.0)
        .with_spawner(move |i, _| {
            let (pos, vel, temperature, is_source) = particles[i as usize];
            ThermalMass {
                position: pos,
                velocity: vel,
                color: Vec3::ZERO, // Will be set by shader
                temperature,
                is_source,
            }
        })
        // Temperature field - the "gas" that conducts heat
        .with_field(
            "temperature",
            FieldConfig::new(48)
                .with_extent(1.0)
                .with_decay(0.995) // Slow cooling to ambient
                .with_blur(0.15) // Heat diffusion rate
                .with_blur_iterations(2), // Smooth diffusion
        )
        // Heat exchange parameters
        .with_uniform::<f32>("exchange_rate", 2.0) // How fast particles exchange with field
        // Heat exchange between particles and field
        .with_rule(Rule::Custom(
            r#"
            let dt = uniforms.delta_time;
            let exchange_rate = uniforms.exchange_rate;

            // Sample field temperature at particle position
            let field_temp = field_read(0u, p.position);

            // Sources: maintain temperature, strongly deposit to field
            // Passive: equilibrate with field, weakly deposit
            if p.is_source == 1u {
                // Heat sources pump heat into field (or cold sinks absorb)
                let deposit = (p.temperature - 0.5) * exchange_rate * 2.0 * dt;
                field_write(0u, p.position, deposit);
                // Sources don't change temperature
            } else {
                // Passive particles exchange heat with field
                let temp_diff = p.temperature - field_temp;
                let deposit = temp_diff * exchange_rate * 0.5 * dt;
                field_write(0u, p.position, deposit);

                // Equilibrate toward field temperature
                p.temperature = mix(p.temperature, field_temp, 2.0 * dt);
                p.temperature = clamp(p.temperature, 0.0, 1.0);
            }

            // Color based on temperature: blue (cold) -> white (medium) -> red (hot)
            let t = p.temperature;
            if t < 0.5 {
                // Cold: blue to white
                let blend = t * 2.0;
                p.color = mix(vec3<f32>(0.2, 0.4, 1.0), vec3<f32>(1.0, 1.0, 1.0), blend);
            } else {
                // Hot: white to red/orange
                let blend = (t - 0.5) * 2.0;
                p.color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 0.3, 0.1), blend);
            }
            "#
            .into(),
        ))
        // Particles bounce around
        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::BounceWalls)
        // Light separation so they don't clump
        .with_spatial_config(0.15, 32)
        .with_rule(Rule::Separate {
            radius: 0.08,
            strength: 0.5,
        })
        // Volume render the temperature field as glowing gas
        .with_volume_render(
            VolumeConfig::new()
                .with_field(0)
                .with_steps(32)                  // Reduced from 64 for performance
                .with_density_scale(15.0)
                .with_palette(Palette::Inferno)
                .with_threshold(0.005),
        )
        // Visuals
        .with_visuals(|v| {
            v.background(Vec3::new(0.02, 0.02, 0.05));
            v.blend_mode(BlendMode::Additive);
        })
        .run();
}
