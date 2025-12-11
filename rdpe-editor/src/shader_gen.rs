//! Shader generation for the embedded simulation.
//!
//! This module generates WGSL compute and render shaders from SimConfig,
//! using the actual rdpe rule system for proper behavior.

use crate::config::{SimConfig, ParticleShapeConfig, PaletteConfig, ColorMappingConfig, MousePower};
use rdpe::Rule;

/// Generate field declarations and helper functions from config.
fn generate_field_code(config: &SimConfig) -> String {
    if config.fields.is_empty() {
        return String::new();
    }

    let registry = config.to_field_registry();
    // Field bindings start at group(2) binding(0)
    registry.to_wgsl_declarations(0)
}

/// Generate mouse power WGSL code based on the selected power.
fn generate_mouse_power_code(power: &MousePower) -> String {
    power.to_wgsl()
}

/// Generate early mouse power WGSL code (runs before dead particle skip).
fn generate_early_mouse_power_code(power: &MousePower) -> String {
    power.to_early_wgsl()
}

/// Generate custom uniform fields for the Uniforms struct.
fn generate_custom_uniform_fields(config: &SimConfig) -> String {
    if config.custom_uniforms.is_empty() {
        return String::new();
    }

    let mut fields = String::new();
    // Sort for deterministic order
    let mut uniforms: Vec<_> = config.custom_uniforms.iter().collect();
    uniforms.sort_by_key(|(name, _)| *name);

    for (name, value) in uniforms {
        fields.push_str(&format!("    {}: {},\n", name, value.wgsl_type()));
    }
    fields
}

/// Generate compute shader from simulation config.
///
/// This generates a WGSL compute shader that:
/// 1. Defines the Particle struct dynamically from config
/// 2. Defines uniforms (view_proj, time, delta_time, custom uniforms)
/// 3. Applies all rules in order
/// 4. Integrates velocity and updates position
pub fn generate_compute_shader(config: &SimConfig) -> String {
    let particle_struct = config.particle_wgsl_struct();

    // Convert rules to rdpe::Rule and then to WGSL
    let rules: Vec<Rule> = config.rules.iter().map(|r| r.to_rule()).collect();

    // Check if any rules need neighbor access
    let needs_neighbors = rules.iter().any(|r| r.requires_neighbors());

    if needs_neighbors {
        generate_compute_shader_with_neighbors(config, &rules, &particle_struct)
    } else {
        generate_compute_shader_simple(config, &rules, &particle_struct)
    }
}

/// Generate simple compute shader (no spatial hashing).
fn generate_compute_shader_simple(config: &SimConfig, rules: &[Rule], particle_struct: &str) -> String {
    // Generate rule code
    let rules_code: String = rules
        .iter()
        .filter(|r| !r.requires_neighbors())
        .map(|r| r.to_wgsl(config.bounds))
        .collect::<Vec<_>>()
        .join("\n\n");

    // Generate custom uniform fields
    let custom_uniform_fields = generate_custom_uniform_fields(config);

    // Generate field code (if any fields are defined)
    let field_code = generate_field_code(config);
    let has_fields = !config.fields.is_empty();

    // Generate mouse power code
    let mouse_power_code = generate_mouse_power_code(&config.mouse.power);
    let early_mouse_power_code = generate_early_mouse_power_code(&config.mouse.power);

    format!(r#"
// ============================================
// RDPE Compute Shader (Generated)
// ============================================

// Particle struct
{particle_struct}

// Mouse interaction data
struct Mouse {{
    pos: vec4<f32>,                    // xyz = world position
    down_radius_strength: vec4<f32>,   // x = down (0/1), y = radius, z = strength
    color: vec4<f32>,                  // rgb = color for paint/spawn
}}

// Uniforms
struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
    _pad: vec2<f32>,
    mouse: Mouse,
{custom_uniform_fields}}}

// Bindings
@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

{field_code}
// Utility functions
{shader_utils}

// Main compute shader
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
    let idx = id.x;
    let index = idx;  // Alias for rules that use 'index'
    if (idx >= arrayLength(&particles)) {{
        return;
    }}

    var p = particles[idx];

    let time = uniforms.time;
    let delta_time = uniforms.delta_time;
    let bounds = {bounds:.6};

    // Mouse interaction helpers (needed for early mouse powers)
    let mouse_pos = uniforms.mouse.pos.xyz;
    let mouse_down = uniforms.mouse.down_radius_strength.x;
    let mouse_radius = uniforms.mouse.down_radius_strength.y;
    let mouse_strength = uniforms.mouse.down_radius_strength.z;
    let mouse_color = uniforms.mouse.color.xyz;

    // ============================================
    // Early mouse powers (run on dead particles too)
    // ============================================
{early_mouse_power_code}

    // Skip dead particles for remaining logic
    if (p.alive == 0u) {{
        particles[idx] = p;  // Write back in case early power revived it
        return;
    }}

    {field_count_decl}

    // ============================================
    // Apply rules
    // ============================================
{rules_code}

    // ============================================
    // Apply mouse power
    // ============================================
{mouse_power_code}

    // ============================================
    // Integrate velocity
    // ============================================
    p.position += p.velocity * delta_time;

    // Update age
    p.age += delta_time;

    // Write back
    particles[idx] = p;
}}
"#,
        particle_struct = particle_struct,
        custom_uniform_fields = custom_uniform_fields,
        field_code = if has_fields { &field_code } else { "// No fields\n" },
        shader_utils = SHADER_UTILS,
        bounds = config.bounds,
        field_count_decl = if has_fields { format!("let field_count = {}u;", config.fields.len()) } else { String::new() },
        rules_code = indent_code(&rules_code, "    "),
        early_mouse_power_code = indent_code(&early_mouse_power_code, "    "),
        mouse_power_code = indent_code(&mouse_power_code, "    "),
    )
}

/// Generate compute shader with spatial hashing for neighbor queries.
fn generate_compute_shader_with_neighbors(config: &SimConfig, rules: &[Rule], particle_struct: &str) -> String {
    // Separate rules into neighbor and non-neighbor
    let neighbor_rules: Vec<&Rule> = rules.iter().filter(|r| r.requires_neighbors()).collect();
    let simple_rules: Vec<&Rule> = rules.iter().filter(|r| !r.requires_neighbors()).collect();

    // Generate rule code
    let simple_rules_code: String = simple_rules
        .iter()
        .map(|r| r.to_wgsl(config.bounds))
        .collect::<Vec<_>>()
        .join("\n\n");

    // Generate neighbor rules code, using custom WGSL when available
    let neighbor_rules_code: String = config.rules.iter()
        .filter(|r| r.requires_neighbors())
        .map(|rule_config| {
            // Check if this rule has custom WGSL for the editor
            if let Some(custom_wgsl) = rule_config.to_neighbor_wgsl() {
                custom_wgsl
            } else {
                // Fall back to core library's WGSL
                rule_config.to_rule().to_neighbor_wgsl()
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Generate accumulator variables for neighbor rules
    let accumulator_vars = generate_accumulator_vars(&neighbor_rules);

    // Generate post-neighbor code for rules that need final processing
    let post_neighbor_code = generate_post_neighbor_code(config, &neighbor_rules);

    // Generate custom uniform fields
    let custom_uniform_fields = generate_custom_uniform_fields(config);

    // Generate field code (if any fields are defined)
    let field_code = generate_field_code(config);
    let has_fields = !config.fields.is_empty();

    // Generate mouse power code
    let mouse_power_code = generate_mouse_power_code(&config.mouse.power);
    let early_mouse_power_code = generate_early_mouse_power_code(&config.mouse.power);

    format!(r#"
// ============================================
// RDPE Compute Shader (Generated with Spatial Hashing)
// ============================================

// Particle struct
{particle_struct}

// Mouse interaction data
struct Mouse {{
    pos: vec4<f32>,                    // xyz = world position
    down_radius_strength: vec4<f32>,   // x = down (0/1), y = radius, z = strength
    color: vec4<f32>,                  // rgb = color for paint/spawn
}}

// Uniforms
struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
    _pad: vec2<f32>,
    mouse: Mouse,
{custom_uniform_fields}}}

// Spatial params for neighbor queries
struct SpatialParams {{
    cell_size: f32,
    grid_resolution: u32,
    num_particles: u32,
    max_neighbors: u32,
}}

// Bindings
@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var<storage, read> sorted_indices: array<u32>;
@group(0) @binding(3) var<storage, read> cell_start: array<u32>;
@group(0) @binding(4) var<storage, read> cell_end: array<u32>;
@group(0) @binding(5) var<uniform> spatial: SpatialParams;

{field_code}
// ============================================
// Morton encoding utilities
// ============================================
{morton_utils}

// ============================================
// Neighbor iteration utilities
// ============================================
{neighbor_utils}

// ============================================
// Utility functions
// ============================================
{shader_utils}

// Main compute shader
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
    let idx = id.x;
    let index = idx;  // Alias for rules that use 'index'
    if (idx >= arrayLength(&particles)) {{
        return;
    }}

    var p = particles[idx];

    let time = uniforms.time;
    let delta_time = uniforms.delta_time;
    let bounds = {bounds:.6};

    // Mouse interaction helpers (needed for early mouse powers)
    let mouse_pos = uniforms.mouse.pos.xyz;
    let mouse_down = uniforms.mouse.down_radius_strength.x;
    let mouse_radius = uniforms.mouse.down_radius_strength.y;
    let mouse_strength = uniforms.mouse.down_radius_strength.z;
    let mouse_color = uniforms.mouse.color.xyz;

    // ============================================
    // Early mouse powers (run on dead particles too)
    // ============================================
{early_mouse_power_code}

    // Skip dead particles for remaining logic
    if (p.alive == 0u) {{
        particles[idx] = p;  // Write back in case early power revived it
        return;
    }}

    {field_count_decl}

    let my_pos = p.position;
    let my_cell = pos_to_cell(my_pos, spatial.cell_size, spatial.grid_resolution);

    // ============================================
    // Accumulator variables for neighbor rules
    // ============================================
{accumulator_vars}

    // ============================================
    // Neighbor iteration
    // ============================================
    var neighbor_count = 0u;
    let max_neighbors = spatial.max_neighbors;

    for (var offset_idx = 0u; offset_idx < 27u; offset_idx++) {{
        // Early exit if max neighbors reached (0 = unlimited)
        if max_neighbors > 0u && neighbor_count >= max_neighbors {{
            break;
        }}

        let neighbor_morton = neighbor_cell_morton(my_cell, offset_idx, spatial.grid_resolution);

        if neighbor_morton == 0xFFFFFFFFu {{
            continue; // Out of bounds
        }}

        let start = cell_start[neighbor_morton];
        let end = cell_end[neighbor_morton];

        if start == 0xFFFFFFFFu {{
            continue; // Empty cell
        }}

        for (var j = start; j < end; j++) {{
            // Early exit if max neighbors reached
            if max_neighbors > 0u && neighbor_count >= max_neighbors {{
                break;
            }}

            let other_idx = sorted_indices[j];

            if other_idx == index {{
                continue; // Skip self
            }}

            let other = particles[other_idx];

            // Skip dead neighbors
            if other.alive == 0u {{
                continue;
            }}

            let neighbor_pos = other.position;
            let neighbor_vel = other.velocity;
            let diff = my_pos - neighbor_pos;
            let neighbor_dist = length(diff);
            let neighbor_dir = select(vec3<f32>(0.0), diff / neighbor_dist, neighbor_dist > 0.0001);

            neighbor_count += 1u;

            // ============================================
            // Apply neighbor rules
            // ============================================
{neighbor_rules_code}
        }}
    }}

    // ============================================
    // Post-neighbor processing
    // ============================================
{post_neighbor_code}

    // ============================================
    // Apply non-neighbor rules
    // ============================================
{simple_rules_code}

    // ============================================
    // Apply mouse power
    // ============================================
{mouse_power_code}

    // ============================================
    // Integrate velocity
    // ============================================
    p.position += p.velocity * delta_time;

    // Update age
    p.age += delta_time;

    // Write back
    particles[idx] = p;
}}
"#,
        particle_struct = particle_struct,
        custom_uniform_fields = custom_uniform_fields,
        field_code = if has_fields { &field_code } else { "// No fields\n" },
        morton_utils = MORTON_WGSL,
        neighbor_utils = NEIGHBOR_UTILS_WGSL,
        shader_utils = SHADER_UTILS,
        bounds = config.bounds,
        field_count_decl = if has_fields { format!("let field_count = {}u;", config.fields.len()) } else { String::new() },
        accumulator_vars = indent_code(&accumulator_vars, "    "),
        neighbor_rules_code = indent_code(&neighbor_rules_code, "            "),
        post_neighbor_code = indent_code(&post_neighbor_code, "    "),
        simple_rules_code = indent_code(&simple_rules_code, "    "),
        early_mouse_power_code = indent_code(&early_mouse_power_code, "    "),
        mouse_power_code = indent_code(&mouse_power_code, "    "),
    )
}

/// Generate accumulator variables needed by neighbor rules.
fn generate_accumulator_vars(rules: &[&Rule]) -> String {
    // Check which accumulators are needed
    let needs_cohesion = rules.iter().any(|r| matches!(r, Rule::Cohere { .. } | Rule::Flock { .. }));
    let needs_alignment = rules.iter().any(|r| matches!(r, Rule::Align { .. } | Rule::Flock { .. }));
    let needs_chase = rules.iter().any(|r| matches!(r, Rule::Chase { .. }));
    let needs_evade = rules.iter().any(|r| matches!(r, Rule::Evade { .. }));
    let needs_viscosity = rules.iter().any(|r| matches!(r, Rule::Viscosity { .. }));
    let needs_pressure = rules.iter().any(|r| matches!(r, Rule::Pressure { .. }));
    let needs_surface_tension = rules.iter().any(|r| matches!(r, Rule::SurfaceTension { .. }));

    let mut vars = String::new();

    if needs_cohesion {
        vars.push_str("    var cohesion_sum = vec3<f32>(0.0);\n    var cohesion_count = 0.0;\n");
    }
    if needs_alignment {
        vars.push_str("    var alignment_sum = vec3<f32>(0.0);\n    var alignment_count = 0.0;\n");
    }
    if needs_chase {
        vars.push_str("    var chase_nearest_dist = 1000.0;\n    var chase_nearest_pos = vec3<f32>(0.0);\n");
    }
    if needs_evade {
        vars.push_str("    var evade_nearest_dist = 1000.0;\n    var evade_nearest_pos = vec3<f32>(0.0);\n");
    }
    if needs_viscosity {
        vars.push_str("    var viscosity_sum = vec3<f32>(0.0);\n    var viscosity_weight = 0.0;\n");
    }
    if needs_pressure {
        vars.push_str("    var pressure_density = 0.0;\n    var pressure_force = vec3<f32>(0.0);\n");
    }
    if needs_surface_tension {
        vars.push_str("    var surface_neighbor_count = 0.0;\n    var surface_center_sum = vec3<f32>(0.0);\n");
    }

    vars
}

/// Generate post-neighbor processing code for rules that need it.
/// Uses custom editor WGSL when available, otherwise falls back to rdpe's Rule::to_post_neighbor_wgsl().
fn generate_post_neighbor_code(config: &SimConfig, _rules: &[&Rule]) -> String {
    config.rules.iter()
        .filter(|r| r.requires_neighbors())
        .map(|rule_config| {
            // Check if this rule has custom post-neighbor WGSL for the editor
            if let Some(custom_wgsl) = rule_config.to_post_neighbor_wgsl() {
                custom_wgsl
            } else {
                // Fall back to core library's WGSL
                rule_config.to_rule().to_post_neighbor_wgsl()
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Generate palette WGSL code (constants + sample function) if palette is enabled.
/// Returns (palette_code, color_expression) where color_expression replaces particle_color.
fn generate_palette_code(
    palette: &PaletteConfig,
    mapping: &ColorMappingConfig,
    particle_count: u32,
) -> (String, String) {
    if *palette == PaletteConfig::None {
        return (String::new(), "particle_color".to_string());
    }

    let colors = palette.colors();
    let palette_consts = format!(
        r#"
// Palette colors
const PALETTE_0: vec3<f32> = vec3<f32>({:.6}, {:.6}, {:.6});
const PALETTE_1: vec3<f32> = vec3<f32>({:.6}, {:.6}, {:.6});
const PALETTE_2: vec3<f32> = vec3<f32>({:.6}, {:.6}, {:.6});
const PALETTE_3: vec3<f32> = vec3<f32>({:.6}, {:.6}, {:.6});
const PALETTE_4: vec3<f32> = vec3<f32>({:.6}, {:.6}, {:.6});

fn sample_palette(t: f32) -> vec3<f32> {{
    let t_clamped = clamp(t, 0.0, 1.0);
    let scaled = t_clamped * 4.0;
    let idx = u32(floor(scaled));
    let frac = fract(scaled);

    var c0: vec3<f32>;
    var c1: vec3<f32>;

    switch idx {{
        case 0u: {{ c0 = PALETTE_0; c1 = PALETTE_1; }}
        case 1u: {{ c0 = PALETTE_1; c1 = PALETTE_2; }}
        case 2u: {{ c0 = PALETTE_2; c1 = PALETTE_3; }}
        case 3u: {{ c0 = PALETTE_3; c1 = PALETTE_4; }}
        default: {{ c0 = PALETTE_4; c1 = PALETTE_4; }}
    }}

    return mix(c0, c1, frac);
}}
"#,
        colors[0].x, colors[0].y, colors[0].z,
        colors[1].x, colors[1].y, colors[1].z,
        colors[2].x, colors[2].y, colors[2].z,
        colors[3].x, colors[3].y, colors[3].z,
        colors[4].x, colors[4].y, colors[4].z,
    );

    // Generate the mapping expression based on ColorMappingConfig
    let mapping_expr = match mapping {
        ColorMappingConfig::None => {
            // No mapping - use particle index as default
            format!("f32(instance_index) / f32({}u)", particle_count.max(1))
        }
        ColorMappingConfig::Index => {
            format!("f32(instance_index) / f32({}u)", particle_count.max(1))
        }
        ColorMappingConfig::Speed { min, max } => {
            format!("clamp((length(particle_vel) - {:.6}) / ({:.6} - {:.6}), 0.0, 1.0)", min, max, min)
        }
        ColorMappingConfig::Age { max_age } => {
            format!("clamp(particle_age / {:.6}, 0.0, 1.0)", max_age)
        }
        ColorMappingConfig::PositionY { min, max } => {
            format!("clamp((particle_pos.y - {:.6}) / ({:.6} - {:.6}), 0.0, 1.0)", min, max, min)
        }
        ColorMappingConfig::Distance { max_dist } => {
            format!("clamp(length(particle_pos) / {:.6}, 0.0, 1.0)", max_dist)
        }
        ColorMappingConfig::Random => {
            "fract(sin(f32(instance_index) * 12.9898) * 43758.5453)".to_string()
        }
    };

    let color_expr = format!("sample_palette({})", mapping_expr);
    (palette_consts, color_expr)
}

/// Generate render shader from visual config.
pub fn generate_render_shader(config: &SimConfig) -> String {
    let visuals = &config.visuals;

    // Generate shape fragment code
    let shape_code = match visuals.shape {
        ParticleShapeConfig::Circle => SHAPE_CIRCLE,
        ParticleShapeConfig::CircleHard => SHAPE_CIRCLE_HARD,
        ParticleShapeConfig::Square => SHAPE_SQUARE,
        ParticleShapeConfig::Ring => SHAPE_RING,
        ParticleShapeConfig::Star => SHAPE_STAR,
        ParticleShapeConfig::Triangle => SHAPE_TRIANGLE,
        ParticleShapeConfig::Hexagon => SHAPE_HEXAGON,
        ParticleShapeConfig::Diamond => SHAPE_DIAMOND,
        ParticleShapeConfig::Point => SHAPE_POINT,
    };

    // Generate vertex effects code
    let vertex_effects_code: String = config
        .vertex_effects
        .iter()
        .map(|effect| effect.to_effect().to_wgsl())
        .collect::<Vec<_>>()
        .join("\n");

    // Generate custom uniform fields
    let custom_uniform_fields = generate_custom_uniform_fields(config);

    // Generate palette code
    let (palette_code, color_expr) = generate_palette_code(
        &visuals.palette,
        &visuals.color_mapping,
        config.particle_count,
    );

    // Velocity stretch code
    let velocity_stretch_code = if visuals.velocity_stretch {
        format!(r#"
    // ============================================
    // Velocity stretch
    // ============================================
    let vel_speed = length(particle_vel);
    if (vel_speed > 0.001) {{
        // Get velocity direction in clip space
        let world_pos_ahead = particle_pos + normalize(particle_vel) * 0.1;
        let clip_ahead = uniforms.view_proj * vec4<f32>(world_pos_ahead, 1.0);
        let clip_current = uniforms.view_proj * vec4<f32>(particle_pos, 1.0);

        // Screen-space velocity direction
        let screen_ahead = clip_ahead.xy / clip_ahead.w;
        let screen_current = clip_current.xy / clip_current.w;
        var screen_dir = screen_ahead - screen_current;

        let screen_len = length(screen_dir);
        if (screen_len > 0.0001) {{
            screen_dir = screen_dir / screen_len;

            // Stretch factor based on speed
            let stretch = 1.0 + vel_speed * {factor:.6};

            // Rotate and stretch the quad
            // x component: stretch along velocity direction
            // y component: keep perpendicular size
            let perp = vec2<f32>(-screen_dir.y, screen_dir.x);
            rotated_quad = screen_dir * rotated_quad.x * stretch + perp * rotated_quad.y;
        }}
    }}
"#, factor = visuals.velocity_stretch_factor)
    } else {
        String::new()
    };

    // Custom shader code
    let custom_vertex_code = if config.custom_shaders.vertex_code.is_empty() {
        String::new()
    } else {
        format!("\n    // ============================================\n    // Custom vertex code\n    // ============================================\n{}\n",
            indent_code(&config.custom_shaders.vertex_code, "    "))
    };

    let custom_fragment_code = if config.custom_shaders.fragment_code.is_empty() {
        String::new()
    } else {
        format!("\n    // ============================================\n    // Custom fragment code\n    // ============================================\n{}\n",
            indent_code(&config.custom_shaders.fragment_code, "    "))
    };

    format!(r#"
// ============================================
// RDPE Render Shader (Generated)
// ============================================

struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
{custom_uniform_fields}}}

struct VertexOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) alpha: f32,
}}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
{palette_code}
@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) particle_pos: vec3<f32>,
    @location(1) particle_vel: vec3<f32>,
    @location(2) particle_color: vec3<f32>,
    @location(3) particle_age: f32,
    @location(4) alive: u32,
    @location(5) scale: f32,
) -> VertexOutput {{
    var out: VertexOutput;

    // Cull dead particles
    if (alive == 0u) {{
        out.position = vec4<f32>(0.0, 0.0, -10.0, 1.0);
        out.color = vec3<f32>(0.0);
        out.uv = vec2<f32>(0.0);
        out.alpha = 0.0;
        return out;
    }}

    // Base particle size
    let particle_size = {particle_size:.6} * scale;

    // Quad vertices (triangle strip) - base positions
    var quad_positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );

    var uvs = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );

    let quad_pos = quad_positions[vertex_index];
    let uv = uvs[vertex_index];

    // ============================================
    // Vertex effect variables
    // ============================================
    var pos_offset = vec3<f32>(0.0, 0.0, 0.0);
    var rotated_quad = quad_pos;
    var size_mult = 1.0;
    var color_mod = {color_expr};

    // ============================================
    // Apply vertex effects
    // ============================================
{vertex_effects_code}
{custom_vertex_code}
{velocity_stretch_code}
    // ============================================
    // Compute final position
    // ============================================
    let final_size = particle_size * size_mult;
    let offset = rotated_quad * final_size;

    // Billboard: offset in clip space
    var world_pos = vec4<f32>(particle_pos + pos_offset, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;
    clip_pos.x += offset.x * clip_pos.w;
    clip_pos.y += offset.y * clip_pos.w;

    out.position = clip_pos;
    out.color = color_mod;
    out.uv = uv;
    out.alpha = 1.0;
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    if (in.alpha < 0.001) {{
        discard;
    }}

    let uv = in.uv;
    let center = vec2<f32>(0.5, 0.5);
    var frag_color = in.color;

    // Shape rendering
{shape_code}
{custom_fragment_code}
    return vec4<f32>(frag_color * alpha, alpha);
}}
"#,
        particle_size = config.particle_size,
        custom_uniform_fields = custom_uniform_fields,
        palette_code = palette_code,
        color_expr = color_expr,
        vertex_effects_code = indent_code(&vertex_effects_code, "    "),
        custom_vertex_code = custom_vertex_code,
        velocity_stretch_code = velocity_stretch_code,
        shape_code = indent_code(shape_code, "    "),
        custom_fragment_code = custom_fragment_code,
    )
}

/// Indent code by a given prefix.
fn indent_code(code: &str, prefix: &str) -> String {
    code.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================
// Shader utility functions
// ============================================

const SHADER_UTILS: &str = r#"
// Random number generation
fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453123);
}

fn hash3(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 45.164))) * 43758.5453);
}

fn rand(seed: ptr<function, f32>) -> f32 {
    *seed = hash(*seed + 1.0);
    return *seed;
}

fn rand_range(seed: ptr<function, f32>, min_val: f32, max_val: f32) -> f32 {
    return min_val + rand(seed) * (max_val - min_val);
}

// Lifecycle helpers
fn is_alive(p: Particle) -> bool {
    return p.alive != 0u;
}

fn is_dead(p: Particle) -> bool {
    return p.alive == 0u;
}

// Gradient noise helpers
fn mod289_3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn mod289_4(x: vec4<f32>) -> vec4<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn permute4(x: vec4<f32>) -> vec4<f32> {
    return mod289_4(((x * 34.0) + 1.0) * x);
}

fn taylor_inv_sqrt4(r: vec4<f32>) -> vec4<f32> {
    return 1.79284291400159 - 0.85373472095314 * r;
}

// 3D Simplex noise
fn noise3(v: vec3<f32>) -> f32 {
    let C = vec2<f32>(1.0/6.0, 1.0/3.0);
    let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);

    // First corner
    var i = floor(v + dot(v, vec3(C.y)));
    let x0 = v - i + dot(i, vec3(C.x));

    // Other corners
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);

    let x1 = x0 - i1 + C.x;
    let x2 = x0 - i2 + C.y;
    let x3 = x0 - D.yyy;

    // Permutations
    i = mod289_3(i);
    let p = permute4(permute4(permute4(
        i.z + vec4<f32>(0.0, i1.z, i2.z, 1.0))
      + i.y + vec4<f32>(0.0, i1.y, i2.y, 1.0))
      + i.x + vec4<f32>(0.0, i1.x, i2.x, 1.0));

    // Gradients
    let n_ = 0.142857142857;
    let ns = n_ * D.wyz - D.xzx;

    let j = p - 49.0 * floor(p * ns.z * ns.z);

    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_);

    let x = x_ * ns.x + ns.yyyy;
    let y = y_ * ns.x + ns.yyyy;
    let h = 1.0 - abs(x) - abs(y);

    let b0 = vec4<f32>(x.xy, y.xy);
    let b1 = vec4<f32>(x.zw, y.zw);

    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4<f32>(0.0));

    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;

    var p0 = vec3<f32>(a0.xy, h.x);
    var p1 = vec3<f32>(a0.zw, h.y);
    var p2 = vec3<f32>(a1.xy, h.z);
    var p3 = vec3<f32>(a1.zw, h.w);

    // Normalize gradients
    let norm = taylor_inv_sqrt4(vec4<f32>(dot(p0,p0), dot(p1,p1), dot(p2,p2), dot(p3,p3)));
    p0 *= norm.x;
    p1 *= norm.y;
    p2 *= norm.z;
    p3 *= norm.w;

    // Mix final noise value
    var m = max(0.6 - vec4<f32>(dot(x0,x0), dot(x1,x1), dot(x2,x2), dot(x3,x3)), vec4<f32>(0.0));
    m = m * m;
    return 42.0 * dot(m*m, vec4<f32>(dot(p0,x0), dot(p1,x1), dot(p2,x2), dot(p3,x3)));
}

// 2D Simplex noise (wrapper using z=0)
fn noise2(p: vec2<f32>) -> f32 {
    return noise3(vec3<f32>(p, 0.0));
}

// Fractal Brownian Motion - 3D
fn fbm3(p: vec3<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < octaves; i++) {
        value += amplitude * noise3(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// Fractal Brownian Motion - 2D
fn fbm2(p: vec2<f32>, octaves: i32) -> f32 {
    return fbm3(vec3<f32>(p, 0.0), octaves);
}

// HSV to RGB conversion
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let hp = h * 6.0;
    let x = c * (1.0 - abs(hp % 2.0 - 1.0));
    let m = v - c;

    var rgb: vec3<f32>;
    if hp < 1.0 {
        rgb = vec3<f32>(c, x, 0.0);
    } else if hp < 2.0 {
        rgb = vec3<f32>(x, c, 0.0);
    } else if hp < 3.0 {
        rgb = vec3<f32>(0.0, c, x);
    } else if hp < 4.0 {
        rgb = vec3<f32>(0.0, x, c);
    } else if hp < 5.0 {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }

    return rgb + vec3<f32>(m, m, m);
}
"#;

// ============================================
// Shape fragment shaders
// ============================================

const SHAPE_CIRCLE: &str = r#"
let dist = length(uv - center);
let alpha = 1.0 - smoothstep(0.4, 0.5, dist);
"#;

const SHAPE_CIRCLE_HARD: &str = r#"
let dist = length(uv - center);
let alpha = select(0.0, 1.0, dist < 0.5);
"#;

const SHAPE_SQUARE: &str = r#"
let alpha = 1.0;
"#;

const SHAPE_RING: &str = r#"
let dist = length(uv - center);
let alpha = smoothstep(0.3, 0.35, dist) * (1.0 - smoothstep(0.45, 0.5, dist));
"#;

const SHAPE_STAR: &str = r#"
let p = uv - center;
let angle = atan2(p.y, p.x);
let r = length(p);
let star = 0.3 + 0.2 * cos(angle * 5.0);
let alpha = 1.0 - smoothstep(star - 0.05, star, r);
"#;

const SHAPE_TRIANGLE: &str = r#"
let p = uv - center;
let a = abs(p.x) * 1.732 + p.y;
let b = -p.y * 2.0;
let alpha = select(0.0, 1.0, max(a, b) < 0.5);
"#;

const SHAPE_HEXAGON: &str = r#"
let p = abs(uv - center);
let hex = max(p.x * 0.866 + p.y * 0.5, p.y);
let alpha = 1.0 - smoothstep(0.4, 0.45, hex);
"#;

const SHAPE_DIAMOND: &str = r#"
let p = abs(uv - center);
let diamond = p.x + p.y;
let alpha = 1.0 - smoothstep(0.45, 0.5, diamond);
"#;

const SHAPE_POINT: &str = r#"
let alpha = 1.0;
"#;

// ============================================
// Spatial hashing utilities
// ============================================

/// WGSL code for Morton encoding utilities
const MORTON_WGSL: &str = r#"
// Expand 10-bit integer to 30 bits by inserting 2 zeros between each bit
fn expand_bits(v: u32) -> u32 {
    var x = v & 0x000003FFu; // 10 bits
    x = (x | (x << 16u)) & 0x030000FFu;
    x = (x | (x <<  8u)) & 0x0300F00Fu;
    x = (x | (x <<  4u)) & 0x030C30C3u;
    x = (x | (x <<  2u)) & 0x09249249u;
    return x;
}

// Compute 30-bit Morton code for 3D point (each coord 0-1023)
fn morton_encode(x: u32, y: u32, z: u32) -> u32 {
    return expand_bits(x) | (expand_bits(y) << 1u) | (expand_bits(z) << 2u);
}

// Convert world position to cell coordinates
fn pos_to_cell(pos: vec3<f32>, cell_size: f32, grid_res: u32) -> vec3<u32> {
    // Offset by half grid to center around origin
    let half_grid = f32(grid_res) * cell_size * 0.5;
    let normalized = (pos + vec3<f32>(half_grid)) / cell_size;
    let clamped = clamp(normalized, vec3<f32>(0.0), vec3<f32>(f32(grid_res - 1u)));
    return vec3<u32>(clamped);
}

// Get Morton code for a world position
fn pos_to_morton(pos: vec3<f32>, cell_size: f32, grid_res: u32) -> u32 {
    let cell = pos_to_cell(pos, cell_size, grid_res);
    return morton_encode(cell.x, cell.y, cell.z);
}

// Compact 30 bits to 10 bits by extracting every third bit
fn compact_bits(v: u32) -> u32 {
    var x = v & 0x09249249u;
    x = (x | (x >>  2u)) & 0x030C30C3u;
    x = (x | (x >>  4u)) & 0x0300F00Fu;
    x = (x | (x >>  8u)) & 0x030000FFu;
    x = (x | (x >> 16u)) & 0x000003FFu;
    return x;
}

// Decode Morton code back to cell coordinates
fn morton_decode(code: u32) -> vec3<u32> {
    return vec3<u32>(
        compact_bits(code),
        compact_bits(code >> 1u),
        compact_bits(code >> 2u)
    );
}
"#;

/// WGSL code for neighbor iteration utilities
const NEIGHBOR_UTILS_WGSL: &str = r#"
// Offsets for 27 neighboring cells (including self)
const NEIGHBOR_OFFSETS: array<vec3<i32>, 27> = array<vec3<i32>, 27>(
    vec3<i32>(-1, -1, -1), vec3<i32>(0, -1, -1), vec3<i32>(1, -1, -1),
    vec3<i32>(-1,  0, -1), vec3<i32>(0,  0, -1), vec3<i32>(1,  0, -1),
    vec3<i32>(-1,  1, -1), vec3<i32>(0,  1, -1), vec3<i32>(1,  1, -1),
    vec3<i32>(-1, -1,  0), vec3<i32>(0, -1,  0), vec3<i32>(1, -1,  0),
    vec3<i32>(-1,  0,  0), vec3<i32>(0,  0,  0), vec3<i32>(1,  0,  0),
    vec3<i32>(-1,  1,  0), vec3<i32>(0,  1,  0), vec3<i32>(1,  1,  0),
    vec3<i32>(-1, -1,  1), vec3<i32>(0, -1,  1), vec3<i32>(1, -1,  1),
    vec3<i32>(-1,  0,  1), vec3<i32>(0,  0,  1), vec3<i32>(1,  0,  1),
    vec3<i32>(-1,  1,  1), vec3<i32>(0,  1,  1), vec3<i32>(1,  1,  1),
);

// Get Morton code for a neighboring cell (returns 0xFFFFFFFF if out of bounds)
fn neighbor_cell_morton(cell: vec3<u32>, offset_idx: u32, grid_res: u32) -> u32 {
    let offset = NEIGHBOR_OFFSETS[offset_idx];
    let neighbor = vec3<i32>(cell) + offset;

    if neighbor.x < 0 || neighbor.y < 0 || neighbor.z < 0 ||
       neighbor.x >= i32(grid_res) || neighbor.y >= i32(grid_res) || neighbor.z >= i32(grid_res) {
        return 0xFFFFFFFFu; // Invalid marker
    }

    return morton_encode(u32(neighbor.x), u32(neighbor.y), u32(neighbor.z));
}
"#;
