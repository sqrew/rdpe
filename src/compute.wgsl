struct Particle {
    position: vec3<f32>,
    _pad0: f32,
    velocity: vec3<f32>,
    _pad1: f32,
};

struct Uniforms {
    view_proj: mat4x4<f32>,
    time: f32,
    delta_time: f32,
};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

const BOUNDS: f32 = 1.0;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_particles = arrayLength(&particles);

    if index >= num_particles {
        return;
    }

    var p = particles[index];

    // Integrate velocity
    p.position += p.velocity * uniforms.delta_time;

    // Bounce off walls
    if p.position.x < -BOUNDS {
        p.position.x = -BOUNDS;
        p.velocity.x = abs(p.velocity.x);
    } else if p.position.x > BOUNDS {
        p.position.x = BOUNDS;
        p.velocity.x = -abs(p.velocity.x);
    }

    if p.position.y < -BOUNDS {
        p.position.y = -BOUNDS;
        p.velocity.y = abs(p.velocity.y);
    } else if p.position.y > BOUNDS {
        p.position.y = BOUNDS;
        p.velocity.y = -abs(p.velocity.y);
    }

    if p.position.z < -BOUNDS {
        p.position.z = -BOUNDS;
        p.velocity.z = abs(p.velocity.z);
    } else if p.position.z > BOUNDS {
        p.position.z = BOUNDS;
        p.velocity.z = -abs(p.velocity.z);
    }

    particles[index] = p;
}
