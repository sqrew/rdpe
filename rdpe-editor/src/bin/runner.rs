//! RDPE Simulation Runner
//!
//! Loads a simulation config from JSON and runs it.
//! Designed to be spawned by the RDPE editor.
//!
//! Usage: `rdpe-runner config.json`

use glam::Vec3;
use rdpe::prelude::*;
use rdpe_editor::config::*;
use std::env;
use std::path::PathBuf;

/// Flexible particle type for the meta simulation
#[derive(Particle, Clone)]
struct MetaParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,
    mass: f32,
    energy: f32,
    heat: f32,
    custom: f32,
    goal: Vec3,
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let h = h * 6.0;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 1.0 {
        (c, x, 0.0)
    } else if h < 2.0 {
        (x, c, 0.0)
    } else if h < 3.0 {
        (0.0, c, x)
    } else if h < 4.0 {
        (0.0, x, c)
    } else if h < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Vec3::new(r + m, g + m, b + m)
}

fn spawn_particle(ctx: &mut SpawnContext, spawn: &SpawnConfig) -> MetaParticle {
    let position = match &spawn.shape {
        SpawnShape::Cube { size } => ctx.random_in_cube(*size),
        SpawnShape::Sphere { radius } => ctx.random_in_sphere(*radius),
        SpawnShape::Shell { inner, outer } => {
            let dir = ctx.random_direction();
            let r = *inner + ctx.random() * (*outer - *inner);
            dir * r
        }
        SpawnShape::Ring { radius, thickness } => {
            let angle = ctx.random() * std::f32::consts::TAU;
            let r = *radius + (ctx.random() - 0.5) * *thickness;
            Vec3::new(angle.cos() * r, (ctx.random() - 0.5) * *thickness, angle.sin() * r)
        }
        SpawnShape::Point => Vec3::ZERO,
        SpawnShape::Line { length } => {
            let t = ctx.random() - 0.5;
            Vec3::new(t * *length, 0.0, 0.0)
        }
        SpawnShape::Plane { width, depth } => {
            Vec3::new(
                (ctx.random() - 0.5) * *width,
                0.0,
                (ctx.random() - 0.5) * *depth,
            )
        }
    };

    let velocity = match &spawn.velocity {
        InitialVelocity::Zero => Vec3::ZERO,
        InitialVelocity::RandomDirection { speed } => ctx.random_direction() * *speed,
        InitialVelocity::Outward { speed } => {
            if position.length() > 0.001 {
                position.normalize() * *speed
            } else {
                ctx.random_direction() * *speed
            }
        }
        InitialVelocity::Inward { speed } => {
            if position.length() > 0.001 {
                -position.normalize() * *speed
            } else {
                ctx.random_direction() * *speed
            }
        }
        InitialVelocity::Swirl { speed } => {
            let tangent = Vec3::new(-position.z, 0.0, position.x);
            if tangent.length() > 0.001 {
                tangent.normalize() * *speed
            } else {
                ctx.random_direction() * *speed
            }
        }
        InitialVelocity::Directional { direction, speed } => {
            Vec3::from_array(*direction).normalize_or_zero() * *speed
        }
    };

    let color = match &spawn.color_mode {
        ColorMode::Uniform { r, g, b } => Vec3::new(*r, *g, *b),
        ColorMode::RandomHue { saturation, value } => {
            hsv_to_rgb(ctx.random(), *saturation, *value)
        }
        ColorMode::ByPosition => {
            Vec3::new(
                position.x * 0.5 + 0.5,
                position.y * 0.5 + 0.5,
                position.z * 0.5 + 0.5,
            )
        }
        ColorMode::ByVelocity => {
            let speed = velocity.length();
            hsv_to_rgb((speed * 2.0).fract(), 0.9, 0.9)
        }
        ColorMode::Gradient { start, end } => {
            // Gradient based on spawn index normalized to 0-1
            let t = ctx.random(); // Approximate gradient with random for now
            let start = Vec3::from_array(*start);
            let end = Vec3::from_array(*end);
            start.lerp(end, t)
        }
    };

    let mass = spawn.mass_range.0 + ctx.random() * (spawn.mass_range.1 - spawn.mass_range.0);
    let energy = spawn.energy_range.0 + ctx.random() * (spawn.energy_range.1 - spawn.energy_range.0);

    MetaParticle {
        position,
        velocity,
        color,
        particle_type: 0,
        mass,
        energy,
        heat: 0.0,
        custom: 0.0,
        goal: Vec3::ZERO,
    }
}

fn main() {
    // Get config path from command line args
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        eprintln!("Usage: rdpe-runner <config.json>");
        eprintln!("No config file specified, using defaults.");
        PathBuf::from("simulation.json")
    };

    // Load config
    let config = match SimConfig::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config from {:?}: {}", config_path, e);
            eprintln!("Using default configuration.");
            SimConfig::default()
        }
    };

    // Clone spawn config for the spawner closure
    let spawn_config = config.spawn.clone();

    // Convert rules
    let rules: Vec<Rule> = config.rules.iter().map(|r| r.to_rule()).collect();
    let needs_spatial = config.needs_spatial();

    // Build simulation
    let mut sim = Simulation::<MetaParticle>::new()
        .with_particle_count(config.particle_count)
        .with_bounds(config.bounds)
        .with_particle_size(config.particle_size)
        .with_spawner(move |ctx| spawn_particle(ctx, &spawn_config));

    // Add spatial config if needed
    if needs_spatial {
        sim = sim.with_spatial_config(config.spatial_cell_size, config.spatial_resolution);
    }

    // Add rules
    for rule in rules {
        sim = sim.with_rule(rule);
    }

    // Add vertex effects
    for effect_config in &config.vertex_effects {
        sim = sim.with_vertex_effect(effect_config.to_effect());
    }

    // Apply visuals
    let visuals = config.visuals.clone();
    sim = sim.with_visuals(|v| {
        v.blend_mode(visuals.blend_mode.to_blend_mode());
        v.shape(visuals.shape.to_shape());
        v.background(glam::Vec3::from_array(visuals.background_color));

        // Apply palette and color mapping
        if visuals.palette != rdpe_editor::config::PaletteConfig::None {
            v.palette(visuals.palette.to_palette(), visuals.color_mapping.to_color_mapping());
        }

        // Apply trails
        if visuals.trail_length > 0 {
            v.trails(visuals.trail_length);
        }

        // Apply connections
        if visuals.connections_enabled {
            v.connections(visuals.connections_radius);
            v.connections_color(glam::Vec3::from_array(visuals.connections_color));
        }

        // Apply velocity stretch
        if visuals.velocity_stretch {
            v.velocity_stretch(visuals.velocity_stretch_factor);
        }

        // Apply spatial grid debug
        if visuals.spatial_grid_opacity > 0.0 {
            v.spatial_grid(visuals.spatial_grid_opacity);
        }

        // Apply wireframe
        if let Some(mesh) = visuals.wireframe.to_mesh() {
            v.wireframe(mesh, visuals.wireframe_thickness);
        }
    });

    // Run with inspectors enabled
    sim.with_particle_inspector()
        .with_rule_inspector()
        .run();
}
