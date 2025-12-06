//! # Wireframe Example
//!
//! Demonstrates rendering particles as 3D wireframe meshes instead of billboards.
//!
//! ## What This Demonstrates
//!
//! - `WireframeMesh` - render particles as 3D wireframe shapes
//! - Built-in shapes: cube, tetrahedron, octahedron, icosahedron
//! - Custom shapes from line segments
//! - Wireframe line thickness control
//!
//! ## Available Shapes
//!
//! - `WireframeMesh::cube()` - 12 edges
//! - `WireframeMesh::tetrahedron()` - 6 edges
//! - `WireframeMesh::octahedron()` - 12 edges
//! - `WireframeMesh::icosahedron()` - 30 edges
//! - `WireframeMesh::diamond()` - 8 edges
//! - `WireframeMesh::star()` - spiky star shape
//! - `WireframeMesh::spiral(turns, segments)` - helix shape
//! - `WireframeMesh::custom(lines)` - your own line segments
//!
//! ## Try This
//!
//! - Change the mesh type (cube, tetrahedron, octahedron, etc.)
//! - Adjust line thickness (0.001 to 0.01 works well)
//! - Try additive blending for glowing wireframes
//! - Increase particle count for a mesh field effect
//!
//! Run with: `cargo run --example wireframe`

use rand::Rng;
use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct CubeParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();
    let count = 200;

    // Pre-generate particles
    let particles: Vec<CubeParticle> = (0..count)
        .map(|i| {
            // Spawn in a grid pattern
            let x = (i % 10) as f32 * 0.2 - 0.9;
            let y = ((i / 10) % 10) as f32 * 0.2 - 0.9;
            let z = (i / 100) as f32 * 0.2 - 0.1;

            // Slight random velocity
            let vx = rng.gen_range(-0.1..0.1);
            let vy = rng.gen_range(-0.1..0.1);
            let vz = rng.gen_range(-0.1..0.1);

            // Color based on position
            let hue = (x + 1.0) / 2.0;
            let color = hsv_to_rgb(hue, 0.8, 1.0);

            CubeParticle {
                position: Vec3::new(x, y, z),
                velocity: Vec3::new(vx, vy, vz),
                color,
            }
        })
        .collect();

    Simulation::<CubeParticle>::new()
        .with_particle_count(count)
        .with_bounds(1.5)
        .with_particle_size(0.01) // This controls the wireframe mesh scale
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Wireframe rendering - each particle is a 3D mesh!
        .with_visuals(|v| {
            // Try different shapes:
            // - WireframeMesh::cube()
            // - WireframeMesh::tetrahedron()
            // - WireframeMesh::octahedron()
            // - WireframeMesh::icosahedron()
            // - WireframeMesh::diamond()
            v.wireframe(WireframeMesh::cube(), 0.001);
            v.blend_mode(BlendMode::Additive); // Glowing wireframes
        })
        // Gentle attraction to center
        .with_rule(Rule::AttractTo {
            point: Vec3::ZERO,
            strength: 0.2,
        })
        // Add some swirl
        .with_rule(Rule::Custom(
            r#"
            let r = length(p.position.xz);
            let swirl = 0.2 / (r + 0.2);
            p.velocity += vec3<f32>(-p.position.z, 0.0, p.position.x) * swirl * uniforms.delta_time;
            "#
            .into(),
        ))
        // Light drag
        .with_rule(Rule::Drag(0.5))
        // Speed limit
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.8 })
        // Bounce off walls
        .with_rule(Rule::BounceWalls)
        .run();
}

// Simple HSV to RGB conversion
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Vec3::new(r + m, g + m, b + m)
}
