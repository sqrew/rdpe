//! Simulation presets

use crate::config::{
    ColorMode, CustomShaderConfig, FieldConfigEntry, FieldTypeConfig, InitialVelocity,
    PaletteConfig, RuleConfig, SimConfig, SpawnConfig, SpawnShape, UniformValueConfig,
    VisualsConfig, VolumeRenderConfig,
};
use std::collections::HashMap;

pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub config: fn() -> SimConfig,
}

pub static PRESETS: &[Preset] = &[
    Preset {
        name: "Boids Flocking",
        description: "Classic boids algorithm with separation, cohesion, alignment",
        config: || SimConfig {
            name: "Boids Flocking".into(),
            particle_count: 5000,
            bounds: 1.0,
            particle_size: 0.01,
            spatial_cell_size: 0.15,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.5 },
                velocity: InitialVelocity::RandomDirection { speed: 0.2 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Separate { radius: 0.05, strength: 5.0 },
                RuleConfig::Cohere { radius: 0.15, strength: 1.0 },
                RuleConfig::Align { radius: 0.1, strength: 2.0 },
                RuleConfig::SpeedLimit { min: 0.1, max: 0.5 },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Gravity Well",
        description: "Particles orbiting around a central attractor",
        config: || SimConfig {
            name: "Gravity Well".into(),
            particle_count: 10000,
            bounds: 1.5,
            particle_size: 0.008,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell { inner: 0.3, outer: 0.8 },
                velocity: InitialVelocity::Swirl { speed: 0.4 },
                color_mode: ColorMode::ByVelocity,
                ..Default::default()
            },
            rules: vec![
                RuleConfig::PointGravity { point: [0.0, 0.0, 0.0], strength: 2.0, softening: 0.05 },
                RuleConfig::Drag(0.1),
                RuleConfig::SpeedLimit { min: 0.0, max: 2.0 },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Explosion",
        description: "Particles exploding outward with gravity",
        config: || SimConfig {
            name: "Explosion".into(),
            particle_count: 20000,
            bounds: 2.0,
            particle_size: 0.005,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.1 },
                velocity: InitialVelocity::Outward { speed: 1.5 },
                color_mode: ColorMode::RandomHue { saturation: 1.0, value: 1.0 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Gravity(3.0),
                RuleConfig::Drag(0.3),
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Fluid Simulation",
        description: "SPH-like fluid with pressure and viscosity",
        config: || SimConfig {
            name: "Fluid Simulation".into(),
            particle_count: 8000,
            bounds: 1.0,
            particle_size: 0.012,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Cube { size: 0.4 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform { r: 0.2, g: 0.5, b: 1.0 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Gravity(5.0),
                RuleConfig::Pressure { radius: 0.08, strength: 3.0, target_density: 10.0 },
                RuleConfig::Viscosity { radius: 0.08, strength: 0.5 },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Vortex",
        description: "Swirling tornado effect",
        config: || SimConfig {
            name: "Vortex".into(),
            particle_count: 15000,
            bounds: 1.5,
            particle_size: 0.006,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell { inner: 0.2, outer: 1.0 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::ByPosition,
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Vortex { center: [0.0, 0.0, 0.0], axis: [0.0, 1.0, 0.0], strength: 3.0 },
                RuleConfig::AttractTo { point: [0.0, 0.0, 0.0], strength: 0.5 },
                RuleConfig::Drag(0.5),
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Molecular Dynamics",
        description: "Lennard-Jones potential for molecular interactions",
        config: || SimConfig {
            name: "Molecular Dynamics".into(),
            particle_count: 3000,
            bounds: 1.0,
            particle_size: 0.02,
            spatial_cell_size: 0.15,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Cube { size: 0.6 },
                velocity: InitialVelocity::RandomDirection { speed: 0.3 },
                color_mode: ColorMode::RandomHue { saturation: 0.6, value: 0.9 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::LennardJones { epsilon: 0.5, sigma: 0.05, cutoff: 0.15 },
                RuleConfig::Drag(0.2),
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Predator Prey",
        description: "Chase and evade dynamics between two species",
        config: || SimConfig {
            name: "Predator Prey".into(),
            particle_count: 5000,
            bounds: 1.0,
            particle_size: 0.012,
            spatial_cell_size: 0.2,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.8 },
                velocity: InitialVelocity::RandomDirection { speed: 0.1 },
                color_mode: ColorMode::RandomHue { saturation: 0.8, value: 0.9 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Chase { self_type: 1, target_type: 0, radius: 0.3, strength: 3.0 },
                RuleConfig::Evade { self_type: 0, threat_type: 1, radius: 0.2, strength: 4.0 },
                RuleConfig::Separate { radius: 0.05, strength: 2.0 },
                RuleConfig::SpeedLimit { min: 0.1, max: 1.0 },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Curl Noise Flow",
        description: "Smooth, divergence-free fluid-like motion",
        config: || SimConfig {
            name: "Curl Noise Flow".into(),
            particle_count: 20000,
            bounds: 1.5,
            particle_size: 0.004,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Cube { size: 1.2 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::ByVelocity,
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Curl { scale: 2.0, strength: 1.5 },
                RuleConfig::Drag(0.2),
                RuleConfig::WrapWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Lifecycle",
        description: "Particles with aging, fading, and respawning",
        config: || SimConfig {
            name: "Lifecycle".into(),
            particle_count: 10000,
            bounds: 1.0,
            particle_size: 0.01,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Point,
                velocity: InitialVelocity::RandomDirection { speed: 0.5 },
                color_mode: ColorMode::Uniform { r: 1.0, g: 0.8, b: 0.3 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Gravity(2.0),
                RuleConfig::Age,
                RuleConfig::FadeOut(3.0),
                RuleConfig::ShrinkOut(3.0),
                RuleConfig::Lifetime(3.0),
                RuleConfig::RespawnBelow { threshold_y: -1.0, spawn_y: 0.5, reset_velocity: true },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "N-Body Gravity",
        description: "Mutual gravitational attraction between particles",
        config: || SimConfig {
            name: "N-Body Gravity".into(),
            particle_count: 2000,
            bounds: 2.0,
            particle_size: 0.015,
            spatial_cell_size: 0.3,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell { inner: 0.5, outer: 1.5 },
                velocity: InitialVelocity::Swirl { speed: 0.2 },
                color_mode: ColorMode::RandomHue { saturation: 0.7, value: 0.9 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::NBodyGravity { strength: 0.3, softening: 0.05, radius: 1.0 },
                RuleConfig::Collide { radius: 0.03, restitution: 0.5 },
                RuleConfig::Drag(0.05),
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Custom Shader Demo",
        description: "Demonstrates custom uniforms and shader code",
        config: || SimConfig {
            name: "Custom Shader Demo".into(),
            particle_count: 8000,
            bounds: 1.5,
            particle_size: 0.015,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell { inner: 0.3, outer: 1.0 },
                velocity: InitialVelocity::Swirl { speed: 0.3 },
                color_mode: ColorMode::Uniform { r: 1.0, g: 1.0, b: 1.0 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::AttractTo { point: [0.0, 0.0, 0.0], strength: 0.3 },
                RuleConfig::Drag(0.2),
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::from([
                ("pulse_speed".to_string(), UniformValueConfig::F32(3.0)),
                ("pulse_amount".to_string(), UniformValueConfig::F32(0.4)),
                ("tint".to_string(), UniformValueConfig::Vec3([1.0, 0.6, 0.2])),
            ]),
            custom_shaders: CustomShaderConfig {
                vertex_code: "// Pulsing size effect\nsize_mult *= 1.0 + uniforms.pulse_amount * sin(uniforms.time * uniforms.pulse_speed);".to_string(),
                fragment_code: "// Apply tint color\nfrag_color *= uniforms.tint;".to_string(),
            },
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
        },
    },
    Preset {
        name: "Volume Field Demo",
        description: "3D field with volume rendering - particles leave glowing trails",
        config: || SimConfig {
            name: "Volume Field Demo".into(),
            particle_count: 5000,
            bounds: 1.2,
            particle_size: 0.008,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell { inner: 0.3, outer: 0.8 },
                velocity: InitialVelocity::Swirl { speed: 0.3 },
                color_mode: ColorMode::ByVelocity,
                ..Default::default()
            },
            rules: vec![
                RuleConfig::PointGravity { point: [0.0, 0.0, 0.0], strength: 1.0, softening: 0.1 },
                RuleConfig::Curl { scale: 3.0, strength: 0.5 },
                RuleConfig::Drag(0.1),
                RuleConfig::SpeedLimit { min: 0.05, max: 0.8 },
                RuleConfig::BounceWalls,
                // Write particle presence to the field - each particle deposits a value
                RuleConfig::Custom { code: "field_write(0u, p.position, 1.0);".into() },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![
                FieldConfigEntry {
                    name: "density".into(),
                    resolution: 64,
                    extent: 1.2,
                    decay: 0.96,
                    blur: 0.3,
                    blur_iterations: 2,
                    field_type: FieldTypeConfig::Scalar,
                },
            ],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 64,
                density_scale: 3.0,
                palette: PaletteConfig::Inferno,
                threshold: 0.01,
                additive: true,
            },
        },
    },
    Preset {
        name: "Pheromone Trails",
        description: "Particles follow and deposit pheromone trails like ants",
        config: || SimConfig {
            name: "Pheromone Trails".into(),
            particle_count: 8000,
            bounds: 1.0,
            particle_size: 0.006,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Cube { size: 0.8 },
                velocity: InitialVelocity::RandomDirection { speed: 0.3 },
                color_mode: ColorMode::Uniform { r: 0.3, g: 1.0, b: 0.5 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Drag(0.3),
                RuleConfig::SpeedLimit { min: 0.1, max: 0.5 },
                RuleConfig::WrapWalls,
                // Pheromone sensing and following - steer toward stronger scent
                RuleConfig::Custom { code: r#"
let speed = length(p.velocity);
if speed > 0.001 {
    let dir = normalize(p.velocity);
    let side = normalize(cross(dir, vec3f(0.0, 1.0, 0.0)));
    let sensor_dist = 0.1;

    let ahead = field_read(0u, p.position + dir * sensor_dist);
    let left = field_read(0u, p.position + (dir + side) * sensor_dist * 0.7);
    let right = field_read(0u, p.position + (dir - side) * sensor_dist * 0.7);

    // Steer toward stronger scent
    let steer = (left - right) * 2.0;
    p.velocity += side * steer * uniforms.delta_time;
}

// Deposit pheromone
field_write(0u, p.position, 0.5);
"#.into() },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![
                FieldConfigEntry {
                    name: "pheromone".into(),
                    resolution: 64,
                    extent: 1.0,
                    decay: 0.99,
                    blur: 0.2,
                    blur_iterations: 1,
                    field_type: FieldTypeConfig::Scalar,
                },
            ],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 48,
                density_scale: 5.0,
                palette: PaletteConfig::Neon,
                threshold: 0.02,
                additive: true,
            },
        },
    },
];
