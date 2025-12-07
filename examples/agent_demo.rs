//! # Agent State Machine Demo
//!
//! Demonstrates the `Rule::Agent` state machine system with creatures that
//! exhibit different behaviors based on their internal state:
//!
//! - **Wandering** (blue): Random movement, looking for food
//! - **Chasing** (red): Spotted food, pursuing it
//! - **Eating** (green): Consuming food, stationary
//! - **Resting** (gray): Low energy, recovering
//!
//! The demo shows:
//! - Entry/exit actions (color changes, velocity resets)
//! - Update actions (energy consumption, movement)
//! - State timer usage (eating duration)
//! - Conditional transitions based on particle fields
//!
//! Run with: `cargo run --example agent_demo`

use rand::Rng;
use rdpe::prelude::*;
use std::f32::consts::TAU;

// State constants for readability
const STATE_WANDERING: u32 = 0;
const STATE_CHASING: u32 = 1;
const STATE_EATING: u32 = 2;
const STATE_RESTING: u32 = 3;

#[derive(Particle, Clone)]
struct Creature {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,

    // Agent state machine fields
    state: u32,      // Current state
    prev_state: u32, // Previous state (for edge detection)
    state_timer: f32, // Time in current state

    // Agent properties
    energy: f32,       // 0.0 to 1.0, depletes while chasing
    food_dist: f32,    // Distance to nearest food (simulated)
    food_detected: f32, // 1.0 if food nearby, 0.0 otherwise
}

fn main() {
    let mut rng = rand::thread_rng();

    // Create creatures
    let particles: Vec<Creature> = (0..500)
        .map(|_| {
            // Random position in a sphere
            let theta = rng.gen_range(0.0..TAU);
            let phi = rng.gen_range(0.0..std::f32::consts::PI);
            let r = rng.gen_range(0.1..0.8);

            let x = r * phi.sin() * theta.cos();
            let y = r * phi.cos();
            let z = r * phi.sin() * theta.sin();

            Creature {
                position: Vec3::new(x, y, z),
                velocity: Vec3::ZERO,
                color: Vec3::new(0.3, 0.5, 0.8), // Start blue (wandering)
                state: STATE_WANDERING,
                prev_state: STATE_WANDERING,
                state_timer: 0.0,
                energy: rng.gen_range(0.5..1.0),
                food_dist: 1.0,
                food_detected: 0.0,
            }
        })
        .collect();

    Simulation::<Creature>::new()
        .with_particle_count(500)
        .with_particle_size(0.015)
        .with_bounds(1.0)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())

        // Simulate food detection (using noise as proxy for "food location")
        .with_rule(Rule::Custom(
            r#"
            // Simulate food detection based on position (food "hotspots")
            let food_pos = vec3<f32>(
                sin(uniforms.time * 0.3) * 0.5,
                cos(uniforms.time * 0.4) * 0.3,
                sin(uniforms.time * 0.2) * 0.5
            );
            p.food_dist = length(p.position - food_pos);
            p.food_detected = select(0.0, 1.0, p.food_dist < 0.4);
        "#
            .into(),
        ))

        // The Agent state machine
        .with_rule(Rule::Agent {
            state_field: "state".into(),
            prev_state_field: "prev_state".into(),
            state_timer_field: Some("state_timer".into()),
            states: vec![
                // STATE 0: Wandering
                AgentState::new(STATE_WANDERING)
                    .named("wandering")
                    .on_enter(
                        r#"
                        p.color = vec3<f32>(0.3, 0.5, 0.8); // Blue
                    "#,
                    )
                    .on_update(
                        r#"
                        // Slowly regenerate energy while wandering
                        p.energy = min(1.0, p.energy + 0.05 * uniforms.delta_time);
                    "#,
                    )
                    // Transitions
                    .transition(STATE_CHASING, "p.food_detected > 0.5 && p.energy > 0.3")
                    .transition(STATE_RESTING, "p.energy < 0.15"),

                // STATE 1: Chasing
                AgentState::new(STATE_CHASING)
                    .named("chasing")
                    .on_enter(
                        r#"
                        p.color = vec3<f32>(1.0, 0.3, 0.2); // Red
                    "#,
                    )
                    .on_update(
                        r#"
                        // Consume energy while chasing
                        p.energy = max(0.0, p.energy - 0.15 * uniforms.delta_time);
                        // Move faster toward food
                        let food_pos = vec3<f32>(
                            sin(uniforms.time * 0.3) * 0.5,
                            cos(uniforms.time * 0.4) * 0.3,
                            sin(uniforms.time * 0.2) * 0.5
                        );
                        let to_food = normalize(food_pos - p.position);
                        p.velocity += to_food * 3.0 * uniforms.delta_time;
                    "#,
                    )
                    .on_exit(
                        r#"
                        // Slow down when transitioning out of chase
                        p.velocity *= 0.5;
                    "#,
                    )
                    // Transitions
                    .transition(STATE_EATING, "p.food_dist < 0.1")
                    .transition(STATE_WANDERING, "p.food_detected < 0.5")
                    .transition(STATE_RESTING, "p.energy < 0.1"),

                // STATE 2: Eating
                AgentState::new(STATE_EATING)
                    .named("eating")
                    .on_enter(
                        r#"
                        p.color = vec3<f32>(0.2, 0.9, 0.3); // Green
                        p.velocity = vec3<f32>(0.0, 0.0, 0.0); // Stop moving
                    "#,
                    )
                    .on_update(
                        r#"
                        // Gain energy while eating
                        p.energy = min(1.0, p.energy + 0.4 * uniforms.delta_time);
                        // Pulse size while eating
                        p.scale = 1.0 + sin(p.state_timer * 10.0) * 0.3;
                    "#,
                    )
                    .on_exit(
                        r#"
                        p.scale = 1.0; // Reset scale
                    "#,
                    )
                    // Transition after eating for 1.5 seconds
                    .transition(STATE_WANDERING, "p.state_timer > 1.5"),

                // STATE 3: Resting
                AgentState::new(STATE_RESTING)
                    .named("resting")
                    .on_enter(
                        r#"
                        p.color = vec3<f32>(0.5, 0.5, 0.5); // Gray
                        p.velocity *= 0.1; // Slow down significantly
                    "#,
                    )
                    .on_update(
                        r#"
                        // Slowly recover energy
                        p.energy = min(1.0, p.energy + 0.15 * uniforms.delta_time);
                        // Gentle breathing animation
                        p.scale = 0.8 + sin(p.state_timer * 3.0) * 0.15;
                    "#,
                    )
                    .on_exit(
                        r#"
                        p.scale = 1.0;
                    "#,
                    )
                    // Resume wandering when energy is restored
                    .transition(STATE_WANDERING, "p.energy > 0.6"),
            ],
        })

        // Dim color when low energy
        .with_rule(Rule::Custom(
            r#"
            p.color *= (0.5 + p.energy * 0.5);
        "#
            .into(),
        ))

        // Add wander movement (separate from state machine for reliability)
        .with_rule(Rule::Wander {
            strength: 0.8,
            frequency: 2.0,
        })

        // Physics
        .with_rule(Rule::Drag(1.5))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.0 })
        .with_rule(Rule::BounceWalls)

        // Visuals
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.02, 0.02, 0.05));
        })
        .run();
}
