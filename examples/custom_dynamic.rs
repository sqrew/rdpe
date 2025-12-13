//! # Custom Dynamic Rules
//!
//! Demonstrates the dynamic custom rule API - the killer feature of RDPE!
//!
//! This example shows how to create custom WGSL rules with parameters that
//! can be edited in real-time through the rule inspector.
//!
//! ## What This Demonstrates
//!
//! - **`Rule::custom_dynamic()`** - Custom WGSL with editable params
//! - **`Rule::neighbor_custom_dynamic()`** - Neighbor rules with editable params
//! - **`.with_param()`** - Declare typed parameters for runtime editing
//! - **Rule Inspector** - All params visible and editable in the UI
//!
//! ## Try This
//!
//! 1. Open the Rule Inspector window
//! 2. Expand "Custom (Dynamic)" rules
//! 3. Drag the parameter values to see instant changes
//! 4. Try extreme values to see emergent behaviors!

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    energy: f32,
    phase: f32,
}

fn main() {
    Simulation::<Particle>::new()
        .with_particle_count(5000)
        .with_bounds(1.0)
        .with_spatial_config(0.15, 32)
        .with_spawner(|ctx| {
            let i = ctx.index;
            let angle = i as f32 * 0.1;
            let radius = 0.3 + (i as f32 * 0.001) % 0.4;
            Particle {
                position: Vec3::new(
                    angle.cos() * radius,
                    ctx.random_range(-0.1, 0.1),
                    angle.sin() * radius,
                ),
                velocity: Vec3::ZERO,
                color: Vec3::new(0.5, 0.7, 1.0),
                energy: 1.0,
                phase: ctx.random_range(0.0, 6.28),
            }
        })
        // Custom pulsing force - all params editable!
        .with_rule(Rule::custom_dynamic(r#"
    // Pulsing radial force from center
    let to_center = -p.position;
    let dist = length(to_center);
    if dist > 0.01 {
        let pulse = sin(uniforms.time * uniforms.pulse_freq + p.phase) * 0.5 + 0.5;
        let force = normalize(to_center) * uniforms.radial_strength * pulse;
        p.velocity += force * uniforms.delta_time;
    }

    // Spinning motion
    let tangent = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), to_center));
    p.velocity += tangent * uniforms.spin_speed * uniforms.delta_time;
"#)
            .with_param("pulse_freq", 3.0)
            .with_param("radial_strength", 2.0)
            .with_param("spin_speed", 1.5)
        )
        // Custom color based on energy and speed
        .with_rule(Rule::custom_dynamic(r#"
    // Energy-based coloring
    let speed = length(p.velocity);
    let energy_color = vec3<f32>(
        uniforms.color_r + speed * 0.2,
        uniforms.color_g * p.energy,
        uniforms.color_b * (1.0 - speed * 0.1)
    );
    p.color = clamp(energy_color, vec3<f32>(0.0), vec3<f32>(1.0));

    // Energy decay
    p.energy = max(0.1, p.energy - uniforms.energy_decay * uniforms.delta_time);
"#)
            .with_param("color_r", 0.4)
            .with_param("color_g", 0.8)
            .with_param("color_b", 1.0)
            .with_param("energy_decay", 0.1)
        )
        // Custom neighbor interaction - editable attraction/repulsion!
        .with_rule(Rule::neighbor_custom_dynamic(r#"
    if neighbor_dist < uniforms.interact_radius && neighbor_dist > 0.01 {
        // Soft boundary - repel when too close, attract when far
        let ideal_dist = uniforms.ideal_spacing;
        let diff = neighbor_dist - ideal_dist;
        let force_dir = neighbor_dir * sign(diff);
        let force_mag = abs(diff) * uniforms.interact_strength;
        p.velocity += force_dir * force_mag * uniforms.delta_time;

        // Energy transfer on close proximity
        if neighbor_dist < ideal_dist * 0.5 {
            p.energy = min(1.0, p.energy + uniforms.energy_transfer);
        }
    }
"#)
            .with_param("interact_radius", 0.12)
            .with_param("ideal_spacing", 0.05)
            .with_param("interact_strength", 3.0)
            .with_param("energy_transfer", 0.01)
        )
        // Standard rules with dynamic params
        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 2.0 })
        .with_rule(Rule::BounceWalls)
        // Enable both inspectors!
        .with_particle_inspector()
        .with_rule_inspector()
        .run().expect("Simulation failed");
}
