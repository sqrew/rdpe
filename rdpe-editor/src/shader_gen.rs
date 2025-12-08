//! Shader generation for the embedded simulation.
//!
//! This module generates WGSL compute and render shaders from SimConfig,
//! using the actual rdpe rule system for proper behavior.

use crate::config::{SimConfig, ParticleShapeConfig};
use crate::particle::MetaParticle;
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
/// 1. Defines the MetaParticle struct (from derive macro)
/// 2. Defines uniforms (view_proj, time, delta_time)
/// 3. Applies all rules in order
/// 4. Integrates velocity and updates position
pub fn generate_compute_shader(config: &SimConfig) -> String {
    let particle_struct = MetaParticle::wgsl_struct();

    // Convert rules to rdpe::Rule and then to WGSL
    let rules: Vec<Rule> = config.rules.iter().map(|r| r.to_rule()).collect();

    // Check if any rules need neighbor access
    let needs_neighbors = rules.iter().any(|r| r.requires_neighbors());

    if needs_neighbors {
        generate_compute_shader_with_neighbors(config, &rules, particle_struct)
    } else {
        generate_compute_shader_simple(config, &rules, particle_struct)
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

    format!(r#"
// ============================================
// RDPE Compute Shader (Generated)
// ============================================

// Particle struct
{particle_struct}

// Uniforms
struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
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

    // Skip dead particles
    if (p.alive == 0u) {{
        return;
    }}

    let time = uniforms.time;
    let delta_time = uniforms.delta_time;
    let bounds = {bounds:.6};
    {field_count_decl}

    // ============================================
    // Apply rules
    // ============================================
{rules_code}

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
    )
}

/// Generate compute shader with spatial hashing for neighbor queries.
fn generate_compute_shader_with_neighbors(config: &SimConfig, rules: &[Rule], particle_struct: &str) -> String {
    // For now, fall back to simple shader without neighbor support
    // Full neighbor support requires the spatial hashing system
    // which is complex to set up outside the full Simulation

    // Filter to non-neighbor rules and generate those
    let simple_rules: Vec<&Rule> = rules.iter().filter(|r| !r.requires_neighbors()).collect();
    let rules_code: String = simple_rules
        .iter()
        .map(|r| r.to_wgsl(config.bounds))
        .collect::<Vec<_>>()
        .join("\n\n");

    // Generate custom uniform fields
    let custom_uniform_fields = generate_custom_uniform_fields(config);

    // Generate field code (if any fields are defined)
    let field_code = generate_field_code(config);
    let has_fields = !config.fields.is_empty();

    // TODO: Add proper spatial hashing support
    // For now, just warn that neighbor rules won't work
    let neighbor_warning = if rules.iter().any(|r| r.requires_neighbors()) {
        "// WARNING: Some rules require neighbor access (Separate, Cohere, Align, etc.)\n    // These rules are not yet supported in the embedded editor.\n"
    } else {
        ""
    };

    format!(r#"
// ============================================
// RDPE Compute Shader (Generated)
// ============================================

// Particle struct
{particle_struct}

// Uniforms
struct Uniforms {{
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
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

    // Skip dead particles
    if (p.alive == 0u) {{
        return;
    }}

    let time = uniforms.time;
    let delta_time = uniforms.delta_time;
    let bounds = {bounds:.6};
    {field_count_decl}

    {neighbor_warning}
    // ============================================
    // Apply rules
    // ============================================
{rules_code}

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
        neighbor_warning = neighbor_warning,
        rules_code = indent_code(&rules_code, "    "),
    )
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

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) particle_pos: vec3<f32>,
    @location(1) particle_color: vec3<f32>,
    @location(2) alive: u32,
    @location(3) scale: f32,
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
    var color_mod = particle_color;

    // ============================================
    // Apply vertex effects
    // ============================================
{vertex_effects_code}
{custom_vertex_code}
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
        vertex_effects_code = indent_code(&vertex_effects_code, "    "),
        custom_vertex_code = custom_vertex_code,
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
