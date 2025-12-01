struct Uniforms {
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) particle_pos: vec3<f32>,
) -> VertexOutput {
    // Generate a quad from 6 vertices (2 triangles)
    var quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    let quad_pos = quad_vertices[vertex_index];
    let particle_size = 0.015;

    // Billboard: offset in screen space after projection
    let world_pos = vec4<f32>(particle_pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;

    // Apply quad offset in clip space (screen-aligned billboard)
    clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
    clip_pos.y += quad_pos.y * particle_size * clip_pos.w;

    var out: VertexOutput;
    out.clip_position = clip_pos;

    // Color based on position with time-based hue shift
    let base_color = normalize(particle_pos) * 0.5 + 0.5;
    let hue_shift = uniforms.time * 0.1;
    out.color = vec3<f32>(
        base_color.x * cos(hue_shift) + base_color.y * sin(hue_shift),
        base_color.y * cos(hue_shift) - base_color.x * sin(hue_shift) * 0.5,
        base_color.z
    );
    out.uv = quad_pos;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Circular particle with soft edges
    let dist = length(in.uv);
    if dist > 1.0 {
        discard;
    }

    // Soft falloff
    let alpha = 1.0 - smoothstep(0.5, 1.0, dist);

    return vec4<f32>(in.color, alpha);
}
