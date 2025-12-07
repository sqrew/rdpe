//! # Meta Simulation Runner
//!
//! Loads a simulation config from JSON and runs it.
//! Designed to be spawned by the RDPE editor.
//!
//! Usage: `cargo run --example meta_sim --features egui -- config.json`

use rdpe::prelude::*;
use std::env;
use std::fs;
use std::path::PathBuf;

// Config types shared with editor (in a real app, this would be a shared crate)
mod config {
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::path::PathBuf;

    #[derive(Clone, Serialize, Deserialize)]
    pub enum SpawnShape {
        Cube { size: f32 },
        Sphere { radius: f32 },
        Shell { inner: f32, outer: f32 },
        Ring { radius: f32, thickness: f32 },
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub enum InitialVelocity {
        Zero,
        RandomDirection { speed: f32 },
        Outward { speed: f32 },
        Inward { speed: f32 },
        Swirl { speed: f32 },
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub enum ColorMode {
        Uniform { r: f32, g: f32, b: f32 },
        RandomHue { saturation: f32, value: f32 },
        ByPosition,
        ByVelocity,
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct SpawnConfig {
        pub shape: SpawnShape,
        pub velocity: InitialVelocity,
        pub mass_range: (f32, f32),
        pub energy_range: (f32, f32),
        pub color_mode: ColorMode,
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub enum RuleConfig {
        Gravity(f32),
        Drag(f32),
        BounceWalls,
        WrapWalls,
        Separate { radius: f32, strength: f32 },
        Cohere { radius: f32, strength: f32 },
        Align { radius: f32, strength: f32 },
        AttractTo { point: [f32; 3], strength: f32 },
        Wander { strength: f32, frequency: f32 },
        SpeedLimit { min: f32, max: f32 },
        Custom { code: String, params: Vec<(String, f32)> },
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct SimConfig {
        pub name: String,
        pub particle_count: u32,
        pub bounds: f32,
        pub particle_size: f32,
        pub spatial_cell_size: f32,
        pub spatial_resolution: u32,
        pub spawn: SpawnConfig,
        pub rules: Vec<RuleConfig>,
    }

    impl SimConfig {
        pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
            let json = fs::read_to_string(path)?;
            let config = serde_json::from_str(&json)?;
            Ok(config)
        }
    }
}

use config::*;

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

fn rule_config_to_rule(rule: &RuleConfig) -> Rule {
    match rule {
        RuleConfig::Gravity(g) => Rule::Gravity(*g),
        RuleConfig::Drag(d) => Rule::Drag(*d),
        RuleConfig::BounceWalls => Rule::BounceWalls,
        RuleConfig::WrapWalls => Rule::WrapWalls,
        RuleConfig::Separate { radius, strength } => Rule::Separate {
            radius: *radius,
            strength: *strength,
        },
        RuleConfig::Cohere { radius, strength } => Rule::Cohere {
            radius: *radius,
            strength: *strength,
        },
        RuleConfig::Align { radius, strength } => Rule::Align {
            radius: *radius,
            strength: *strength,
        },
        RuleConfig::AttractTo { point, strength } => Rule::AttractTo {
            point: Vec3::from_array(*point),
            strength: *strength,
        },
        RuleConfig::Wander { strength, frequency } => Rule::Wander {
            strength: *strength,
            frequency: *frequency,
        },
        RuleConfig::SpeedLimit { min, max } => Rule::SpeedLimit {
            min: *min,
            max: *max,
        },
        RuleConfig::Custom { code, params } => {
            let mut builder = Rule::custom_dynamic(code.clone());
            for (name, value) in params {
                builder = builder.with_param(name, *value);
            }
            builder.into()
        }
    }
}

fn main() {
    // Get config path from command line args
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("simulation.json")
    };

    // Load config
    let config = match SimConfig::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config from {:?}: {}", config_path, e);
            eprintln!("Using default configuration.");
            SimConfig {
                name: "Default".into(),
                particle_count: 5000,
                bounds: 1.0,
                particle_size: 0.015,
                spatial_cell_size: 0.1,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Sphere { radius: 0.5 },
                    velocity: InitialVelocity::RandomDirection { speed: 0.1 },
                    mass_range: (1.0, 1.0),
                    energy_range: (1.0, 1.0),
                    color_mode: ColorMode::RandomHue { saturation: 0.8, value: 0.9 },
                },
                rules: vec![
                    RuleConfig::Gravity(2.0),
                    RuleConfig::Drag(0.5),
                    RuleConfig::BounceWalls,
                ],
            }
        }
    };

    println!("Running simulation: {}", config.name);
    println!("  Particles: {}", config.particle_count);
    println!("  Rules: {}", config.rules.len());

    // Clone spawn config for the spawner closure
    let spawn_config = config.spawn.clone();

    // Convert rules
    let rules: Vec<Rule> = config.rules.iter().map(rule_config_to_rule).collect();
    let needs_spatial = rules.iter().any(|r| r.requires_neighbors());

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

    // Run with inspectors enabled
    sim.with_particle_inspector()
        .with_rule_inspector()
        .run();
}
