//! Simulation presets

use crate::config::{
    BlendModeConfig, ColorMappingConfig, ColorMode, CustomShaderConfig, Falloff, FieldConfigEntry,
    FieldTypeConfig, InitialVelocity, MouseConfig, PaletteConfig, ParticleFieldDef, ParticleFieldType,
    ParticleShapeConfig, RuleConfig, SimConfig, SpawnConfig, SpawnShape, UniformValueConfig,
    VertexEffectConfig, VisualsConfig, VolumeRenderConfig,
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
            speed: 1.0,
            spatial_cell_size: 0.15,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.5 },
                velocity: InitialVelocity::RandomDirection { speed: 0.2 },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Separate {
                    radius: 0.05,
                    strength: 5.0,
                },
                RuleConfig::Cohere {
                    radius: 0.15,
                    strength: 1.0,
                },
                RuleConfig::Align {
                    radius: 0.1,
                    strength: 2.0,
                },
                RuleConfig::SpeedLimit { min: 0.1, max: 0.5 },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
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
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.1 },
                velocity: InitialVelocity::Outward { speed: 1.5 },
                color_mode: ColorMode::RandomHue {
                    saturation: 1.0,
                    value: 1.0,
                },
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
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        },
    },
    Preset {
        name: "Fluid Simulation",
        description: "SPH-like fluid with pressure and viscosity",
        config: || SimConfig {
            name: "Fluid Simulation".into(),
            particle_count: 10000,
            bounds: 1.0,
            particle_size: 0.010,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.5 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 0.2,
                    g: 0.5,
                    b: 1.0,
                },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Gravity(1.5),
                RuleConfig::Pressure {
                    radius: 0.05,
                    strength: 1.0,
                    target_density: 1.0,
                },
                RuleConfig::Viscosity {
                    radius: 2.0,
                    strength: 1.0,
                },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        },
    },
    Preset {
        name: "Custom Shader Demo",
        description: "Demonstrates custom uniforms and shader code",
        config: || {
            SimConfig {
            name: "Custom Shader Demo".into(),
            particle_count: 8000,
            bounds: 1.5,
            particle_size: 0.015,
            speed: 1.0,
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
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        }
        },
    },
    Preset {
        name: "Volume Field Demo",
        description: "3D field with volume rendering - particles leave glowing trails",
        config: || SimConfig {
            name: "Volume Field Demo".into(),
            particle_count: 5000,
            bounds: 1.0,
            particle_size: 0.005,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell {
                    inner: 0.3,
                    outer: 0.8,
                },
                velocity: InitialVelocity::Swirl { speed: 0.3 },
                color_mode: ColorMode::ByVelocity,
                ..Default::default()
            },
            rules: vec![
                RuleConfig::PointGravity {
                    point: [0.0, 0.0, 0.0],
                    strength: 0.5,
                    softening: 0.1,
                },
                RuleConfig::Curl {
                    scale: 3.0,
                    strength: 0.5,
                },
                RuleConfig::Drag(0.1),
                RuleConfig::SpeedLimit {
                    min: 0.05,
                    max: 1.0,
                },
                RuleConfig::BounceWalls,
                // Write particle presence to the field - each particle deposits a value
                RuleConfig::Custom {
                    code: "field_write(0u, p.position, 1.0);".into(),
                },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![FieldConfigEntry {
                name: "density".into(),
                resolution: 64,
                extent: 1.2,
                decay: 0.96,
                blur: 0.3,
                blur_iterations: 2,
                field_type: FieldTypeConfig::Scalar,
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 64,
                density_scale: 3.0,
                palette: PaletteConfig::Inferno,
                threshold: 0.01,
                additive: true,
            },
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
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
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Cube { size: 0.8 },
                velocity: InitialVelocity::RandomDirection { speed: 0.3 },
                color_mode: ColorMode::Uniform {
                    r: 0.3,
                    g: 1.0,
                    b: 0.5,
                },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Drag(0.3),
                RuleConfig::SpeedLimit { min: 0.1, max: 0.5 },
                RuleConfig::WrapWalls,
                // Pheromone sensing and following - steer toward stronger scent
                RuleConfig::Custom {
                    code: r#"
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
"#
                    .into(),
                },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![FieldConfigEntry {
                name: "pheromone".into(),
                resolution: 64,
                extent: 1.0,
                decay: 0.99,
                blur: 0.2,
                blur_iterations: 1,
                field_type: FieldTypeConfig::Scalar,
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 48,
                density_scale: 5.0,
                palette: PaletteConfig::Neon,
                threshold: 0.02,
                additive: true,
            },
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        },
    },
    // === New presets from examples ===
    Preset {
        name: "Shockwave",
        description: "Expanding shockwaves that push particles outward with breathing effect",
        config: || SimConfig {
            name: "Shockwave".into(),
            particle_count: 30000,
            bounds: 1.5,
            particle_size: 0.012,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell {
                    inner: 0.3,
                    outer: 0.7,
                },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::ByVelocity,
                ..Default::default()
            },
            rules: vec![
                // Repeating shockwave every 3 seconds
                RuleConfig::Shockwave {
                    origin: [0.0, 0.0, 0.0],
                    speed: 0.8,
                    width: 0.25,
                    strength: 4.0,
                    repeat: 3.0,
                },
                // Gentle breathing pulse
                RuleConfig::Pulse {
                    point: [0.0, 0.0, 0.0],
                    strength: 0.5,
                    frequency: 0.3,
                    radius: 0.0,
                },
                // Soft attraction back to center
                RuleConfig::Radial {
                    point: [0.0, 0.0, 0.0],
                    strength: -0.8,
                    radius: 2.0,
                    falloff: Falloff::Linear,
                },
                RuleConfig::Drag(1.5),
                RuleConfig::SpeedLimit { min: 0.0, max: 1.5 },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                palette: PaletteConfig::Ocean,
                color_mapping: ColorMappingConfig::Distance { max_dist: 1.5 },
                trail_length: 10,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        },
    },
    Preset {
        name: "Galaxy",
        description: "Stars orbiting a central mass with spiral arm dynamics",
        config: || SimConfig {
            name: "Galaxy".into(),
            particle_count: 100,
            bounds: 2.0,
            particle_size: 0.01,
            speed: 1.0,
            spatial_cell_size: 0.2,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell {
                    inner: 0.5,
                    outer: 1.0,
                },
                velocity: InitialVelocity::Swirl { speed: 0.5 },
                color_mode: ColorMode::ByVelocity,
                ..Default::default()
            },
            rules: vec![
                // N-body gravity between nearby stars (mass-weighted)
                RuleConfig::NBodyGravity {
                    strength: 0.01,
                    softening: 0.05,
                    radius: 0.5,
                },
                // Very light drag (dynamical friction)
                RuleConfig::Drag(0.1),
                // Custom coloring by orbital velocity
                RuleConfig::Custom {
                    code: r#"
// Color based on velocity (orbital speed)
let speed = length(p.velocity);
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
"#
                    .into(),
                },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.0, 0.0, 0.02],
                velocity_stretch: true,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        },
    },
    Preset {
        name: "Crystal Growth",
        description: "Diffusion-limited aggregation creating dendritic fractal structures",
        config: || SimConfig {
            name: "Crystal Growth".into(),
            particle_count: 5000,
            bounds: 1.0,
            particle_size: 0.02,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            particle_fields: vec![ParticleFieldDef {
                name: "custom".into(),
                field_type: ParticleFieldType::F32,
            }],
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.8 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                },
                ..Default::default()
            },
            rules: vec![
                // Combined rule: seed initialization + Brownian motion + coloring
                RuleConfig::Custom {
                    code: r#"
// Initialize seeds in first frame (index < 5 become crystal seeds)
if uniforms.time < 0.05 && index < 5u {
    p.particle_type = 1u;
    let angle = f32(index) * 1.2566;  // TAU/5
    p.position = vec3<f32>(cos(angle) * 0.2, sin(angle) * 0.2, 0.0);
    p.velocity = vec3<f32>(0.0, 0.0, 0.0);
    p.custom = 0.0;
}

// Brownian motion for free particles
if p.particle_type == 0u {
    // Create random seed using editor's rand API (pointer-based)
    var rng_seed = f32(index) * 12.9898 + uniforms.time * 1000.0;

    // Generate random direction on sphere using three independent random values
    let rx = rand(&rng_seed) * 2.0 - 1.0;
    let ry = rand(&rng_seed) * 2.0 - 1.0;
    let rz = rand(&rng_seed) * 2.0 - 1.0;
    let v = vec3<f32>(rx, ry, rz);
    let len = length(v);

    var random_dir = vec3<f32>(0.0, 1.0, 0.0);
    if len > 0.001 {
        random_dir = v / len;
    }

    p.velocity = random_dir * 0.5;

    // Soft boundary - push back if too far
    let dist = length(p.position);
    if dist > 0.85 {
        p.velocity -= p.position * 0.3;
    }

    // Free particles are green
    p.color = vec3<f32>(0.3, 0.9, 0.3);
} else {
    // Crystallized - frozen in place
    p.velocity = vec3<f32>(0.0, 0.0, 0.0);

    // Color by crystallization time
    let t = fract(p.custom * 0.1);
    if t < 0.33 {
        let blend = t * 3.0;
        p.color = mix(vec3<f32>(0.2, 0.4, 1.0), vec3<f32>(0.2, 0.9, 1.0), blend);
    } else if t < 0.66 {
        let blend = (t - 0.33) * 3.0;
        p.color = mix(vec3<f32>(0.2, 0.9, 1.0), vec3<f32>(1.0, 1.0, 1.0), blend);
    } else {
        let blend = (t - 0.66) * 3.0;
        p.color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 0.5, 0.8), blend);
    }
}
"#
                    .into(),
                },
                // When free particle touches crystal, crystallize
                RuleConfig::OnCollision {
                    radius: 0.025,
                    response: r#"
if p.particle_type == 0u && other.particle_type == 1u {
    p.particle_type = 1u;
    p.custom = uniforms.time;
    p.velocity = vec3<f32>(0.0, 0.0, 0.0);
}
"#
                    .into(),
                },
            ],
            vertex_effects: vec![VertexEffectConfig::Pulse {
                frequency: 3.0,
                amplitude: 0.3,
            }],
            visuals: VisualsConfig {
                background_color: [0.0, 0.0, 0.0],
                connections_enabled: true,
                connections_radius: 0.05,
                shape: ParticleShapeConfig::Star,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            mouse: MouseConfig::default(),
        },
    },
    Preset {
        name: "Slime Mold",
        description: "Physarum-inspired agents depositing and following pheromone trails",
        config: || SimConfig {
            name: "Slime Mold".into(),
            particle_count: 25000,
            bounds: 1.0,
            particle_size: 0.01,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            particle_fields: vec![ParticleFieldDef {
                name: "custom".into(),
                field_type: ParticleFieldType::F32,
            }],
            spawn: SpawnConfig {
                shape: SpawnShape::Shell {
                    inner: 0.0,
                    outer: 0.8,
                },
                velocity: InitialVelocity::RandomDirection { speed: 0.01 },
                color_mode: ColorMode::Uniform {
                    r: 0.2,
                    g: 0.8,
                    b: 0.3,
                },
                ..Default::default()
            },
            rules: vec![
                // Wall wrapping
                RuleConfig::WrapWalls,
                // Slime mold behavior - sense and follow pheromones
                RuleConfig::Custom {
                    code: r#"
let dt = uniforms.delta_time;
let speed = 0.5;
let turn_speed = 4.0;
let sense_dist = 0.1;
let sense_angle = 0.4;

// Deposit pheromone at current position
field_write(0u, p.position, 0.2);

// Use p.custom as heading angle (radians)
let forward = vec3<f32>(cos(p.custom), 0.0, sin(p.custom));

// Sense in three directions
let sense_fwd = p.position + forward * sense_dist;
let left_angle = p.custom + sense_angle;
let right_angle = p.custom - sense_angle;
let sense_left = p.position + vec3<f32>(cos(left_angle), 0.0, sin(left_angle)) * sense_dist;
let sense_right = p.position + vec3<f32>(cos(right_angle), 0.0, sin(right_angle)) * sense_dist;

// Sample pheromone at each sensor
let val_fwd = field_read(0u, sense_fwd);
let val_left = field_read(0u, sense_left);
let val_right = field_read(0u, sense_right);

// Turn toward highest concentration
if val_left > val_fwd && val_left > val_right {
    p.custom = p.custom + turn_speed * dt;
} else if val_right > val_fwd && val_right > val_left {
    p.custom = p.custom - turn_speed * dt;
}

// Move forward
let new_forward = vec3<f32>(cos(p.custom), 0.0, sin(p.custom));
p.position = p.position + new_forward * speed * dt;

// Wrap at boundaries
if p.position.x > 1.1 { p.position.x = -1.1; }
if p.position.x < -1.1 { p.position.x = 1.1; }
if p.position.z > 1.1 { p.position.z = -1.1; }
if p.position.z < -1.1 { p.position.z = 1.1; }
p.position.y = 0.0;

// Color by local pheromone
let pheromone = field_read(0u, p.position);
let intensity = clamp(pheromone * 2.0, 0.0, 1.0);
p.color = vec3<f32>(intensity * 0.2, 0.3 + intensity * 0.5, 0.1 + intensity * 0.3);

p.velocity = vec3<f32>(0.0, 0.0, 0.0);
"#
                    .into(),
                },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                background_color: [0.02, 0.02, 0.05],
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![FieldConfigEntry {
                name: "pheromone".into(),
                resolution: 64,
                extent: 1.2,
                decay: 0.98,
                blur: 0.1,
                blur_iterations: 1,
                field_type: FieldTypeConfig::Scalar,
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 48,
                density_scale: 6.0,
                palette: PaletteConfig::Neon,
                threshold: 0.01,
                additive: true,
            },
            mouse: MouseConfig::default(),
        },
    },
    Preset {
        name: "Aurora",
        description: "Northern lights effect with flowing ribbons of color",
        config: || SimConfig {
            name: "Aurora".into(),
            particle_count: 15000,
            bounds: 1.5,
            particle_size: 0.01,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Plane {
                    width: 2.5,
                    depth: 0.5,
                },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::ByPosition,
                ..Default::default()
            },
            rules: vec![
                // Flowing curtain motion
                RuleConfig::Curl {
                    scale: 2.0,
                    strength: 0.8,
                },
                RuleConfig::Turbulence {
                    scale: 3.0,
                    strength: 0.3,
                },
                // Gentle upward drift
                RuleConfig::Acceleration {
                    direction: [0.0, 0.1, 0.0],
                },
                RuleConfig::Drag(1.5),
                // Aurora coloring
                RuleConfig::Custom {
                    code: r#"
// Aurora colors based on position and time
let t = uniforms.time * 0.3;
let wave = sin(p.position.x * 3.0 + t) * 0.5 + 0.5;
let height = (p.position.y + 1.0) * 0.5;

// Green to blue to purple gradient
if wave < 0.3 {
    p.color = vec3<f32>(0.1, 0.8, 0.3);  // Green
} else if wave < 0.6 {
    let blend = (wave - 0.3) / 0.3;
    p.color = mix(vec3<f32>(0.1, 0.8, 0.3), vec3<f32>(0.2, 0.5, 0.9), blend);
} else {
    let blend = (wave - 0.6) / 0.4;
    p.color = mix(vec3<f32>(0.2, 0.5, 0.9), vec3<f32>(0.6, 0.2, 0.8), blend);
}

// Fade at edges
p.color *= smoothstep(0.0, 0.3, height) * (1.0 - smoothstep(0.7, 1.0, height));
"#
                    .into(),
                },
                RuleConfig::WrapWalls,
            ],
            vertex_effects: vec![VertexEffectConfig::Wave {
                direction: [0.0, 1.0, 0.0],
                frequency: 2.0,
                speed: 1.0,
                amplitude: 0.1,
            }],
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.0, 0.0, 0.02],
                velocity_stretch: true,
                velocity_stretch_factor: 3.0,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        },
    },
    Preset {
        name: "Fireflies",
        description: "Glowing particles that pulse and wander in the dark",
        config: || SimConfig {
            name: "Fireflies".into(),
            particle_count: 500,
            bounds: 1.5,
            particle_size: 0.03,
            speed: 1.0,
            spatial_cell_size: 0.2,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Cube { size: 2.0 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 0.8,
                    g: 1.0,
                    b: 0.3,
                },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Wander {
                    strength: 0.8,
                    frequency: 2.0,
                },
                RuleConfig::Drag(3.0),
                RuleConfig::SpeedLimit { min: 0.0, max: 0.3 },
                // Pulsing glow effect
                RuleConfig::Custom {
                    code: r#"
// Each firefly has its own phase based on index
let phase = f32(index) * 0.1;
let pulse = sin(uniforms.time * 2.0 + phase) * 0.5 + 0.5;
let glow = pulse * pulse;  // Sharper pulse

// Yellow-green glow
p.color = vec3<f32>(0.8 + glow * 0.2, 1.0, 0.3) * (0.3 + glow * 0.7);
p.scale = 0.5 + glow * 0.5;
"#
                    .into(),
                },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.0, 0.02, 0.05],
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        },
    },
    Preset {
        name: "Tornado",
        description: "Swirling vortex pulling particles upward",
        config: || SimConfig {
            name: "Tornado".into(),
            particle_count: 20000,
            bounds: 2.0,
            particle_size: 0.005,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Plane {
                    width: 2.0,
                    depth: 2.0,
                },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::ByPosition,
                ..Default::default()
            },
            rules: vec![
                // Strong central vortex
                RuleConfig::Vortex {
                    center: [0.0, 0.0, 0.0],
                    axis: [0.0, 1.0, 0.0],
                    strength: 10.0,
                },
                // Pull toward center
                RuleConfig::AttractTo {
                    point: [0.0, 0.0, 0.0],
                    strength: 1.5,
                },
                // Upward lift in center
                RuleConfig::Custom {
                    code: r#"
let dist_xz = length(vec2<f32>(p.position.x, p.position.z));
let lift = max(0.0, 1.0 - dist_xz * 2.0);
p.velocity.y = p.velocity.y + lift * 3.0 * uniforms.delta_time;

// Respawn at bottom when too high
if p.position.y > 1.4 {
    p.position.y = -0.8;
    p.position.x = p.position.x * 0.5;
    p.position.z = p.position.z * 0.5;
}
"#
                    .into(),
                },
                RuleConfig::Drag(1.0),
                RuleConfig::SpeedLimit { min: 0.0, max: 5.0 },
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                velocity_stretch: true,
                velocity_stretch_factor: 2.0,
                trail_length: 8,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        },
    },
];
