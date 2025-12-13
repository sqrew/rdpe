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

use rdpe::prelude::*;

#[derive(rdpe::Particle, Clone)]
struct CubeParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    Simulation::<CubeParticle>::new()
        .with_particle_count(200)
        .with_bounds(1.5)
        .with_particle_size(0.01) // This controls the wireframe mesh scale
        .with_spawner(|ctx| {
            // Grid layout
            let pos = ctx.grid_position(10, 10, 2);
            CubeParticle {
                position: pos,
                velocity: ctx.random_direction() * 0.1,
                color: ctx.hsv((pos.x + 1.0) / 2.0, 0.8, 1.0),
            }
        })
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
        .run().expect("Simulation failed");
}
