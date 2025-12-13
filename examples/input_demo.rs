//! # Input Demo
//!
//! Demonstrates the full Input API with keyboard and mouse interactions.
//!
//! ## Controls
//!
//! - **Left mouse (hold)**: Attract particles toward cursor
//! - **Right mouse (hold)**: Repel particles from cursor
//! - **Space (press)**: Create a burst/explosion effect
//! - **R (press)**: Reset particles to center
//! - **Arrow keys (hold)**: Move the force origin
//!
//! ## What This Demonstrates
//!
//! - `ctx.input` - Full input state access
//! - `key_pressed()` - Detect key just pressed this frame
//! - `key_held()` - Detect key continuously held
//! - `mouse_held()` - Detect mouse button held
//! - `mouse_ndc()` - Get mouse position in normalized device coordinates
//! - `MouseButton::Left/Right` - Mouse button identifiers
//! - `KeyCode::Space/R/Up/Down/Left/Right` - Key identifiers
//!
//! ## Input vs Held
//!
//! - `key_pressed()` fires once when key goes down (good for triggers)
//! - `key_held()` fires every frame while held (good for continuous motion)
//!
//! Run with: `cargo run --example input_demo`

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

    let particles: Vec<Mote> = (0..20_000)
        .map(|_| Mote {
            position: Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            ),
            velocity: Vec3::ZERO,
            color: Vec3::new(0.3, 0.6, 1.0),
        })
        .collect();

    Simulation::<Mote>::new()
        .with_particle_count(20_000)
        .with_bounds(2.0)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())
        // Custom uniforms for forces
        .with_uniform("force_pos", Vec3::ZERO)
        .with_uniform("attract_strength", 0.0f32)
        .with_uniform("repel_strength", 0.0f32)
        .with_uniform("burst", 0.0f32)
        .with_uniform("reset", 0.0f32)
        // Update uniforms based on input
        .with_update(|ctx| {
            let mouse = ctx.input.mouse_ndc();
            let force_pos = Vec3::new(mouse.x * 2.0, mouse.y * 2.0, 0.0);

            // Mouse: left attracts, right repels
            let attract = if ctx.input.mouse_held(MouseButton::Left) { 8.0 } else { 0.0 };
            let repel = if ctx.input.mouse_held(MouseButton::Right) { 12.0 } else { 0.0 };

            ctx.set("force_pos", force_pos);
            ctx.set("attract_strength", attract);
            ctx.set("repel_strength", repel);

            // Space: burst effect (fires once per press)
            if ctx.input.key_pressed(KeyCode::Space) {
                ctx.set("burst", 1.0f32);
            } else {
                ctx.set("burst", 0.0f32);
            }

            // R: reset to center (fires once per press)
            if ctx.input.key_pressed(KeyCode::R) {
                ctx.set("reset", 1.0f32);
            } else {
                ctx.set("reset", 0.0f32);
            }
        })
        // Force rules
        .with_rule(Rule::Custom(
            r#"
            // Attraction force (mouse left click)
            if uniforms.attract_strength > 0.0 {
                let to_target = uniforms.force_pos - p.position;
                let dist = length(to_target);
                if dist > 0.05 {
                    let dir = to_target / dist;
                    let force = uniforms.attract_strength / (dist + 0.5);
                    p.velocity += dir * force * uniforms.delta_time;
                }
            }

            // Repulsion force (mouse right click)
            if uniforms.repel_strength > 0.0 {
                let away = p.position - uniforms.force_pos;
                let dist = length(away);
                if dist > 0.05 && dist < 2.0 {
                    let dir = away / dist;
                    let force = uniforms.repel_strength / (dist * dist + 0.3);
                    p.velocity += dir * force * uniforms.delta_time;
                }
            }

            // Burst effect (Space key)
            if uniforms.burst > 0.0 {
                let away = p.position - uniforms.force_pos;
                let dist = length(away);
                if dist > 0.01 && dist < 1.5 {
                    let dir = away / dist;
                    p.velocity += dir * 3.0;
                }
            }

            // Reset to center (R key)
            if uniforms.reset > 0.0 {
                p.position = p.position * 0.1;
                p.velocity = vec3<f32>(0.0);
            }

            // Color based on speed
            let speed = length(p.velocity);
            p.color = mix(
                vec3<f32>(0.3, 0.6, 1.0),  // Blue when slow
                vec3<f32>(1.0, 0.4, 0.2),  // Orange when fast
                clamp(speed / 3.0, 0.0, 1.0)
            );
"#
            .into(),
        ))
        .with_rule(Rule::Drag(2.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 5.0 })
        .with_rule(Rule::BounceWalls)
        .run().expect("Simulation failed");
}
