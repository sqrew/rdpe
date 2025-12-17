//! Simulation presets

use crate::{
    PostProcessEffect,
    config::{
        BlendModeConfig, ColorMappingConfig, ColorMode, CustomShaderConfig, Falloff,
        FieldConfigEntry, FieldTypeConfig, InitialVelocity, InteractionConfig, MouseConfig,
        PaletteConfig, ParticleFieldDef, ParticleFieldType, ParticleShapeConfig, PostProcessConfig,
        RuleConfig, SimConfig, SpawnConfig, SpawnShape, VertexEffectConfig, VisualsConfig,
        VolumeRenderConfig,
    },
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
                RuleConfig::BounceWalls { restitution: 1.0 },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Explosion",
        description: "Particles exploding outward with gravity",
        config: || SimConfig {
            name: "Explosion".into(),
            particle_count: 50000,
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
                RuleConfig::BounceWalls { restitution: 1.0 },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Fluid Simulation",
        description: "SPH-like fluid with pressure and viscosity",
        config: || SimConfig {
            name: "Fluid Simulation".into(),
            particle_count: 10000,
            bounds: 1.0,
            particle_size: 0.001,
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
                RuleConfig::Gravity(1.0),
                RuleConfig::Pressure {
                    radius: 0.05,
                    strength: 1.0,
                    target_density: 1.0,
                },
                RuleConfig::Viscosity {
                    radius: 2.0,
                    strength: 1.0,
                },
                RuleConfig::BounceWalls { restitution: 1.0 },
                // Write fluid density to field for volume visualization
                RuleConfig::Custom {
                    code: "field_write(0u, p.position, 1.0);".into(),
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
                name: "density".into(),
                resolution: 48,
                extent: 1.1,
                decay: 0.92,
                blur: 0.2,
                blur_iterations: 2,
                field_type: FieldTypeConfig::Scalar,
                custom_update: None,
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 64,
                density_scale: 4.0,
                palette: PaletteConfig::Ocean,
                threshold: 0.02,
                additive: true,
            },
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
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
            visuals: VisualsConfig {
                palette: PaletteConfig::Magma,
                ..Default::default()
            },
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
                custom_update: None,
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 48,
                density_scale: 5.0,
                palette: PaletteConfig::Magma,
                threshold: 0.02,
                additive: true,
            },
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
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
                RuleConfig::BounceWalls { restitution: 1.0 },
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
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Galaxy",
        description: "Spiral galaxy with central bulge, rotating arms, and stellar populations",
        config: || SimConfig {
            name: "Galaxy".into(),
            particle_count: 20000,
            bounds: 2.0,
            particle_size: 0.008,
            speed: 1.0,
            spatial_cell_size: 0.2,
            spatial_resolution: 32,
            particle_fields: vec![ParticleFieldDef {
                name: "custom".into(),
                field_type: ParticleFieldType::F32,
            }],
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.01 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                },
                ..Default::default()
            },
            rules: vec![
                // Central supermassive black hole gravity
                RuleConfig::PointGravity {
                    point: [0.0, 0.0, 0.0],
                    strength: 1.2,
                    softening: 0.1,
                },
                // Initialize spiral structure, orbital velocities, and ongoing dynamics
                RuleConfig::Custom {
                    code: r#"
// One-time initialization
if uniforms.time < 0.02 {
    var seed = f32(index) * 17.31 + 0.5;

    // Determine if bulge (15%), disk (75%), or halo (10%)
    let region_roll = rand(&seed);

    if region_roll < 0.15 {
        // Central bulge - dense spherical core
        let bulge_r = pow(rand(&seed), 0.5) * 0.2;  // Concentrated toward center
        let theta = rand(&seed) * 6.283;
        let phi = acos(2.0 * rand(&seed) - 1.0);
        p.position = vec3<f32>(
            bulge_r * sin(phi) * cos(theta),
            bulge_r * cos(phi) * 0.3,  // Flattened
            bulge_r * sin(phi) * sin(theta)
        );
        p.custom = 0.0;  // Mark as bulge star
    } else if region_roll < 0.9 {
        // Disk with spiral arms
        let arm = floor(rand(&seed) * 2.0);  // 2 main arms
        let arm_offset = arm * 3.14159;

        // Distribute along arm with more stars in middle regions
        let t = rand(&seed);
        let spiral_r = 0.12 + t * 1.1;

        // Tighter spiral winding
        let spiral_theta = arm_offset + t * 5.0 + (rand(&seed) - 0.5) * 0.5;

        // Arm thickness increases slightly outward
        let arm_spread = (rand(&seed) - 0.5) * 0.12 * (0.3 + t * 0.7);
        let final_r = spiral_r + arm_spread;

        p.position = vec3<f32>(
            final_r * cos(spiral_theta),
            (rand(&seed) - 0.5) * 0.04,  // Very thin disk
            final_r * sin(spiral_theta)
        );
        p.custom = 1.0 + t;  // Mark as disk star
    } else {
        // Halo - sparse globular cluster stars
        let halo_r = 0.6 + rand(&seed) * 0.9;
        let theta = rand(&seed) * 6.283;
        let phi = acos(2.0 * rand(&seed) - 1.0);
        p.position = vec3<f32>(
            halo_r * sin(phi) * cos(theta),
            halo_r * cos(phi) * 0.5,
            halo_r * sin(phi) * sin(theta)
        );
        p.custom = 3.0;  // Mark as halo star
    }

    // Set initial circular orbital velocity
    let r = length(vec2<f32>(p.position.x, p.position.z));
    if r > 0.01 {
        // Orbital velocity for circular orbit (with flat rotation curve like real galaxies)
        let v_circ = sqrt(1.2 * r / (r * r + 0.1));
        let angle = atan2(p.position.z, p.position.x);
        p.velocity = vec3<f32>(
            -sin(angle) * v_circ,
            0.0,
            cos(angle) * v_circ
        );
    }
}

// Frame dragging from rotating central mass - falls off with distance
let r_xz = length(vec2<f32>(p.position.x, p.position.z));
if r_xz > 0.01 {
    let frame_drag = 0.15 / (r_xz * r_xz + 0.05);  // Strong near center, drops off
    let tangent = vec3<f32>(-p.position.z, 0.0, p.position.x) / r_xz;
    p.velocity += tangent * frame_drag * uniforms.delta_time;
}

// Keep disk flat
p.velocity.y *= 0.92;
p.position.y *= 0.97;
"#
                    .into(),
                },
                // Color stars by population
                RuleConfig::Custom {
                    code: r#"
let r = length(vec2<f32>(p.position.x, p.position.z));
let star_type = p.custom;

if star_type < 0.5 {
    // Bulge stars - old, yellow/orange population II
    let core_glow = smoothstep(0.25, 0.0, r);
    p.color = mix(
        vec3<f32>(1.0, 0.65, 0.35),  // Orange-red
        vec3<f32>(1.0, 0.9, 0.7),    // Warm white core
        core_glow
    );
    p.scale = 0.7 + core_glow * 0.8;
} else if star_type < 3.0 {
    // Disk stars - color gradient along arms
    let arm_pos = star_type - 1.0;

    // Inner: yellow (old), outer: blue (young star-forming regions)
    if arm_pos < 0.25 {
        let t = arm_pos / 0.25;
        p.color = mix(vec3<f32>(1.0, 0.75, 0.45), vec3<f32>(1.0, 0.9, 0.75), t);
    } else if arm_pos < 0.6 {
        let t = (arm_pos - 0.25) / 0.35;
        p.color = mix(vec3<f32>(1.0, 0.9, 0.75), vec3<f32>(0.85, 0.9, 1.0), t);
    } else {
        let t = (arm_pos - 0.6) / 0.5;
        p.color = mix(vec3<f32>(0.85, 0.9, 1.0), vec3<f32>(0.5, 0.65, 1.0), t);
    }

    p.scale = 0.4 + arm_pos * 0.4;
} else {
    // Halo stars - old, dim red giants
    p.color = vec3<f32>(0.85, 0.55, 0.35) * 0.4;
    p.scale = 0.35;
}

// Subtle brightness variation
let shimmer = 0.85 + sin(f32(index) * 3.7 + uniforms.time * 0.5) * 0.15;
p.color *= shimmer;
"#
                    .into(),
                },
                RuleConfig::Drag(0.02),
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.0, 0.0, 0.015],
                trail_length: 8,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Galaxy Formation",
        description: "Galaxy with central gravity and N-body stellar dynamics",
        config: || SimConfig {
            name: "Galaxy Formation".into(),
            particle_count: 2000,
            bounds: 3.0,
            particle_size: 0.015,
            speed: 1.0,
            spatial_cell_size: 0.3,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                // Start as a disk directly
                shape: SpawnShape::Shell {
                    inner: 0.2,
                    outer: 1.2,
                },
                velocity: InitialVelocity::Swirl { speed: 0.5 },
                color_mode: ColorMode::Uniform {
                    r: 1.0,
                    g: 0.9,
                    b: 0.7,
                },
                ..Default::default()
            },
            rules: vec![
                // Central gravity - pulls everything inward
                RuleConfig::PointGravity {
                    point: [0.0, 0.0, 0.0],
                    strength: 0.8,
                    softening: 0.1,
                },
                // Vortex - provides continuous rotation to balance gravity
                RuleConfig::Vortex {
                    center: [0.0, 0.0, 0.0],
                    axis: [0.0, 1.0, 0.0],
                    strength: 1.2,
                },
                // Weak N-body for local clumping and spiral arms
                RuleConfig::NBodyGravity {
                    strength: 0.03,
                    softening: 0.05,
                    radius: 0.4,
                },
                // Keep it flat and color by radius
                RuleConfig::Custom {
                    code: r#"
// Flatten to disk
p.velocity.y *= 0.92;
p.position.y *= 0.95;

// Color by distance - yellow/white center to blue edge
let r = length(vec2<f32>(p.position.x, p.position.z));
let t = clamp(r / 1.2, 0.0, 1.0);
if t < 0.2 {
    // Bright core
    p.color = vec3<f32>(1.0, 0.95, 0.85);
    p.scale = 1.2;
} else if t < 0.5 {
    let s = (t - 0.2) / 0.3;
    p.color = mix(vec3<f32>(1.0, 0.9, 0.7), vec3<f32>(0.9, 0.85, 0.95), s);
    p.scale = 1.0 - s * 0.2;
} else {
    let s = (t - 0.5) / 0.5;
    p.color = mix(vec3<f32>(0.9, 0.85, 0.95), vec3<f32>(0.6, 0.7, 1.0), s);
    p.scale = 0.8 - s * 0.3;
}
"#
                    .into(),
                },
                RuleConfig::Drag(0.1),
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.0, 0.0, 0.012],
                trail_length: 15,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
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
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
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
                custom_update: None,
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
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
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
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
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
                RuleConfig::BounceWalls { restitution: 1.0 },
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
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
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
                RuleConfig::BounceWalls { restitution: 1.0 },
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
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Plasma Core",
        description: "Pulsating energy core with swirling plasma field and volume rendering",
        config: || SimConfig {
            name: "Plasma Core".into(),
            particle_count: 15000,
            bounds: 1.5,
            particle_size: 0.008,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell {
                    inner: 0.2,
                    outer: 0.6,
                },
                velocity: InitialVelocity::Swirl { speed: 0.4 },
                color_mode: ColorMode::Uniform {
                    r: 0.6,
                    g: 0.8,
                    b: 1.0,
                },
                ..Default::default()
            },
            rules: vec![
                // Soft attraction to center - creates the core
                RuleConfig::PointGravity {
                    point: [0.0, 0.0, 0.0],
                    strength: 0.8,
                    softening: 0.15,
                },
                // Swirling vortex motion around Y axis
                RuleConfig::Vortex {
                    center: [0.0, 0.0, 0.0],
                    axis: [0.0, 1.0, 0.0],
                    strength: 2.0,
                },
                // Turbulent organic motion
                RuleConfig::Curl {
                    scale: 4.0,
                    strength: 0.6,
                },
                // Breathing pulse - expand and contract
                RuleConfig::Pulse {
                    point: [0.0, 0.0, 0.0],
                    strength: 0.4,
                    frequency: 0.5,
                    radius: 0.0,
                },
                RuleConfig::Drag(0.8),
                RuleConfig::SpeedLimit {
                    min: 0.05,
                    max: 1.2,
                },
                RuleConfig::BounceWalls { restitution: 1.0 },
                // Deposit energy to field and color particles
                RuleConfig::Custom {
                    code: r#"
// Distance from center determines energy intensity
let dist = length(p.position);
let core_intensity = smoothstep(0.8, 0.0, dist);
let speed = length(p.velocity);

// Deposit more energy near center and when moving fast
let energy = core_intensity * (0.5 + speed * 0.5);
field_write(0u, p.position, energy);

// Color gradient: hot center (white/cyan) to cool edges (blue/purple)
let t = smoothstep(0.0, 0.6, dist);
var core_color: vec3<f32>;
if t < 0.3 {
    // Inner core: white to cyan
    let blend = t / 0.3;
    core_color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.4, 0.9, 1.0), blend);
} else if t < 0.6 {
    // Mid region: cyan to blue
    let blend = (t - 0.3) / 0.3;
    core_color = mix(vec3<f32>(0.4, 0.9, 1.0), vec3<f32>(0.3, 0.5, 1.0), blend);
} else {
    // Outer region: blue to purple/magenta
    let blend = (t - 0.6) / 0.4;
    core_color = mix(vec3<f32>(0.3, 0.5, 1.0), vec3<f32>(0.7, 0.3, 0.9), blend);
}

// Add shimmer based on speed
let shimmer = sin(uniforms.time * 8.0 + f32(index) * 0.1) * 0.1 + 0.9;
p.color = core_color * shimmer * (0.6 + core_intensity * 0.4);

// Particles near center glow brighter (scale up)
p.scale = 0.5 + core_intensity * 1.0;
"#
                    .into(),
                },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.01, 0.01, 0.03],
                trail_length: 6,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![FieldConfigEntry {
                name: "energy".into(),
                resolution: 48,
                extent: 1.2,
                decay: 0.92,
                blur: 0.25,
                blur_iterations: 2,
                field_type: FieldTypeConfig::Scalar,
                custom_update: None,
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 64,
                density_scale: 5.0,
                palette: PaletteConfig::Plasma,
                threshold: 0.02,
                additive: true,
            },
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Immortal Jellyfish",
        description: "Bioluminescent jellyfish with pulsing bell and flowing tentacles",
        config: || SimConfig {
            name: "Jellyfish".into(),
            particle_count: 6000,
            bounds: 2.0,
            particle_size: 0.015,
            speed: 1.0,
            spatial_cell_size: 0.15,
            spatial_resolution: 32,
            particle_fields: vec![ParticleFieldDef {
                name: "custom".into(),
                field_type: ParticleFieldType::F32,
            }],
            spawn: SpawnConfig {
                shape: SpawnShape::Shell {
                    inner: 0.0,
                    outer: 0.4,
                },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 0.5,
                    g: 0.7,
                    b: 1.0,
                },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Drag(3.0),
                RuleConfig::Custom {
                    code: r#"
// Use index to determine role: first 60% are bell, rest are tentacles
let is_bell = index < u32(f32(arrayLength(&particles)) * 0.6);

// Store random phase in custom field (initialize once)
if uniforms.time < 0.02 {
    var seed = f32(index) * 17.31 + 0.5;
    p.custom = rand(&seed) * 6.283;
}

// Pulsing
let pulse = sin(uniforms.time * 4.0 + p.custom * 0.2) * 0.5 + 0.5;

// Jellyfish center drifts gently
let jelly_center = vec3<f32>(
    sin(uniforms.time * 0.2) * 0.3,
    sin(uniforms.time * 0.15) * 0.2,
    cos(uniforms.time * 0.18) * 0.3
);

if is_bell {
    // === BELL ===
    // Create dome shape based on index
    let bell_count = u32(f32(arrayLength(&particles)) * 0.6);
    let bell_idx = f32(index) / f32(bell_count);

    // Spherical coordinates for dome
    let theta = bell_idx * 6.283 * 13.0 + p.custom;  // Angle around
    let phi_raw = bell_idx * 3.14159 * 0.4;  // 0 to ~72 degrees from top
    let phi = phi_raw + sin(bell_idx * 50.0) * 0.1;  // Add variation

    // Dome radius pulses
    let dome_radius = 0.25 + pulse * 0.08;

    // Convert to cartesian (dome shape - hemisphere facing down)
    let target_x = jelly_center.x + sin(phi) * cos(theta) * dome_radius;
    let target_z = jelly_center.z + sin(phi) * sin(theta) * dome_radius;
    let target_y = jelly_center.y + cos(phi) * dome_radius * 0.6;  // Flatten vertically

    let dest = vec3<f32>(target_x, target_y, target_z);

    // Move toward target position
    p.velocity = (dest - p.position) * 4.0;

    // Color: white center fading to pink/cyan at edges
    let edge = sin(phi) / sin(3.14159 * 0.4);  // 0 at top, 1 at edge
    let glow = 0.6 + pulse * 0.4;
    p.color = mix(
        vec3<f32>(0.9, 0.95, 1.0),
        vec3<f32>(0.9, 0.5, 0.8),
        edge
    ) * glow;
    p.scale = 1.0 - edge * 0.3;

} else {
    // === TENTACLES ===
    let tent_start = u32(f32(arrayLength(&particles)) * 0.6);
    let tent_count = arrayLength(&particles) - tent_start;
    let tent_idx = f32(index - tent_start) / f32(tent_count);

    // 8 tentacle strands
    let strand = floor(tent_idx * 8.0);
    let along = fract(tent_idx * 8.0);  // Position along strand (0=top, 1=bottom)

    // Strand angle
    let strand_angle = strand * 0.785398 + p.custom * 0.1;  // PI/4 spacing

    // Attachment point at bell edge
    let attach_radius = 0.22 + pulse * 0.05;
    let attach = vec3<f32>(
        jelly_center.x + cos(strand_angle) * attach_radius,
        jelly_center.y - 0.05,
        jelly_center.z + sin(strand_angle) * attach_radius
    );

    // Tentacle hangs down with wave motion
    let hang_depth = along * 0.7;
    let wave_time = uniforms.time * 2.0 + along * 3.0 + p.custom;
    let wave_amp = along * 0.15;  // More wave at tips

    let target_x = attach.x + sin(wave_time) * wave_amp;
    let target_z = attach.z + cos(wave_time * 0.7) * wave_amp;
    let target_y = attach.y - hang_depth + pulse * 0.1 * (1.0 - along);

    let dest = vec3<f32>(target_x, target_y, target_z);

    // Tentacles move slower/laggier the further from attachment
    let lag = 2.0 + along * 4.0;
    p.velocity = (dest - p.position) * lag;

    // Color: pink to purple to blue gradient down tentacle
    let glow = 0.4 + pulse * 0.3;
    if along < 0.4 {
        p.color = vec3<f32>(1.0, 0.5, 0.85) * glow;
    } else if along < 0.7 {
        let t = (along - 0.4) / 0.3;
        p.color = mix(vec3<f32>(1.0, 0.5, 0.85), vec3<f32>(0.6, 0.3, 0.9), t) * glow;
    } else {
        let t = (along - 0.7) / 0.3;
        p.color = mix(vec3<f32>(0.6, 0.3, 0.9), vec3<f32>(0.3, 0.4, 0.8), t) * glow;
    }
    p.scale = 0.6 - along * 0.3;
}
"#
                    .into(),
                },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.0, 0.02, 0.08],
                trail_length: 5,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Water Cycle",
        description: "Evaporating water rises, condenses into clouds, and rains back down",
        config: || SimConfig {
            name: "Water Cycle".into(),
            particle_count: 5000,
            bounds: 1.5,
            particle_size: 0.015,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            particle_fields: vec![ParticleFieldDef {
                name: "custom".into(),
                field_type: ParticleFieldType::F32,
            }],
            spawn: SpawnConfig {
                shape: SpawnShape::Plane {
                    width: 2.0,
                    depth: 2.0,
                },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 0.4,
                    g: 0.7,
                    b: 1.0,
                },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Drag(0.3),
                RuleConfig::Custom {
                    code: r#"
// State machine: 0 = water/pool, 1 = vapor (rising), 2 = cloud, 3 = rain (falling)
// Use custom field to track state

// Initialize state randomly on first frame
if uniforms.time < 0.02 {
    var seed = f32(index) * 13.37;
    p.custom = floor(rand(&seed) * 4.0);  // Random initial state
    // Spread out vertically based on state
    p.position.y = -0.9 + rand(&seed) * 1.8;
}

let state = u32(p.custom);

// Ground level and sky level
let ground = -0.9;
let sky = 0.9;

if state == 0u {
    // POOL - sitting at bottom, occasionally evaporates
    p.position.y = ground;
    p.velocity = vec3<f32>(0.0, 0.0, 0.0);

    // Random chance to start evaporating
    var seed = f32(index) * 7.7 + uniforms.time * 2.0;
    if rand(&seed) < 0.005 {
        p.custom = 1.0;  // Become vapor
    }

    // Water color - blue
    p.color = vec3<f32>(0.2, 0.5, 0.9);
    p.scale = 0.8;

} else if state == 1u {
    // VAPOR - rising slowly
    p.velocity.y = 0.4;

    // Drift sideways slightly
    var seed = f32(index) * 3.14 + uniforms.time;
    p.velocity.x = (rand(&seed) - 0.5) * 0.2;
    seed += 1.0;
    p.velocity.z = (rand(&seed) - 0.5) * 0.2;

    // When reaching sky, become cloud
    if p.position.y > sky - 0.2 {
        p.custom = 2.0;
    }

    // Vapor color - light misty blue, semi-transparent look
    let rise_t = (p.position.y - ground) / (sky - ground);
    p.color = mix(vec3<f32>(0.4, 0.6, 0.9), vec3<f32>(0.8, 0.85, 0.95), rise_t);
    p.scale = 0.5 + rise_t * 0.5;

} else if state == 2u {
    // CLOUD - drifting at top
    p.position.y = sky;
    p.velocity.y = 0.0;

    // Drift horizontally
    var seed = f32(index) * 5.5 + uniforms.time * 0.3;
    p.velocity.x = (rand(&seed) - 0.5) * 0.3;
    seed += 2.0;
    p.velocity.z = (rand(&seed) - 0.5) * 0.3;

    // Random chance to start raining
    seed = f32(index) * 11.1 + uniforms.time * 3.0;
    if rand(&seed) < 0.008 {
        p.custom = 3.0;  // Become rain
    }

    // Cloud color - white/gray
    p.color = vec3<f32>(0.9, 0.92, 0.95);
    p.scale = 1.2;

} else {
    // RAIN - falling fast
    p.velocity.y = -1.2;

    // Slight sideways drift
    var seed = f32(index) * 9.9;
    p.velocity.x = (rand(&seed) - 0.5) * 0.1;

    // When hitting ground, become pool water
    if p.position.y < ground + 0.05 {
        p.custom = 0.0;
        p.position.y = ground;
    }

    // Rain color - blue droplets
    p.color = vec3<f32>(0.3, 0.6, 1.0);
    p.scale = 0.6;
}

// Keep in horizontal bounds
if abs(p.position.x) > 1.0 { p.position.x *= 0.95; }
if abs(p.position.z) > 1.0 { p.position.z *= 0.95; }
"#
                    .into(),
                },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Alpha,
                background_color: [0.1, 0.15, 0.25],
                velocity_stretch: true,
                velocity_stretch_factor: 2.0,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Network Contagion",
        description: "Signal spreads through adjacency graph connections",
        config: || SimConfig {
            name: "Network Contagion".into(),
            particle_count: 2000,
            bounds: 1.0,
            particle_size: 0.02,
            speed: 1.0,
            spatial_cell_size: 0.15,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.8 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 0.2,
                    g: 0.3,
                    b: 0.8,
                },
                ..Default::default()
            },
            rules: vec![
                // Gentle separation to spread particles out
                RuleConfig::Separate {
                    radius: 0.08,
                    strength: 0.5,
                },
                // Light cohesion to keep them together
                RuleConfig::Cohere {
                    radius: 0.2,
                    strength: 0.1,
                },
                // Damping to settle
                RuleConfig::Drag(3.0),
                RuleConfig::SpeedLimit { min: 0.0, max: 0.3 },
                RuleConfig::BounceWalls { restitution: 1.0 },
                // Initialize signal for a few "seed" particles
                RuleConfig::Custom {
                    code: r#"
// Seed particles: first 3 particles start with signal
if index < 3u && p.age < 0.1 {
    p.signal = 1.0;
}
"#
                    .into(),
                },
                // Spread signal through adjacency connections
                RuleConfig::Custom {
                    code: r#"
// Propagate signal through the adjacency graph
let my_signal = p.signal;
let adj_count = adjacency_count(index);

// Accumulate signal from infected neighbors
var incoming_signal = 0.0;
for (var i = 0u; i < adj_count; i++) {
    let adj_neighbor_idx = adjacency_neighbor(index, i);
    let adj_neighbor_signal = particles[adj_neighbor_idx].signal;
    if adj_neighbor_signal > 0.5 {
        incoming_signal += 0.02; // Infection rate per neighbor
    }
}

// Apply infection (once infected, stay infected but slowly fade)
if my_signal < 0.5 && incoming_signal > 0.0 {
    p.signal = min(1.0, my_signal + incoming_signal);
} else if my_signal > 0.0 {
    // Slowly decay signal (creates wave effect)
    p.signal = max(0.0, my_signal - 0.002);
}

// Color based on signal level
let t = p.signal;
// Blue (cold) -> Yellow (warming) -> Red (hot) -> dim
if t < 0.01 {
    p.color = vec3<f32>(0.15, 0.2, 0.4); // Uninfected: dark blue
} else if t < 0.5 {
    let s = t * 2.0;
    p.color = mix(vec3<f32>(0.2, 0.3, 0.8), vec3<f32>(1.0, 0.9, 0.2), s);
} else {
    let s = (t - 0.5) * 2.0;
    p.color = mix(vec3<f32>(1.0, 0.9, 0.2), vec3<f32>(1.0, 0.2, 0.1), s);
}

// Pulse size when highly infected
p.scale = 1.0 + p.signal * 0.5;
"#
                    .into(),
                },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Alpha,
                background_color: [0.02, 0.03, 0.06],
                shape: ParticleShapeConfig::Circle,
                connections_enabled: true,
                connections_radius: 0.12,
                connections_color: [0.2, 0.25, 0.4],
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: vec![ParticleFieldDef {
                name: "signal".into(),
                field_type: ParticleFieldType::F32,
            }],
            mouse: MouseConfig::default(),
            adjacency_enabled: true,
            adjacency_max_neighbors: 16,
            adjacency_radius: 0.12,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Chromatic Life",
        description: "Emergent particle ecosystems - types attract and repel creating living patterns",
        config: || {
            use crate::config::{InteractionConfig, RuleMatrixCell};

            // Create interaction matrix for 4 types with interesting dynamics
            let mut interactions = InteractionConfig::with_num_types(4);
            interactions.enabled = true;
            let n = interactions.num_types;

            // Type 0 (Red) - aggressive, chases others
            interactions.matrix[0 * n + 0] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.15,
                strength: 0.5,
            });
            interactions.matrix[0 * n + 1] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.2,
                strength: 2.0,
            });
            interactions.matrix[0 * n + 2] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.15,
                strength: 1.0,
            });
            interactions.matrix[0 * n + 3] = RuleMatrixCell::with_rule(RuleConfig::Separate {
                radius: 0.1,
                strength: 3.0,
            });

            // Type 1 (Green) - flees red, attracted to blue
            interactions.matrix[1 * n + 0] = RuleMatrixCell::with_rule(RuleConfig::Separate {
                radius: 0.25,
                strength: 4.0,
            });
            interactions.matrix[1 * n + 1] = RuleMatrixCell::with_rule(RuleConfig::Cohere {
                radius: 0.15,
                strength: 1.0,
            });
            interactions.matrix[1 * n + 2] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.2,
                strength: 1.5,
            });
            interactions.matrix[1 * n + 3] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.15,
                strength: 0.8,
            });

            // Type 2 (Blue) - orbits around, gentle
            interactions.matrix[2 * n + 0] = RuleMatrixCell::with_rule(RuleConfig::Separate {
                radius: 0.15,
                strength: 2.0,
            });
            interactions.matrix[2 * n + 1] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.18,
                strength: 1.2,
            });
            interactions.matrix[2 * n + 2] = RuleMatrixCell::with_rules(vec![
                RuleConfig::Cohere {
                    radius: 0.2,
                    strength: 0.8,
                },
                RuleConfig::Separate {
                    radius: 0.05,
                    strength: 2.0,
                },
            ]);
            interactions.matrix[2 * n + 3] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.2,
                strength: 2.0,
            });

            // Type 3 (Yellow) - catalyst, attracts everything mildly
            interactions.matrix[3 * n + 0] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.15,
                strength: 1.0,
            });
            interactions.matrix[3 * n + 1] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.15,
                strength: 1.0,
            });
            interactions.matrix[3 * n + 2] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.15,
                strength: 1.0,
            });
            interactions.matrix[3 * n + 3] = RuleMatrixCell::with_rule(RuleConfig::Separate {
                radius: 0.1,
                strength: 3.0,
            });

            SimConfig {
                name: "Chromatic Life".into(),
                particle_count: 8000,
                bounds: 1.2,
                particle_size: 0.008,
                speed: 1.0,
                spatial_cell_size: 0.1,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Sphere { radius: 0.8 },
                    velocity: InitialVelocity::RandomDirection { speed: 0.1 },
                    color_mode: ColorMode::Uniform {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                    },
                    type_weights: vec![1.0, 1.0, 1.0, 1.0],
                    ..Default::default()
                },
                rules: vec![
                    RuleConfig::Drag(1.5),
                    RuleConfig::SpeedLimit {
                        min: 0.05,
                        max: 0.8,
                    },
                    RuleConfig::BounceWalls { restitution: 1.0 },
                    // Color by type
                    RuleConfig::Custom {
                        code: r#"
let t = p.particle_type;
if t == 0u {
    p.color = vec3<f32>(1.0, 0.3, 0.2);  // Red
} else if t == 1u {
    p.color = vec3<f32>(0.2, 1.0, 0.4);  // Green
} else if t == 2u {
    p.color = vec3<f32>(0.3, 0.5, 1.0);  // Blue
} else {
    p.color = vec3<f32>(1.0, 0.9, 0.3);  // Yellow
}
// Add subtle glow based on speed
let spd = length(p.velocity);
p.color *= 0.7 + spd * 0.5;
"#
                        .into(),
                    },
                ],
                vertex_effects: Vec::new(),
                visuals: VisualsConfig {
                    blend_mode: BlendModeConfig::Additive,
                    background_color: [0.02, 0.02, 0.04],
                    trail_length: 12,
                    shape: ParticleShapeConfig::Circle,
                    ..Default::default()
                },
                custom_uniforms: HashMap::new(),
                custom_shaders: CustomShaderConfig::default(),
                fields: Vec::new(),
                volume_render: VolumeRenderConfig::default(),
                particle_fields: Vec::new(),
                mouse: MouseConfig::default(),
                adjacency_enabled: false,
                adjacency_max_neighbors: 32,
                adjacency_radius: 0.1,
                interactions,
                post_process: PostProcessConfig::default(),
                emitters: Vec::new(),
            }
        },
    },
    Preset {
        name: "Event Horizon",
        description: "Black hole with spiraling accretion disk and relativistic jets",
        config: || SimConfig {
            name: "Event Horizon".into(),
            particle_count: 25000,
            bounds: 2.0,
            particle_size: 0.006,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell {
                    inner: 0.6,
                    outer: 1.5,
                },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 1.0,
                    g: 0.8,
                    b: 0.5,
                },
                ..Default::default()
            },
            rules: vec![
                // Strong gravity toward center
                RuleConfig::PointGravity {
                    point: [0.0, 0.0, 0.0],
                    strength: 3.0,
                    softening: 0.15,
                },
                // Initial orbital kick and continuous tangential force
                RuleConfig::Custom {
                    code: r#"
// Give particles orbital velocity (tangential to center)
let to_center = -p.position;
let dist = length(to_center);

if dist > 0.01 {
    let radial = normalize(to_center);
    // Tangent vector (cross with up, or use perpendicular in XZ plane)
    let tangent = normalize(cross(radial, vec3<f32>(0.0, 1.0, 0.0)));

    // Orbital velocity decreases with distance (Keplerian-ish)
    let orbital_speed = 0.8 / sqrt(dist + 0.1);

    // Add slight tangential acceleration to maintain orbit
    p.velocity += tangent * orbital_speed * 0.1 * uniforms.delta_time;

    // Flatten toward disk plane (reduce Y velocity)
    p.velocity.y *= 0.98;
    p.position.y *= 0.995;
}

// Event horizon - particles that get too close respawn at edge
if dist < 0.12 {
    // Respawn in outer disk
    var seed = f32(index) * 7.77 + uniforms.time;
    let angle = seed * 6.28;
    let spawn_dist = 1.0 + fract(seed * 3.33) * 0.5;
    p.position = vec3<f32>(cos(angle) * spawn_dist, 0.0, sin(angle) * spawn_dist);
    p.velocity = vec3<f32>(0.0);
}
"#
                    .into(),
                },
                // Color based on distance and speed - hot near center
                RuleConfig::Custom {
                    code: r#"
let col_dist = length(p.position);
let speed = length(p.velocity);

// Temperature gradient: white-hot center to red outer
let temp = clamp(1.0 - col_dist * 0.5, 0.0, 1.0);
let temp2 = temp * temp;

// Inner: white/blue-white, middle: yellow/orange, outer: red/dim
var col: vec3<f32>;
if temp > 0.7 {
    // Inner disk - blue-white hot
    let t = (temp - 0.7) / 0.3;
    col = mix(vec3<f32>(1.0, 0.9, 0.6), vec3<f32>(0.9, 0.95, 1.0), t);
} else if temp > 0.3 {
    // Middle disk - yellow to orange
    let t = (temp - 0.3) / 0.4;
    col = mix(vec3<f32>(1.0, 0.4, 0.1), vec3<f32>(1.0, 0.9, 0.6), t);
} else {
    // Outer disk - red to dim
    let t = temp / 0.3;
    col = mix(vec3<f32>(0.3, 0.05, 0.02), vec3<f32>(1.0, 0.4, 0.1), t);
}

// Boost brightness with speed (doppler-ish effect)
col *= 0.7 + speed * 0.8;

// Particles very close to center glow intensely
if col_dist < 0.2 {
    col *= 1.5;
}

p.color = col;
p.scale = 0.5 + temp * 0.8;
"#
                    .into(),
                },
                RuleConfig::Drag(0.3),
                RuleConfig::SpeedLimit { min: 0.0, max: 3.0 },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.0, 0.0, 0.02],
                trail_length: 15,
                velocity_stretch: true,
                velocity_stretch_factor: 1.5,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig::default(),
            emitters: Vec::new(),
        },
    },
    Preset {
        name: "Electromagnetism",
        description: "Charged particles with Coulomb attraction/repulsion - like charges repel, opposites attract",
        config: || {
            use crate::config::{
                FieldTypeConfig, GlyphColorModeConfig, GlyphModeConfig, GlyphsConfig,
                InteractionConfig, PostProcessEffect, RuleMatrixCell,
            };

            // Create interaction matrix for 2 types: positive and negative charges
            let mut interactions = InteractionConfig::with_num_types(2);
            interactions.enabled = true;
            let n = interactions.num_types;

            // Set type names and colors
            interactions.type_info[0].name = "Positive".to_string();
            interactions.type_info[0].color = [1.0, 0.3, 0.3]; // Red
            interactions.type_info[1].name = "Negative".to_string();
            interactions.type_info[1].color = [0.3, 0.5, 1.0]; // Blue

            // Positive + Positive: Repel (like charges)
            interactions.matrix[0 * n + 0] = RuleMatrixCell::with_rule(RuleConfig::Separate {
                radius: 0.25,
                strength: 3.0,
            });

            // Positive + Negative: Attract (opposite charges)
            interactions.matrix[0 * n + 1] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.4,
                strength: 2.0,
            });

            // Negative + Positive: Attract (opposite charges)
            interactions.matrix[1 * n + 0] = RuleMatrixCell::with_rule(RuleConfig::Attract {
                radius: 0.4,
                strength: 2.0,
            });

            // Negative + Negative: Repel (like charges)
            interactions.matrix[1 * n + 1] = RuleMatrixCell::with_rule(RuleConfig::Separate {
                radius: 0.25,
                strength: 3.0,
            });

            SimConfig {
                name: "Electromagnetism".into(),
                particle_count: 3000,
                bounds: 1.5,
                particle_size: 0.012,
                speed: 1.0,
                spatial_cell_size: 0.15,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Sphere { radius: 1.0 },
                    velocity: InitialVelocity::RandomDirection { speed: 0.15 },
                    color_mode: ColorMode::Uniform {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                    },
                    type_weights: vec![1.0, 1.0], // Equal positive and negative
                    ..Default::default()
                },
                rules: vec![
                    RuleConfig::Drag(0.8),
                    RuleConfig::SpeedLimit {
                        min: 0.0,
                        max: 10.0,
                    },
                    RuleConfig::BounceWalls { restitution: 0.9 },
                    // Color by charge type and add glow based on speed
                    RuleConfig::Custom {
                        code: r#"
// Color by charge: red = positive, blue = negative
if p.particle_type == 0u {
    p.color = vec3<f32>(1.0, 0.2, 0.15);  // Positive - red
} else {
    p.color = vec3<f32>(0.15, 0.4, 1.0);  // Negative - blue
}
// Brighten fast-moving particles (excited charges)
let spd = length(p.velocity);
p.color *= 0.6 + spd * 0.8;

// Write charge to field: positive = +1, negative = -1
let charge = select(-1.0, 1.0, p.particle_type == 0u);
field_write(0u, p.position, charge * 0.5);
"#
                        .into(),
                    },
                ],
                vertex_effects: Vec::new(),
                visuals: VisualsConfig {
                    blend_mode: BlendModeConfig::Additive,
                    background_color: [0.02, 0.02, 0.06],
                    shape: ParticleShapeConfig::Circle,
                    trail_length: 8,
                    connections_enabled: false,
                    glyphs: GlyphsConfig {
                        mode: GlyphModeConfig::VectorField { field_index: 0 },
                        grid_resolution: 8,
                        scale: 0.15,
                        color_mode: GlyphColorModeConfig::ByMagnitude,
                        color: [0.5, 0.8, 1.0],
                    },
                    ..Default::default()
                },
                custom_uniforms: HashMap::new(),
                custom_shaders: CustomShaderConfig::default(),
                fields: vec![FieldConfigEntry {
                    name: "electric_field".into(),
                    field_type: FieldTypeConfig::Vector,
                    resolution: 16,
                    extent: 1.5,
                    decay: 0.92,
                    blur: 0.2,
                    blur_iterations: 1,
                    custom_update: None,
                }],
                volume_render: VolumeRenderConfig::default(),
                particle_fields: Vec::new(),
                mouse: MouseConfig::default(),
                adjacency_enabled: false,
                adjacency_max_neighbors: 32,
                adjacency_radius: 0.15,
                interactions,
                post_process: PostProcessConfig {
                    enabled: true,
                    effects: vec![
                        PostProcessEffect::Bloom {
                            intensity: 0.6,
                            threshold: 0.4,
                            radius: 0.004,
                        },
                        PostProcessEffect::ChromaticAberration {
                            intensity: 0.15,
                            radial: true,
                        },
                    ],
                },
                emitters: Vec::new(),
            }
        },
    },
    Preset {
        name: "Neural Pulse",
        description: "Neurons fire and propagate signals through synaptic connections",
        config: || {
            use crate::config::PostProcessEffect;

            SimConfig {
                name: "Neural Pulse".into(),
                particle_count: 1500,
                bounds: 1.2,
                particle_size: 0.018,
                speed: 1.0,
                spatial_cell_size: 0.15,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Sphere { radius: 1.0 },
                    velocity: InitialVelocity::Zero,
                    color_mode: ColorMode::Uniform {
                        r: 0.2,
                        g: 0.4,
                        b: 0.8,
                    },
                    ..Default::default()
                },
                // Custom particle fields for neural state
                particle_fields: vec![
                    ParticleFieldDef {
                        name: "activation".into(),
                        field_type: ParticleFieldType::F32,
                    },
                    ParticleFieldDef {
                        name: "refractory".into(),
                        field_type: ParticleFieldType::F32,
                    },
                ],
                rules: vec![
                    // Slight drift to keep network dynamic
                    RuleConfig::Wander {
                        strength: 0.02,
                        frequency: 0.5,
                    },
                    RuleConfig::Drag(3.0),
                    RuleConfig::BounceWalls { restitution: 0.5 },
                    // Neurons gently repel to spread out
                    RuleConfig::Separate {
                        radius: 0.08,
                        strength: 0.5,
                    },
                    // Neural dynamics - decay, random firing, refractory countdown
                    RuleConfig::Custom {
                        code: r#"
// Slow decay - signals persist long enough to propagate
p.activation *= 0.96;

// Countdown refractory period
p.refractory = max(0.0, p.refractory - uniforms.delta_time);

// Pacemaker neurons fire spontaneously (only a few per frame)
// Use particle index to make some neurons more likely to be pacemakers
let is_pacemaker = (index % 50u) == 0u;
let hash = (index * 1103515245u + u32(uniforms.time * 500.0)) ^ (index << 13u);
let rand = f32(hash & 0xFFFFu) / 65535.0;
let fire_chance = select(0.0002, 0.008, is_pacemaker);
if rand < fire_chance && p.refractory <= 0.0 && p.activation < 0.3 {
    p.activation = 1.0;
    p.refractory = 0.5;  // Immediately go refractory after spontaneous fire
}
"#
                        .into(),
                    },
                    // Propagate signal to neighbors
                    RuleConfig::NeighborCustom {
                        code: r#"
// Receive signals from firing neighbors
if neighbor_dist < 0.15 && neighbor_dist > 0.01 {
    // If neighbor just fired and we're receptive
    if other.activation > 0.8 && other.refractory > 0.4 && p.refractory <= 0.0 {
        // Receive signal with distance falloff and slight delay via probability
        let signal_strength = (1.0 - neighbor_dist / 0.15);
        let delay_hash = (index * 374761393u + u32(uniforms.time * 200.0)) ^ (index >> 3u);
        let delay_rand = f32(delay_hash & 0xFFu) / 255.0;
        // Only propagate with probability based on signal strength
        if delay_rand < signal_strength * 0.7 {
            p.activation = max(p.activation, 0.85 + delay_rand * 0.15);
        }
    }
}
"#
                        .into(),
                    },
                    // Post-fire: enter refractory period
                    RuleConfig::Custom {
                        code: r#"
// If we reached threshold, enter refractory period
if p.activation > 0.8 && p.refractory <= 0.0 {
    p.activation = 1.0;  // Ensure full fire
    p.refractory = 0.5;  // Refractory period
}

// Clamp activation
p.activation = clamp(p.activation, 0.0, 1.0);

// Color based on neural state
let base_color = vec3<f32>(0.1, 0.15, 0.35);  // Dark blue resting
let fire_color = vec3<f32>(1.0, 0.9, 0.4);    // Bright yellow firing
let refract_color = vec3<f32>(0.5, 0.15, 0.4); // Magenta refractory

var col = base_color;
if p.activation > 0.2 {
    // Firing - interpolate to bright
    let t = (p.activation - 0.2) / 0.8;
    col = mix(base_color, fire_color, t * t);
}
if p.refractory > 0.0 {
    // In refractory - blend to magenta
    let r = p.refractory / 0.5;
    col = mix(col, refract_color, r * 0.7);
}

p.color = col;
p.scale = 0.7 + p.activation * 0.8;
"#
                        .into(),
                    },
                ],
                vertex_effects: Vec::new(),
                visuals: VisualsConfig {
                    blend_mode: BlendModeConfig::Additive,
                    background_color: [0.02, 0.02, 0.05],
                    shape: ParticleShapeConfig::Circle,
                    trail_length: 0,
                    connections_enabled: true,
                    connections_radius: 0.15,
                    connections_color: [0.08, 0.12, 0.25],
                    ..Default::default()
                },
                custom_uniforms: HashMap::new(),
                custom_shaders: CustomShaderConfig::default(),
                fields: Vec::new(),
                volume_render: VolumeRenderConfig::default(),
                mouse: MouseConfig::default(),
                adjacency_enabled: true,
                adjacency_max_neighbors: 16,
                adjacency_radius: 0.15,
                interactions: InteractionConfig::default(),
                post_process: PostProcessConfig {
                    enabled: true,
                    effects: vec![PostProcessEffect::Bloom {
                        intensity: 0.7,
                        threshold: 0.3,
                        radius: 0.005,
                    }],
                },
                emitters: Vec::new(),
            }
        },
    },
    Preset {
        name: "Reaction Diffusion",
        description: "Gray-Scott reaction-diffusion creating Turing patterns",
        config: || {
            use crate::config::PostProcessEffect;

            SimConfig {
                name: "Reaction Diffusion".into(),
                particle_count: 8000,
                bounds: 1.0,
                particle_size: 0.025,
                speed: 1.0,
                spatial_cell_size: 0.05,
                spatial_resolution: 64,
                spawn: SpawnConfig {
                    // Flat plane for 2D-like pattern formation
                    shape: SpawnShape::Plane {
                        width: 1.8,
                        depth: 1.8,
                    },
                    velocity: InitialVelocity::Zero,
                    color_mode: ColorMode::Uniform {
                        r: 0.1,
                        g: 0.2,
                        b: 0.4,
                    },
                    ..Default::default()
                },
                // Chemical concentrations: u (substrate), v (activator)
                particle_fields: vec![
                    ParticleFieldDef {
                        name: "u".into(),
                        field_type: ParticleFieldType::F32,
                    },
                    ParticleFieldDef {
                        name: "v".into(),
                        field_type: ParticleFieldType::F32,
                    },
                    ParticleFieldDef {
                        name: "u_laplacian".into(),
                        field_type: ParticleFieldType::F32,
                    },
                    ParticleFieldDef {
                        name: "v_laplacian".into(),
                        field_type: ParticleFieldType::F32,
                    },
                ],
                rules: vec![
                    // Initialize concentrations (runs every frame but only changes uninitialized particles)
                    RuleConfig::Custom {
                        code: r#"
// Initialize if not yet set (u and v both start at 0)
if p.u < 0.01 && p.v < 0.01 {
    p.u = 1.0;
    p.v = 0.0;

    // Seed some initial v patches based on position
    let seed_hash = u32((p.position.x + 1.0) * 500.0) ^ u32((p.position.z + 1.0) * 500.0);
    let seed_rand = f32(seed_hash & 0xFFu) / 255.0;

    // Create several seed regions spread across the plane
    let d1 = length(p.position.xz - vec2<f32>(0.3, 0.2));
    let d2 = length(p.position.xz - vec2<f32>(-0.3, -0.3));
    let d3 = length(p.position.xz - vec2<f32>(-0.4, 0.4));
    let d4 = length(p.position.xz - vec2<f32>(0.35, -0.35));
    let d5 = length(p.position.xz);

    let in_seed = d1 < 0.15 || d2 < 0.12 || d3 < 0.1 || d4 < 0.11 || d5 < 0.18;
    if in_seed {
        p.v = 0.25 + seed_rand * 0.25;
        p.u = 0.5;
        // Color seeds immediately so they're visible
        p.color = vec3<f32>(0.9, 0.7, 0.3);
    } else {
        p.color = vec3<f32>(0.02, 0.05, 0.15);
    }
}

// Keep particles flat on the plane
p.velocity.y = 0.0;
p.position.y = 0.0;
"#
                        .into(),
                    },
                    // Very gentle repulsion to keep particles spread evenly
                    RuleConfig::Separate {
                        radius: 0.04,
                        strength: 0.3,
                    },
                    RuleConfig::Drag(8.0),
                    // Compute Laplacian (diffusion term) from neighbors
                    RuleConfig::NeighborCustom {
                        code: r#"
if neighbor_dist < 0.08 && neighbor_dist > 0.001 {
    // Weight by distance (closer neighbors contribute more)
    let weight = 1.0 - neighbor_dist / 0.08;

    // Accumulate difference from neighbors (discrete Laplacian)
    p.u_laplacian += (other.u - p.u) * weight;
    p.v_laplacian += (other.v - p.v) * weight;
}
"#
                        .into(),
                    },
                    // Gray-Scott reaction-diffusion dynamics
                    RuleConfig::Custom {
                        code: r#"
// Gray-Scott parameters - these create different patterns:
// f=0.055, k=0.062 -> mitosis (splitting spots)
// f=0.030, k=0.057 -> coral growth
// f=0.025, k=0.060 -> maze/labyrinth
// f=0.078, k=0.061 -> spots
let f = 0.055;  // feed rate
let k = 0.062;  // kill rate

// Diffusion rates (u diffuses faster than v)
let Du = 0.4;
let Dv = 0.2;

// Scale factor for simulation speed
let dt = uniforms.delta_time * 12.0;

// Reaction: u + 2v -> 3v (autocatalysis)
let uvv = p.u * p.v * p.v;

// Gray-Scott equations
let du = Du * p.u_laplacian - uvv + f * (1.0 - p.u);
let dv = Dv * p.v_laplacian + uvv - (f + k) * p.v;

// Update concentrations
p.u = clamp(p.u + du * dt, 0.0, 1.0);
p.v = clamp(p.v + dv * dt, 0.0, 1.0);

// Reset laplacians for next frame
p.u_laplacian = 0.0;
p.v_laplacian = 0.0;

// Color based on v concentration (the activator creates the pattern)
let v_norm = clamp(p.v * 2.5, 0.0, 1.0);

// Beautiful color gradient: deep blue -> teal -> yellow -> white
var col: vec3<f32>;
if v_norm < 0.33 {
    let t = v_norm / 0.33;
    col = mix(vec3<f32>(0.02, 0.05, 0.15), vec3<f32>(0.0, 0.4, 0.5), t);
} else if v_norm < 0.66 {
    let t = (v_norm - 0.33) / 0.33;
    col = mix(vec3<f32>(0.0, 0.4, 0.5), vec3<f32>(0.9, 0.7, 0.2), t);
} else {
    let t = (v_norm - 0.66) / 0.34;
    col = mix(vec3<f32>(0.9, 0.7, 0.2), vec3<f32>(1.0, 1.0, 0.95), t);
}

p.color = col;
p.scale = 0.8 + v_norm * 0.5;
"#
                        .into(),
                    },
                    RuleConfig::BounceWalls { restitution: 0.0 },
                ],
                vertex_effects: Vec::new(),
                visuals: VisualsConfig {
                    blend_mode: BlendModeConfig::Alpha,
                    background_color: [0.01, 0.02, 0.05],
                    shape: ParticleShapeConfig::Circle,
                    trail_length: 0,
                    ..Default::default()
                },
                custom_uniforms: HashMap::new(),
                custom_shaders: CustomShaderConfig::default(),
                fields: Vec::new(),
                volume_render: VolumeRenderConfig::default(),
                mouse: MouseConfig::default(),
                adjacency_enabled: true,
                adjacency_max_neighbors: 32,
                adjacency_radius: 0.08,
                interactions: InteractionConfig::default(),
                post_process: PostProcessConfig {
                    enabled: true,
                    effects: vec![PostProcessEffect::Bloom {
                        intensity: 0.3,
                        threshold: 0.5,
                        radius: 0.003,
                    }],
                },
                emitters: Vec::new(),
            }
        },
    },
    Preset {
        name: "Black Hole",
        description: "Accretion disk spiraling into a black hole with relativistic jets",
        config: || {
            use crate::config::PostProcessEffect;

            SimConfig {
                name: "Black Hole".into(),
                particle_count: 25000,
                bounds: 2.0,
                particle_size: 0.012,
                speed: 1.0,
                spatial_cell_size: 0.1,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    // Spawn in a thick disk around the center
                    shape: SpawnShape::Ring {
                        radius: 0.8,
                        thickness: 0.5,
                    },
                    // Initial tangential velocity for orbital motion
                    velocity: InitialVelocity::RandomDirection { speed: 0.6 },
                    color_mode: ColorMode::Uniform {
                        r: 1.0,
                        g: 0.6,
                        b: 0.2,
                    },
                    ..Default::default()
                },
                particle_fields: vec![
                    ParticleFieldDef {
                        name: "heat".into(),
                        field_type: ParticleFieldType::F32,
                    },
                    ParticleFieldDef {
                        name: "in_jet".into(),
                        field_type: ParticleFieldType::F32,
                    },
                ],
                rules: vec![
                    // Initialize with tangential velocity for orbit
                    RuleConfig::Custom {
                        code: r#"
// Set up initial orbital velocity on spawn
if length(p.velocity) < 0.1 || p.heat < 0.01 {
    let to_center = -p.position;
    let dist = length(to_center);

    // Tangent vector (perpendicular to radial, in xz plane)
    let tangent = normalize(vec3<f32>(-to_center.z, 0.0, to_center.x));

    // Orbital speed - faster closer to center (Keplerian-ish)
    let orbital_speed = 1.2 / sqrt(max(dist, 0.1));
    p.velocity = tangent * orbital_speed;

    // Flatten to disk with some thickness
    p.position.y *= 0.15;
    p.velocity.y *= 0.1;

    // Initialize heat based on distance
    p.heat = clamp(1.0 - dist, 0.1, 1.0);
    p.in_jet = 0.0;
}
"#
                        .into(),
                    },
                    // Massive central gravity
                    RuleConfig::Custom {
                        code: r#"
let to_center = -p.position;
let dist = length(to_center);
let dir = to_center / max(dist, 0.001);

// Strong gravity with 1/r^2 falloff
let gravity_strength = 2.5 / max(dist * dist, 0.01);
p.velocity += dir * gravity_strength * uniforms.delta_time;

// Increase heat as particle falls inward
p.heat = clamp(0.3 / max(dist, 0.05), 0.0, 1.5);

// Event horizon - particles that get too close
if dist < 0.08 {
    // 20% chance to become a jet particle
    let hash = u32(p.position.x * 10000.0) ^ u32(p.position.z * 10000.0) ^ u32(uniforms.time * 100.0);
    let rand = f32(hash & 0xFFu) / 255.0;

    if rand < 0.2 {
        // Launch into jet!
        p.in_jet = 1.0;
        let jet_dir = select(-1.0, 1.0, rand > 0.1);
        p.position = vec3<f32>(
            (rand - 0.1) * 0.06,
            jet_dir * 0.1,
            (f32((hash >> 8u) & 0xFFu) / 255.0 - 0.5) * 0.06
        );
        p.velocity = vec3<f32>(0.0, jet_dir * 4.0, 0.0);
        p.heat = 1.5;
    } else {
        // Respawn in outer disk
        let angle = rand * 6.28318;
        let r = 0.6 + rand * 0.4;
        p.position = vec3<f32>(cos(angle) * r, (rand - 0.5) * 0.1, sin(angle) * r);
        p.velocity = vec3<f32>(0.0);
        p.heat = 0.1;
    }
}
"#
                        .into(),
                    },
                    // Jet particle dynamics
                    RuleConfig::Custom {
                        code: r#"
if p.in_jet > 0.5 {
    // Jets spread slightly and decelerate
    let spread = 0.3 * uniforms.delta_time;
    p.velocity.x += (p.position.x) * spread;
    p.velocity.z += (p.position.z) * spread;
    p.velocity.y *= 0.995;  // Slight deceleration

    // Cool down
    p.heat *= 0.99;

    // Return to disk when jet particle slows/cools
    if abs(p.velocity.y) < 0.5 || abs(p.position.y) > 1.8 {
        p.in_jet = 0.0;
        let hash = u32(p.position.x * 1000.0 + uniforms.time * 500.0);
        let rand = f32(hash & 0xFFu) / 255.0;
        let angle = rand * 6.28318;
        let r = 0.5 + rand * 0.5;
        p.position = vec3<f32>(cos(angle) * r, 0.0, sin(angle) * r);
        p.velocity = vec3<f32>(0.0);
    }
}
"#
                        .into(),
                    },
                    // Slight drag to help disk formation
                    RuleConfig::Drag(0.3),
                    // Color by heat
                    RuleConfig::Custom {
                        code: r#"
// Hot = white/blue, warm = orange, cool = red/dark
let h = clamp(p.heat, 0.0, 1.5);

var col: vec3<f32>;
if h < 0.3 {
    // Cool outer disk: dark red
    let t = h / 0.3;
    col = mix(vec3<f32>(0.15, 0.02, 0.0), vec3<f32>(0.6, 0.1, 0.0), t);
} else if h < 0.7 {
    // Warm middle: red -> orange -> yellow
    let t = (h - 0.3) / 0.4;
    col = mix(vec3<f32>(0.6, 0.1, 0.0), vec3<f32>(1.0, 0.7, 0.1), t);
} else if h < 1.0 {
    // Hot inner: yellow -> white
    let t = (h - 0.7) / 0.3;
    col = mix(vec3<f32>(1.0, 0.7, 0.1), vec3<f32>(1.0, 1.0, 0.9), t);
} else {
    // Jets: white -> blue
    let t = min((h - 1.0) / 0.5, 1.0);
    col = mix(vec3<f32>(1.0, 1.0, 0.9), vec3<f32>(0.6, 0.8, 1.0), t);
}

// Boost brightness for jet particles
if p.in_jet > 0.5 {
    col = mix(col, vec3<f32>(0.7, 0.85, 1.0), 0.5);
}

p.color = col;
p.scale = 0.6 + h * 0.5;
"#
                        .into(),
                    },
                ],
                vertex_effects: Vec::new(),
                visuals: VisualsConfig {
                    blend_mode: BlendModeConfig::Additive,
                    background_color: [0.0, 0.0, 0.02],
                    shape: ParticleShapeConfig::Circle,
                    trail_length: 8,
                    ..Default::default()
                },
                custom_uniforms: HashMap::new(),
                custom_shaders: CustomShaderConfig::default(),
                fields: Vec::new(),
                volume_render: VolumeRenderConfig::default(),
                mouse: MouseConfig::default(),
                adjacency_enabled: false,
                adjacency_max_neighbors: 32,
                adjacency_radius: 0.1,
                interactions: InteractionConfig::default(),
                post_process: PostProcessConfig {
                    enabled: true,
                    effects: vec![
                        PostProcessEffect::Bloom {
                            intensity: 1.2,
                            threshold: 0.2,
                            radius: 0.008,
                        },
                        PostProcessEffect::ChromaticAberration {
                            intensity: 0.003,
                            radial: true,
                        },
                    ],
                },
                emitters: Vec::new(),
            }
        },
    },
    Preset {
        name: "Murmuration",
        description: "Starling-like flocking with dramatic swooping formations",
        config: || {
            SimConfig {
                name: "Murmuration".into(),
                particle_count: 10000,
                bounds: 5.0,
                particle_size: 0.01,
                speed: 1.0,
                spatial_cell_size: 0.2,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Sphere { radius: 1.0 },
                    velocity: InitialVelocity::RandomDirection { speed: 1.0 },
                    color_mode: ColorMode::Uniform {
                        r: 0.08,
                        g: 0.06,
                        b: 0.05,
                    },
                    ..Default::default()
                },
                particle_fields: vec![
                    ParticleFieldDef {
                        name: "wander_offset".into(),
                        field_type: ParticleFieldType::F32,
                    },
                ],
                rules: vec![
                    // Initialize wander offset for variety
                    RuleConfig::Custom {
                        code: r#"
if p.wander_offset < 0.01 {
    let hash = index * 1103515245u + 12345u;
    p.wander_offset = f32(hash & 0xFFFFu) / 65535.0 * 6.28318;
}
"#
                        .into(),
                    },
                    // Core flocking behavior
                    RuleConfig::Separate {
                        radius: 0.08,
                        strength: 3.0,
                    },
                    RuleConfig::Cohere {
                        radius: 0.4,
                        strength: 1.2,
                    },
                    RuleConfig::Align {
                        radius: 0.25,
                        strength: 2.5,
                    },
                    // Moving attractor creates the swooping motion
                    RuleConfig::Custom {
                        code: r#"
// Multiple moving attractor points that orbit and bob
let t = uniforms.time * 0.4;

// Primary attractor - large slow orbit
let a1 = vec3<f32>(
    sin(t) * 1.2,
    sin(t * 1.3) * 0.4,
    cos(t) * 1.2
);

// Secondary attractor - faster, offset orbit
let a2 = vec3<f32>(
    cos(t * 1.7 + 2.0) * 0.9,
    cos(t * 0.9) * 0.5,
    sin(t * 1.7 + 2.0) * 0.9
);

// Tertiary attractor - vertical figure-8
let a3 = vec3<f32>(
    sin(t * 0.8) * 0.6,
    sin(t * 1.6) * 0.8,
    cos(t * 0.8) * 0.6
);

// Find nearest attractor
let d1 = length(p.position - a1);
let d2 = length(p.position - a2);
let d3 = length(p.position - a3);

var goal = a1;
var dist = d1;
if d2 < dist {
    goal = a2;
    dist = d2;
}
if d3 < dist {
    goal = a3;
    dist = d3;
}

// Gentle attraction to nearest goal
let to_goal = normalize(goal - p.position);
let attract_strength = 0.8;
p.velocity += to_goal * attract_strength * uniforms.delta_time;

// Add slight wander for organic feel
let wander_t = uniforms.time * 2.0 + p.wander_offset;
let wander = vec3<f32>(
    sin(wander_t * 1.1) * 0.15,
    sin(wander_t * 1.3 + 1.0) * 0.1,
    sin(wander_t * 0.9 + 2.0) * 0.15
);
p.velocity += wander * uniforms.delta_time;
"#
                        .into(),
                    },
                    // Speed limits for natural motion
                    RuleConfig::SpeedLimit { min: 0.5, max: 1.8 },
                    // Gentle drag
                    RuleConfig::Drag(0.5),
                    // Soft boundary - steer away from edges
                    RuleConfig::Custom {
                        code: r#"
let edge = 2.5;
let margin = 0.8;
let turn_strength = 2.0;

if p.position.x > edge - margin {
    p.velocity.x -= turn_strength * uniforms.delta_time;
}
if p.position.x < -edge + margin {
    p.velocity.x += turn_strength * uniforms.delta_time;
}
if p.position.y > edge - margin {
    p.velocity.y -= turn_strength * uniforms.delta_time;
}
if p.position.y < -edge + margin {
    p.velocity.y += turn_strength * uniforms.delta_time;
}
if p.position.z > edge - margin {
    p.velocity.z -= turn_strength * uniforms.delta_time;
}
if p.position.z < -edge + margin {
    p.velocity.z += turn_strength * uniforms.delta_time;
}
"#
                        .into(),
                    },
                    // Leader particles follow attractors directly
                    RuleConfig::Custom {
                        code: r#"
// First 3 particles are leaders - one for each attractor
let leader_t = uniforms.time * 0.4;

if index == 0u {
    // Leader 1 follows primary attractor
    let a1 = vec3<f32>(sin(leader_t) * 1.2, sin(leader_t * 1.3) * 0.4, cos(leader_t) * 1.2);
    p.velocity = (a1 - p.position) * 3.0;
}
if index == 1u {
    // Leader 2 follows secondary attractor
    let a2 = vec3<f32>(cos(leader_t * 1.7 + 2.0) * 0.9, cos(leader_t * 0.9) * 0.5, sin(leader_t * 1.7 + 2.0) * 0.9);
    p.velocity = (a2 - p.position) * 3.0;
}
if index == 2u {
    // Leader 3 follows tertiary attractor
    let a3 = vec3<f32>(sin(leader_t * 0.8) * 0.6, sin(leader_t * 1.6) * 0.8, cos(leader_t * 0.8) * 0.6);
    p.velocity = (a3 - p.position) * 3.0;
}
"#
                        .into(),
                    },
                    // Subtle color variation based on local density/speed
                    RuleConfig::Custom {
                        code: r#"
// Leaders are larger and colored differently
if index < 3u {
    p.color = vec3<f32>(0.95, 0.6, 0.2);  // Warm orange
    p.scale = 3.0;
} else {
    let speed = length(p.velocity);
    let speed_factor = clamp((speed - 0.5) / 1.3, 0.0, 1.0);

    // Dark birds with subtle variation
    // Faster = slightly lighter (catching light)
    let base = vec3<f32>(0.06, 0.05, 0.04);
    let highlight = vec3<f32>(0.25, 0.22, 0.2);
    p.color = mix(base, highlight, speed_factor * 0.4);

    // Slight scale variation
    p.scale = 0.8 + speed_factor * 0.3;
}
"#
                        .into(),
                    },
                ],
                vertex_effects: Vec::new(),
                visuals: VisualsConfig {
                    blend_mode: BlendModeConfig::Alpha,
                    // Dusk sky gradient background
                    background_color: [0.4, 0.25, 0.2],
                    shape: ParticleShapeConfig::Circle,
                    trail_length: 3,
                    ..Default::default()
                },
                custom_uniforms: HashMap::new(),
                custom_shaders: CustomShaderConfig::default(),
                fields: Vec::new(),
                volume_render: VolumeRenderConfig::default(),
                mouse: MouseConfig::default(),
                adjacency_enabled: false,
                adjacency_max_neighbors: 32,
                adjacency_radius: 0.4,
                interactions: InteractionConfig::default(),
                post_process: PostProcessConfig { enabled: true, effects: vec![PostProcessEffect::default_edge_glow()] },
                emitters: Vec::new(),
            }
        },
    },
    Preset {
        name: "Lava Lamp",
        description: "Rising and falling blobs with temperature-based buoyancy",
        config: || {
            use crate::config::PostProcessEffect;

            SimConfig {
                name: "Lava Lamp".into(),
                particle_count: 3000,
                bounds: 1.5,
                particle_size: 0.04,
                speed: 1.0,
                spatial_cell_size: 0.12,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    // Start scattered in a tall column
                    shape: SpawnShape::Cube { size: 0.8 },
                    velocity: InitialVelocity::Zero,
                    color_mode: ColorMode::Uniform {
                        r: 0.9,
                        g: 0.3,
                        b: 0.1,
                    },
                    ..Default::default()
                },
                particle_fields: vec![ParticleFieldDef {
                    name: "temp".into(),
                    field_type: ParticleFieldType::F32,
                }],
                rules: vec![
                    // Initialize temperature
                    RuleConfig::Custom {
                        code: r#"
if p.temp < 0.01 {
    // Start with temperature based on height
    p.temp = 0.5 + p.position.y * 0.3;
}
"#
                        .into(),
                    },
                    // Strong cohesion to form blobs
                    RuleConfig::Cohere {
                        radius: 0.2,
                        strength: 2.5,
                    },
                    // Separation to prevent total collapse
                    RuleConfig::Separate {
                        radius: 0.06,
                        strength: 4.0,
                    },
                    // Temperature dynamics and buoyancy
                    RuleConfig::Custom {
                        code: r#"
// Heating at bottom, cooling at top
let height_normalized = (p.position.y + 1.0) / 2.0;  // 0 at bottom, 1 at top

// Heat source at bottom
if height_normalized < 0.15 {
    p.temp += 0.8 * uniforms.delta_time;
}
// Cooling at top
if height_normalized > 0.85 {
    p.temp -= 0.6 * uniforms.delta_time;
}

// Ambient cooling/heating toward middle temperature
p.temp = mix(p.temp, 0.5, 0.1 * uniforms.delta_time);

// Clamp temperature
p.temp = clamp(p.temp, 0.0, 1.0);

// Buoyancy force based on temperature
// Hot (temp > 0.5) rises, cold (temp < 0.5) sinks
let buoyancy = (p.temp - 0.5) * 3.0;
p.velocity.y += buoyancy * uniforms.delta_time;
"#
                        .into(),
                    },
                    // Viscous drag - lava is thick!
                    RuleConfig::Drag(3.0),
                    // Horizontal containment (cylinder shape)
                    RuleConfig::Custom {
                        code: r#"
// Keep in cylindrical bounds
let horizontal_dist = length(p.position.xz);
let max_radius = 0.5;

if horizontal_dist > max_radius {
    let push_back = normalize(p.position.xz) * (horizontal_dist - max_radius) * 5.0;
    p.velocity.x -= push_back.x * uniforms.delta_time;
    p.velocity.z -= push_back.y * uniforms.delta_time;
}

// Vertical bounds with bounce
if p.position.y < -0.9 {
    p.position.y = -0.9;
    p.velocity.y = abs(p.velocity.y) * 0.3;
    p.temp += 0.1;  // Extra heat from hitting bottom
}
if p.position.y > 0.9 {
    p.position.y = 0.9;
    p.velocity.y = -abs(p.velocity.y) * 0.3;
    p.temp -= 0.1;  // Extra cooling at top
}
"#
                        .into(),
                    },
                    // Color by temperature
                    RuleConfig::Custom {
                        code: r#"
// Temperature gradient: dark red (cold) -> orange -> yellow (hot)
let temp_clamped = clamp(p.temp, 0.0, 1.0);

var col: vec3<f32>;
if temp_clamped < 0.4 {
    // Cold: dark red/maroon
    let blend = temp_clamped / 0.4;
    col = mix(vec3<f32>(0.4, 0.05, 0.08), vec3<f32>(0.8, 0.15, 0.05), blend);
} else if temp_clamped < 0.7 {
    // Warm: red to orange
    let blend = (temp_clamped - 0.4) / 0.3;
    col = mix(vec3<f32>(0.8, 0.15, 0.05), vec3<f32>(1.0, 0.5, 0.1), blend);
} else {
    // Hot: orange to yellow
    let blend = (temp_clamped - 0.7) / 0.3;
    col = mix(vec3<f32>(1.0, 0.5, 0.1), vec3<f32>(1.0, 0.85, 0.3), blend);
}

p.color = col;

// Slightly larger when hot (expansion)
p.scale = 0.9 + temp_clamped * 0.4;
"#
                        .into(),
                    },
                ],
                vertex_effects: Vec::new(),
                visuals: VisualsConfig {
                    blend_mode: BlendModeConfig::Additive,
                    background_color: [0.02, 0.01, 0.05],
                    shape: ParticleShapeConfig::Circle,
                    trail_length: 0,
                    // Connections help blobs look more cohesive
                    connections_enabled: true,
                    connections_radius: 0.08,
                    connections_color: [0.6, 0.2, 0.1],
                    ..Default::default()
                },
                custom_uniforms: HashMap::new(),
                custom_shaders: CustomShaderConfig::default(),
                fields: Vec::new(),
                volume_render: VolumeRenderConfig::default(),
                mouse: MouseConfig::default(),
                adjacency_enabled: false,
                adjacency_max_neighbors: 32,
                adjacency_radius: 0.2,
                interactions: InteractionConfig::default(),
                post_process: PostProcessConfig {
                    enabled: true,
                    effects: vec![PostProcessEffect::Bloom {
                        intensity: 0.8,
                        threshold: 0.3,
                        radius: 0.01,
                    }],
                },
                emitters: Vec::new(),
            }
        },
    },
    Preset {
        name: "Game of Life",
        description: "Conway's Game of Life cellular automaton using 3D field",
        config: || {
            SimConfig {
                name: "Game of Life".into(),
                // Grid of cells - 40x40 = 1600 particles
                particle_count: 1600,
                bounds: 1.2,
                particle_size: 0.03,
                speed: 1.0,
                spatial_cell_size: 0.1,
                spatial_resolution: 32,
                spawn: SpawnConfig {
                    shape: SpawnShape::Plane {
                        width: 1.6,
                        depth: 1.6,
                    },
                    velocity: InitialVelocity::Zero,
                    color_mode: ColorMode::Uniform {
                        r: 0.1,
                        g: 0.8,
                        b: 0.3,
                    },
                    ..Default::default()
                },
                particle_fields: Vec::new(),
                rules: vec![
                    // All logic in one rule to avoid sync issues
                    RuleConfig::Custom {
                        code: r#"
// Grid setup
let grid_size = 40u;
let cell_size = 1.6 / f32(grid_size);

let gx = index % grid_size;
let gy = index / grid_size;

// Snap to grid position
p.position.x = (f32(gx) - f32(grid_size) / 2.0 + 0.5) * cell_size;
p.position.z = (f32(gy) - f32(grid_size) / 2.0 + 0.5) * cell_size;
p.position.y = 0.0;
p.velocity = vec3<f32>(0.0);

// Initialize on first frame
if uniforms.time < 0.05 {
    let hash = (index * 1103515245u + 12345u) ^ (index << 13u);
    let rand_val = f32(hash & 0xFFFFu) / 65535.0;
    if rand_val < 0.3 {
        p.age = 1.0;  // alive
    } else {
        p.age = 0.0;  // dead
    }
}

// Write current state to field
field_write(0u, p.position, p.age);

// Color based on state
if p.age > 0.5 {
    p.color = vec3<f32>(0.2, 0.95, 0.4);
    p.scale = 1.0;
} else {
    p.color = vec3<f32>(0.08, 0.12, 0.08);
    p.scale = 0.75;
}
"#
                        .into(),
                    },
                    // Read neighbors and compute next state
                    RuleConfig::Custom {
                        code: r#"
// Read neighbor states from field (no timing - runs every frame)
let step = 1.6 / 40.0;
let px = p.position.x;
let pz = p.position.z;

// Sample 8 neighbors
let n1 = field_read(0u, vec3<f32>(px - step, 0.0, pz - step));
let n2 = field_read(0u, vec3<f32>(px, 0.0, pz - step));
let n3 = field_read(0u, vec3<f32>(px + step, 0.0, pz - step));
let n4 = field_read(0u, vec3<f32>(px - step, 0.0, pz));
let n5 = field_read(0u, vec3<f32>(px + step, 0.0, pz));
let n6 = field_read(0u, vec3<f32>(px - step, 0.0, pz + step));
let n7 = field_read(0u, vec3<f32>(px, 0.0, pz + step));
let n8 = field_read(0u, vec3<f32>(px + step, 0.0, pz + step));

// Count alive neighbors
var neighbors = 0.0;
if n1 > 0.5 { neighbors += 1.0; }
if n2 > 0.5 { neighbors += 1.0; }
if n3 > 0.5 { neighbors += 1.0; }
if n4 > 0.5 { neighbors += 1.0; }
if n5 > 0.5 { neighbors += 1.0; }
if n6 > 0.5 { neighbors += 1.0; }
if n7 > 0.5 { neighbors += 1.0; }
if n8 > 0.5 { neighbors += 1.0; }

// Conway's B3/S23
if p.age > 0.5 {
    if neighbors < 1.5 || neighbors > 3.5 {
        p.age = 0.0;
    }
} else {
    if neighbors > 2.5 && neighbors < 3.5 {
        p.age = 1.0;
    }
}
"#
                        .into(),
                    },
                ],
                vertex_effects: Vec::new(),
                visuals: VisualsConfig {
                    blend_mode: BlendModeConfig::Alpha,
                    background_color: [0.02, 0.03, 0.02],
                    shape: ParticleShapeConfig::Square,
                    trail_length: 0,
                    ..Default::default()
                },
                custom_uniforms: HashMap::new(),
                custom_shaders: CustomShaderConfig::default(),
                fields: vec![FieldConfigEntry {
                    name: "life_state".into(),
                    field_type: FieldTypeConfig::Scalar,
                    resolution: 40,
                    decay: 0.0,
                    blur: 0.0,
                    extent: 1.0,
                    blur_iterations: 0,
                    custom_update: None,
                }],
                volume_render: VolumeRenderConfig::default(),
                mouse: MouseConfig::default(),
                adjacency_enabled: false,
                adjacency_max_neighbors: 8,
                adjacency_radius: 0.1,
                interactions: InteractionConfig::default(),
                post_process: PostProcessConfig::default(),
                emitters: Vec::new(),
            }
        },
    },
    // Custom Field Shader Demo
    Preset {
        name: "Morphogenic Field",
        description: "Reaction-diffusion patterns with custom field shader",
        config: || SimConfig {
            name: "Morphogenic Field".into(),
            particle_count: 3000,
            bounds: 1.0,
            particle_size: 0.006,
            speed: 0.5,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Sphere { radius: 0.4 },
                velocity: InitialVelocity::RandomDirection { speed: 0.15 },
                color_mode: ColorMode::ByVelocity,
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Drag(0.2),
                RuleConfig::SpeedLimit {
                    min: 0.05,
                    max: 0.4,
                },
                RuleConfig::WrapWalls,
                // Chemotaxis: particles follow field gradient and deposit
                RuleConfig::Custom {
                    code: r#"
// Sample field at offset positions to compute gradient
let eps = 0.05;
let fx_pos = field_read(0u, p.position + vec3f(eps, 0.0, 0.0));
let fx_neg = field_read(0u, p.position - vec3f(eps, 0.0, 0.0));
let fy_pos = field_read(0u, p.position + vec3f(0.0, eps, 0.0));
let fy_neg = field_read(0u, p.position - vec3f(0.0, eps, 0.0));
let fz_pos = field_read(0u, p.position + vec3f(0.0, 0.0, eps));
let fz_neg = field_read(0u, p.position - vec3f(0.0, 0.0, eps));

// Gradient points toward increasing field values
let gradient = vec3f(fx_pos - fx_neg, fy_pos - fy_neg, fz_pos - fz_neg) / (2.0 * eps);

// Follow gradient with some randomness
let grad_strength = 0.6;
p.velocity += gradient * grad_strength * uniforms.delta_time;

// Deposit to field
field_write(0u, p.position, 0.3);
"#
                    .into(),
                },
                // Random wandering for organic movement
                RuleConfig::Wander {
                    strength: 0.3,
                    frequency: 2.0,
                },
                // Gentle separation
                RuleConfig::Separate {
                    radius: 0.025,
                    strength: 0.8,
                },
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.01, 0.01, 0.02],
                palette: PaletteConfig::Viridis,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![FieldConfigEntry {
                name: "morphogen".into(),
                resolution: 48,
                extent: 1.2,
                decay: 0.99,
                blur: 0.15,
                blur_iterations: 1,
                field_type: FieldTypeConfig::Scalar,
                custom_update: Some(
                    r#"// Reaction-diffusion inspired field update
// Creates organic, evolving patterns

// Read neighbors for Laplacian (diffusion term)
let n_px = read_neighbor(1, 0, 0);
let n_nx = read_neighbor(-1, 0, 0);
let n_py = read_neighbor(0, 1, 0);
let n_ny = read_neighbor(0, -1, 0);
let n_pz = read_neighbor(0, 0, 1);
let n_nz = read_neighbor(0, 0, -1);

// 3D Laplacian for diffusion
let laplacian = (n_px + n_nx + n_py + n_ny + n_pz + n_nz) / 6.0 - value;

// Reaction term: bistable dynamics (creates spots)
let reaction = value * (1.0 - value) * (value - 0.3);

// Time-varying spatial modulation for organic feel
let phase = uniforms.time * 0.3 + world_pos.x * 2.0 + world_pos.z * 1.5;
let modulation = 0.02 + 0.015 * sin(phase);

// Combine: diffusion + reaction + feed
let diffusion_rate = params.blur * 2.5;
new_value = value + uniforms.delta_time * (
    diffusion_rate * laplacian +
    reaction * 5.0 +
    modulation * (1.0 - value)
);

// Apply decay and clamp
new_value = clamp(new_value * params.decay, 0.0, 1.0);"#
                        .into(),
                ),
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 64,
                density_scale: 3.5,
                palette: PaletteConfig::Rainbow,
                threshold: 0.03,
                additive: true,
            },
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig {
                enabled: true,
                effects: vec![PostProcessEffect::Bloom {
                    threshold: 0.2,
                    intensity: 0.4,
                    radius: 0.003,
                }],
            },
            emitters: Vec::new(),
        },
    },
    // Dramatic visual showcase
    Preset {
        name: "Tritium Vortex",
        description: "Swirling tritium plasma storm with chaotic vortices and electric energy",
        config: || SimConfig {
            name: "Tritium Vortex".into(),
            particle_count: 10000,
            bounds: 1.0,
            particle_size: 0.01,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Shell {
                    inner: 0.2,
                    outer: 0.9,
                },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::ByVelocity,
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Drag(0.05),
                RuleConfig::SpeedLimit { min: 0.0, max: 5.0 },
                RuleConfig::BounceWalls { restitution: 0.8 },
                // Follow the vector field flow
                RuleConfig::Custom {
                    code: r#"
// Read vector field at particle position
let flow = field_read(0u, p.position);
let flow_strength = length(flow);

// Strong acceleration along flow direction
p.velocity += flow * 4.0 * uniforms.delta_time;

// Add turbulent kick based on local field intensity
let turb = sin(p.position.x * 10.0 + uniforms.time * 2.0) *
           cos(p.position.z * 10.0 - uniforms.time * 1.5);
p.velocity.y += turb * flow_strength * 0.8 * uniforms.delta_time;

// Color by speed (cool blue to hot orange)
let speed = length(p.velocity);
let heat = clamp(speed / 1.5, 0.0, 1.0);
p.color = mix(vec3f(0.1, 0.3, 1.0), vec3f(1.0, 0.3, 0.05), heat);
"#
                    .into(),
                },
                // Central vortex pull
                RuleConfig::Vortex {
                    center: [0.0, 0.0, 0.0],
                    axis: [0.0, 1.0, 0.0],
                    strength: 0.8,
                },
                // Secondary off-center vortex
                RuleConfig::Vortex {
                    center: [0.4, 0.2, 0.0],
                    axis: [0.3, 0.9, 0.1],
                    strength: 0.5,
                },
                // Pulsing attraction to keep things interesting
                RuleConfig::Pulse {
                    point: [0.0, 0.0, 0.0],
                    strength: 1.5,
                    frequency: 0.8,
                    radius: 1.5,
                },
            ],
            vertex_effects: vec![
                // Stretch particles along velocity
                VertexEffectConfig::Squash {
                    axis: [1.0, 0.0, 0.0],
                    amount: 0.4,
                },
                // Add flutter for electric feel
                VertexEffectConfig::Flutter {
                    intensity: 0.15,
                    speed: 25.0,
                },
            ],
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Additive,
                background_color: [0.0, 0.0, 0.02],
                palette: PaletteConfig::Inferno,
                trail_length: 4,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![FieldConfigEntry {
                name: "plasma".into(),
                resolution: 40,
                extent: 1.3,
                decay: 0.92,
                blur: 0.2,
                blur_iterations: 1,
                field_type: FieldTypeConfig::Vector,
                custom_update: Some(
                    r#"// Plasma vortex field dynamics
// Creates swirling, chaotic flow patterns

// Sample neighbors for curl-like computation
let n_px = read_neighbor(1, 0, 0);
let n_nx = read_neighbor(-1, 0, 0);
let n_py = read_neighbor(0, 1, 0);
let n_ny = read_neighbor(0, -1, 0);
let n_pz = read_neighbor(0, 0, 1);
let n_nz = read_neighbor(0, 0, -1);

// Compute curl (rotation) of the field
let curl = vec3<f32>(
    (n_py.z - n_ny.z) - (n_pz.y - n_nz.y),
    (n_pz.x - n_nz.x) - (n_px.z - n_nx.z),
    (n_px.y - n_nx.y) - (n_py.x - n_ny.x)
) * 0.5;

// Average neighbors for diffusion
let avg = (n_px + n_nx + n_py + n_ny + n_pz + n_nz) / 6.0;
let laplacian = avg - value;

// Time-varying vortex injection points
let t = uniforms.time;
let vortex1 = vec3<f32>(sin(t * 0.3) * 0.3, cos(t * 0.2) * 0.2, sin(t * 0.4) * 0.3);
let vortex2 = vec3<f32>(cos(t * 0.25) * 0.4, sin(t * 0.35) * 0.3, cos(t * 0.3) * 0.2);

// Distance to vortex centers
let d1 = length(world_pos - vortex1);
let d2 = length(world_pos - vortex2);

// Inject rotational energy near vortex points
let inject1 = exp(-d1 * d1 * 8.0) * vec3<f32>(
    -world_pos.z + vortex1.z,
    0.3,
    world_pos.x - vortex1.x
) * 0.5;
let inject2 = exp(-d2 * d2 * 10.0) * vec3<f32>(
    world_pos.z - vortex2.z,
    -0.2,
    -world_pos.x + vortex2.x
) * 0.4;

// Combine: diffusion + curl feedback + vortex injection
let diffusion_rate = params.blur * 1.5;
new_value = value + uniforms.delta_time * (
    diffusion_rate * laplacian +
    curl * 0.3 +
    inject1 + inject2
);

// Apply decay
new_value = new_value * params.decay;

// Clamp magnitude to prevent explosion
let mag = length(new_value);
if mag > 2.0 {
    new_value = new_value * (2.0 / mag);
}"#
                    .into(),
                ),
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 80,
                density_scale: 2.5,
                palette: PaletteConfig::Inferno,
                threshold: 0.02,
                additive: true,
            },
            particle_fields: Vec::new(),
            mouse: MouseConfig {
                power: crate::config::MousePower::Attract,
                radius: 0.3,
                strength: 2.0,
                color: [1.0, 0.5, 0.2],
            },
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig {
                enabled: true,
                effects: vec![
                    PostProcessEffect::Bloom {
                        threshold: 0.15,
                        intensity: 0.7,
                        radius: 0.004,
                    },
                    PostProcessEffect::ChromaticAberration {
                        intensity: 0.003,
                        radial: true,
                    },
                    PostProcessEffect::Vignette {
                        intensity: 0.4,
                        softness: 0.6,
                    },
                ],
            },
            emitters: Vec::new(),
        },
    },
    // Ocean simulation
    Preset {
        name: "Ocean Swells",
        description: "Rolling ocean waves with particles drifting in the currents",
        config: || SimConfig {
            name: "Ocean Swells".into(),
            particle_count: 10000,
            bounds: 1.0,
            particle_size: 0.01,
            speed: 1.0,
            spatial_cell_size: 0.12,
            spatial_resolution: 32,
            spawn: SpawnConfig {
                shape: SpawnShape::Cube { size: 2.4 },
                velocity: InitialVelocity::Zero,
                color_mode: ColorMode::Uniform {
                    r: 0.3,
                    g: 0.6,
                    b: 0.9,
                },
                ..Default::default()
            },
            rules: vec![
                RuleConfig::Drag(0.15),
                RuleConfig::SpeedLimit { min: 0.0, max: 0.6 },
                // Follow ocean currents
                RuleConfig::Custom {
                    code: r#"
// Sample wave field (scalar = wave height)
let wave_height = field_read(0u, p.position);

// Sample nearby to get gradient (wave slope)
let eps = 0.08;
let hx = field_read(0u, p.position + vec3f(eps, 0.0, 0.0)) -
         field_read(0u, p.position - vec3f(eps, 0.0, 0.0));
let hz = field_read(0u, p.position + vec3f(0.0, 0.0, eps)) -
         field_read(0u, p.position - vec3f(0.0, 0.0, eps));

// Particles move with the wave - circular orbital motion
// Horizontal push from wave slope, vertical from wave height change
let wave_push = vec3f(-hx * 0.5, wave_height * 0.3, -hz * 0.5);
p.velocity += wave_push * uniforms.delta_time;

// Buoyancy - particles float near y=0 surface
let surface_y = wave_height * 0.3;
let depth = p.position.y - surface_y;
p.velocity.y += (-depth * 2.0 - p.velocity.y * 0.5) * uniforms.delta_time;

// Color by depth - lighter near surface, darker below
let depth_factor = clamp(-depth * 2.0 + 0.5, 0.0, 1.0);
let deep_color = vec3f(0.05, 0.15, 0.3);
let surface_color = vec3f(0.4, 0.75, 0.95);
let foam_color = vec3f(0.8, 0.9, 1.0);

// Add foam on wave peaks
let foam = smoothstep(0.15, 0.3, wave_height);
var water_color = mix(deep_color, surface_color, depth_factor);
water_color = mix(water_color, foam_color, foam * depth_factor);
p.color = water_color;
"#
                    .into(),
                },
                // Gentle horizontal drift
                RuleConfig::Wind {
                    direction: [0.3, 0.0, 0.1],
                    strength: 0.05,
                    turbulence: 0.02,
                },
                // Wrap at boundaries for endless ocean feel
                RuleConfig::WrapWalls,
            ],
            vertex_effects: vec![
                // Gentle bob
                VertexEffectConfig::Sway {
                    frequency: 2.0,
                    amplitude: 0.03,
                    axis: [0.0, 1.0, 0.0],
                },
            ],
            visuals: VisualsConfig {
                blend_mode: BlendModeConfig::Alpha,
                background_color: [0.02, 0.05, 0.12],
                palette: PaletteConfig::Ocean,
                shape: ParticleShapeConfig::Circle,
                trail_length: 0,
                ..Default::default()
            },
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: vec![FieldConfigEntry {
                name: "waves".into(),
                resolution: 48,
                extent: 1.6,
                decay: 0.995,
                blur: 0.1,
                blur_iterations: 1,
                field_type: FieldTypeConfig::Scalar,
                custom_update: Some(
                    r#"// Ocean wave propagation
// Simulates rolling swells using wave equation dynamics

// Read neighbors
let n_px = read_neighbor(1, 0, 0);
let n_nx = read_neighbor(-1, 0, 0);
let n_py = read_neighbor(0, 1, 0);
let n_ny = read_neighbor(0, -1, 0);
let n_pz = read_neighbor(0, 0, 1);
let n_nz = read_neighbor(0, 0, -1);

// Laplacian for wave spreading
let laplacian = (n_px + n_nx + n_pz + n_nz) / 4.0 - value;

// Wave equation: acceleration proportional to curvature
// Using simplified integration (value acts as both height and has implicit velocity)
let wave_speed = 0.4;
let damping = 0.02;

// Multiple wave sources moving across the ocean
let t = uniforms.time;

// Primary swell - long rolling waves from one direction
let swell1_pos = vec3<f32>(sin(t * 0.1) * 2.0, 0.0, -1.5 + t * 0.05);
let d1 = length(world_pos.xz - swell1_pos.xz);
let swell1 = sin(d1 * 3.0 - t * 1.5) * exp(-d1 * 0.3) * 0.15;

// Secondary swell - crossing waves
let swell2_pos = vec3<f32>(-1.5, 0.0, sin(t * 0.08) * 2.0);
let d2 = length(world_pos.xz - swell2_pos.xz);
let swell2 = sin(d2 * 4.0 - t * 1.8) * exp(-d2 * 0.4) * 0.1;

// Gentle wind chop - small ripples
let chop = sin(world_pos.x * 12.0 + t * 3.0) *
           cos(world_pos.z * 10.0 + t * 2.5) * 0.02;

// Combine wave sources
let wave_input = swell1 + swell2 + chop;

// Update with wave equation + damping + input
new_value = value + uniforms.delta_time * (
    wave_speed * laplacian +
    (wave_input - value) * 0.5 -
    value * damping
);

// Soft clamp
new_value = clamp(new_value, -0.5, 0.5);"#
                        .into(),
                ),
            }],
            volume_render: VolumeRenderConfig {
                enabled: true,
                field_index: 0,
                steps: 32,
                density_scale: 2.0,
                palette: PaletteConfig::Plasma,
                threshold: 0.02,
                additive: false,
            },
            particle_fields: Vec::new(),
            mouse: MouseConfig {
                power: crate::config::MousePower::Explode,
                radius: 1.0,
                strength: 10.0,
                color: [0.5, 0.8, 1.0],
            },
            adjacency_enabled: false,
            adjacency_max_neighbors: 32,
            adjacency_radius: 0.1,
            interactions: InteractionConfig::default(),
            post_process: PostProcessConfig {
                enabled: true,
                effects: vec![PostProcessEffect::Bloom {
                    threshold: 0.4,
                    intensity: 0.25,
                    radius: 0.003,
                }],
            },
            emitters: Vec::new(),
        },
    },
];
