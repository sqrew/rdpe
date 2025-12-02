//! # Orbiting Attractor
//!
//! An automatic attractor that orbits through space while pulsing
//! between attraction and repulsion.
//!
//! ## What This Demonstrates
//!
//! - Time-based uniform updates
//! - Oscillating parameters for rhythmic effects
//! - 3D orbital motion using trigonometry
//!
//! ## The Animation Loop
//!
//! Every frame, `with_update()` is called with `ctx.time()`:
//!
//! ```ignore
//! .with_update(|ctx| {
//!     let t = ctx.time();
//!
//!     // Circular orbit
//!     ctx.set("attractor", Vec3::new(t.cos(), t.sin(), 0.0));
//!
//!     // Pulsing strength
//!     let cycle = t % 4.0;
//!     let strength = if cycle < 3.0 { 3.0 } else { -5.0 };
//!     ctx.set("strength", strength);
//! })
//! ```
//!
//! ## The Gather-Scatter Cycle
//!
//! - **Seconds 0-3**: Attraction pulls particles toward orbiting point
//! - **Seconds 3-4**: Repulsion scatters them dramatically
//! - **Repeat**: Creates rhythmic "breathing" effect
//!
//! ## Try This
//!
//! - Change orbit to figure-8: `(2t).sin()` for x
//! - Use sine wave for smooth strength pulsing
//! - Add second attractor on opposite orbit
//! - Make orbit radius change over time
//!
//! Run with: `cargo run --example orbiting_attractor`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Mote {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    let mut rng = rand::thread_rng();

    let particles: Vec<Mote> = (0..10_000)
        .map(|_| Mote {
            position: Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            ),
            velocity: Vec3::ZERO,
            color: Vec3::new(0.8, 0.5, 1.0),
        })
        .collect();

    Simulation::<Mote>::new()
        .with_particle_count(10_000)
        .with_bounds(2.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        // Uniforms for orbiting attractor
        .with_uniform("attractor", Vec3::ZERO)
        .with_uniform("strength", 1.0f32)
        .with_update(|ctx| {
            let t = ctx.time();

            // Attractor orbits in 3D (Lissajous-like path)
            ctx.set(
                "attractor",
                Vec3::new(t.cos() * 0.8, t.sin() * 0.5, t.sin() * 0.8),
            );

            // Pulse: gather for 3 sec, scatter for 1 sec
            let cycle = t % 10.0;
            let strength = if cycle < 9.0 {
                3.0 // Attract
            } else {
                -5.0 // Repel
            };
            ctx.set("strength", strength);
        })
        // Attraction/repulsion force
        .with_rule(Rule::Custom(
            r#"
            let to_attractor = uniforms.attractor - p.position;
            let dist = length(to_attractor);
            if dist > 0.01 {
                let dir = to_attractor / dist;
                let force = uniforms.strength / (dist * dist + 0.3);
                p.velocity += dir * force * uniforms.delta_time;
            }
"#
            .into(),
        ))
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .run();
}
