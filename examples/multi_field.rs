//! # Multi-Field Example
//!
//! Demonstrates using multiple 3D spatial fields simultaneously.
//! Two types of particles each deposit to their own field and avoid the other.
//!
//! - Red particles deposit to field 0, avoid field 1
//! - Blue particles deposit to field 1, avoid field 0
//!
//! Run with: `cargo run --example multi_field`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Agent {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Which team: 0 = red (deposits field 0), 1 = blue (deposits field 1)
    team: u32,
    heading: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Pre-generate particles - half red, half blue
    let particles: Vec<_> = (0..20_000)
        .map(|i| {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let radius = rng.gen_range(0.0_f32..0.8).sqrt();
            let pos = Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
            let heading = rng.gen_range(0.0..std::f32::consts::TAU);
            let team = if i < 10_000 { 0u32 } else { 1u32 };
            (pos, heading, team)
        })
        .collect();

    Simulation::<Agent>::new()
        .with_particle_count(20_000)
        .with_bounds(1.2)
        .with_spawner(move |i, _| {
            let (pos, heading, team) = particles[i as usize];
            let color = if team == 0 {
                Vec3::new(1.0, 0.3, 0.2) // Red team
            } else {
                Vec3::new(0.2, 0.4, 1.0) // Blue team
            };
            Agent {
                position: pos,
                velocity: Vec3::ZERO,
                color,
                team,
                heading,
            }
        })
        // Field 0: Red team pheromones
        .with_field(
            "red_pheromone",
            FieldConfig::new(48)
                .with_extent(1.2)
                .with_decay(0.97)
                .with_blur(0.15),
        )
        // Field 1: Blue team pheromones
        .with_field(
            "blue_pheromone",
            FieldConfig::new(48)
                .with_extent(1.2)
                .with_decay(0.97)
                .with_blur(0.15),
        )
        .with_rule(Rule::Custom(
            r#"
            let dt = uniforms.delta_time;
            let speed = 0.4;
            let turn_speed = 3.0;
            let sense_dist = 0.12;
            let sense_angle = 0.5;
            let deposit = 0.15;

            // Each team deposits to their own field
            let my_field = p.team;
            let other_field = 1u - p.team;

            // Deposit pheromone to my team's field
            field_write(my_field, p.position, deposit);

            // Calculate forward direction
            let forward = vec3<f32>(cos(p.heading), 0.0, sin(p.heading));

            // Sense positions
            let sense_fwd = p.position + forward * sense_dist;
            let left_angle = p.heading + sense_angle;
            let right_angle = p.heading - sense_angle;
            let sense_left = p.position + vec3<f32>(cos(left_angle), 0.0, sin(left_angle)) * sense_dist;
            let sense_right = p.position + vec3<f32>(cos(right_angle), 0.0, sin(right_angle)) * sense_dist;

            // Sample MY team's pheromone (attractive)
            let my_fwd = field_read(my_field, sense_fwd);
            let my_left = field_read(my_field, sense_left);
            let my_right = field_read(my_field, sense_right);

            // Sample OTHER team's pheromone (repulsive)
            let other_fwd = field_read(other_field, sense_fwd);
            let other_left = field_read(other_field, sense_left);
            let other_right = field_read(other_field, sense_right);

            // Combined scores: follow my team, avoid other team
            let score_fwd = my_fwd - other_fwd * 1.5;
            let score_left = my_left - other_left * 1.5;
            let score_right = my_right - other_right * 1.5;

            // Turn toward best combined score
            if score_left > score_fwd && score_left > score_right {
                p.heading += turn_speed * dt;
            } else if score_right > score_fwd {
                p.heading -= turn_speed * dt;
            }

            // Small random wiggle
            p.heading += sin(uniforms.time * 8.0 + f32(p.alive) * 77.7) * 0.15 * dt;

            // Move forward
            let new_forward = vec3<f32>(cos(p.heading), 0.0, sin(p.heading));
            p.position += new_forward * speed * dt;

            // Wrap at boundaries
            if p.position.x > 1.1 { p.position.x = -1.1; }
            if p.position.x < -1.1 { p.position.x = 1.1; }
            if p.position.z > 1.1 { p.position.z = -1.1; }
            if p.position.z < -1.1 { p.position.z = 1.1; }
            p.position.y = 0.0;

            // Update color intensity based on pheromone concentration
            let my_intensity = clamp(field_read(my_field, p.position) * 3.0, 0.0, 1.0);
            if p.team == 0u {
                p.color = vec3<f32>(0.4 + my_intensity * 0.6, 0.2, 0.1);
            } else {
                p.color = vec3<f32>(0.1, 0.2 + my_intensity * 0.3, 0.4 + my_intensity * 0.6);
            }

            p.velocity = vec3<f32>(0.0, 0.0, 0.0);
        "#
            .into(),
        ))
        .run();
}
