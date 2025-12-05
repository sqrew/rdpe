//! # Chemistry Toy
//!
//! A chemistry-inspired particle simulation with multiple atom types,
//! affinities, and reactions.
//!
//! **Elements:**
//! - Hydrogen (white/yellow) - light, fast, loves oxygen
//! - Oxygen (red) - attracts hydrogen, forms water
//! - Nitrogen (blue) - inert, clusters with itself
//! - Carbon (gray) - bonds with everything
//!
//! When hydrogen and oxygen get close enough, they can "react" and
//! become water (cyan), releasing energy (speed burst).
//!
//! Run with: `cargo run --example chemistry`

use rand::Rng;
use rdpe::prelude::*;
use std::f32::consts::TAU;

// Element types
const HYDROGEN: u32 = 0;
const OXYGEN: u32 = 1;
const NITROGEN: u32 = 2;
const CARBON: u32 = 3;
const WATER: u32 = 4; // H + O reaction product

#[derive(Particle, Clone)]
struct Atom {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,
    energy: f32,     // kinetic energy proxy / temperature
    charge: f32,     // slight charge for electromagnetic effects
    mass: f32,       // affects how forces move it
    bonds: f32,      // how many bonds this atom has (affects reactivity)
}

fn main() {
    let mut rng = rand::thread_rng();

    // Create atoms with different distributions
    let particles: Vec<Atom> = (0..1500)
        .map(|i| {
            // Random position in sphere
            let theta = rng.gen_range(0.0..TAU);
            let phi = rng.gen_range(0.0..std::f32::consts::PI);
            let r = rng.gen_range(0.1..0.9);

            let x = r * phi.sin() * theta.cos();
            let y = r * phi.cos();
            let z = r * phi.sin() * theta.sin();

            // Distribute elements: 50% H, 25% O, 15% N, 10% C
            let (element, color, mass, charge) = match i % 20 {
                0..=9 => (HYDROGEN, Vec3::new(1.0, 1.0, 0.8), 1.0, 0.1),   // H - light, slight positive
                10..=14 => (OXYGEN, Vec3::new(1.0, 0.3, 0.2), 16.0, -0.2), // O - heavy, slight negative
                15..=17 => (NITROGEN, Vec3::new(0.3, 0.5, 1.0), 14.0, 0.0), // N - inert
                _ => (CARBON, Vec3::new(0.4, 0.4, 0.4), 12.0, 0.0),        // C - neutral
            };

            Atom {
                position: Vec3::new(x, y, z),
                velocity: Vec3::new(
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(-0.2..0.2),
                ),
                color,
                particle_type: element,
                energy: rng.gen_range(0.3..0.7),
                charge,
                mass,
                bonds: 0.0,
            }
        })
        .collect();

    Simulation::<Atom>::new()
        .with_particle_count(1500)
        .with_particle_size(0.012)
        .with_bounds(1.0)
        .with_spawner(move |i, _| particles[i as usize].clone())
        .with_spatial_config(0.15, 32)

        // === REACTIONS ===
        // Hydrogen + Oxygen → Water (with probability)
        .with_rule(Rule::Convert {
            from_type: HYDROGEN,
            trigger_type: OXYGEN,
            to_type: WATER,
            radius: 0.03,
            probability: 0.002,
        })

        // === INTER-ELEMENT FORCES ===

        // Hydrogen attracts Oxygen (wants to bond)
        .with_rule(Rule::Typed {
            self_type: HYDROGEN,
            other_type: Some(OXYGEN),
            rule: Box::new(Rule::NeighborCustom(r#"
                if neighbor_dist < 0.15 && neighbor_dist > 0.02 {
                    let attract = 0.8 / (neighbor_dist * neighbor_dist + 0.01);
                    p.velocity += neighbor_dir * attract * uniforms.delta_time;
                }
            "#.into())),
        })

        // Oxygen attracts Hydrogen (mutual)
        .with_rule(Rule::Typed {
            self_type: OXYGEN,
            other_type: Some(HYDROGEN),
            rule: Box::new(Rule::NeighborCustom(r#"
                if neighbor_dist < 0.15 && neighbor_dist > 0.02 {
                    let attract = 0.3 / (neighbor_dist * neighbor_dist + 0.01);
                    p.velocity += neighbor_dir * attract * uniforms.delta_time;
                }
            "#.into())),
        })

        // Nitrogen clusters with itself (N2)
        .with_rule(Rule::Typed {
            self_type: NITROGEN,
            other_type: Some(NITROGEN),
            rule: Box::new(Rule::Cohere { radius: 0.12, strength: 1.5 }),
        })

        // Carbon attracts everything weakly
        .with_rule(Rule::Typed {
            self_type: CARBON,
            other_type: Some(HYDROGEN),
            rule: Box::new(Rule::Cohere { radius: 0.1, strength: 0.8 }),
        })
        .with_rule(Rule::Typed {
            self_type: CARBON,
            other_type: Some(OXYGEN),
            rule: Box::new(Rule::Cohere { radius: 0.1, strength: 0.8 }),
        })
        .with_rule(Rule::Typed {
            self_type: CARBON,
            other_type: Some(NITROGEN),
            rule: Box::new(Rule::Cohere { radius: 0.1, strength: 0.5 }),
        })

        // Water molecules attract each other (surface tension)
        .with_rule(Rule::Typed {
            self_type: WATER,
            other_type: Some(WATER),
            rule: Box::new(Rule::Cohere { radius: 0.15, strength: 2.0 }),
        })

        // === UNIVERSAL FORCES ===

        // Everything repels at very close range (electron shells)
        .with_rule(Rule::Separate { radius: 0.025, strength: 3.0 })

        // Charge-based attraction/repulsion
        .with_rule(Rule::NeighborCustom(r#"
            if neighbor_dist < 0.12 && neighbor_dist > 0.01 {
                let force = -p.charge * other.charge / (neighbor_dist * neighbor_dist + 0.001);
                p.velocity += neighbor_dir * force * 0.5 * uniforms.delta_time;
            }
        "#.into()))

        // === ENERGY / TEMPERATURE ===

        // Energy affects movement (temperature)
        .with_rule(Rule::Wander {
            strength: 0.3,
            frequency: 3.0,
        })

        // Reaction products get energy boost (exothermic)
        .with_rule(Rule::Custom(r#"
            if p.particle_type == 4u { // Water
                // Newly formed water gets a speed boost
                if p.bonds < 0.5 {
                    p.velocity *= 1.5;
                    p.bonds = 1.0;
                    p.energy = 1.0;
                }
            }
        "#.into()))

        // Energy slowly equalizes (heat dissipation)
        .with_rule(Rule::Custom(r#"
            p.energy = mix(p.energy, 0.5, 0.1 * uniforms.delta_time);
        "#.into()))

        // === COLORING ===

        .with_rule(Rule::Custom(r#"
            // Base colors by type
            if p.particle_type == 0u { // Hydrogen
                p.color = vec3<f32>(1.0, 1.0, 0.7);
            } else if p.particle_type == 1u { // Oxygen
                p.color = vec3<f32>(1.0, 0.25, 0.2);
            } else if p.particle_type == 2u { // Nitrogen
                p.color = vec3<f32>(0.3, 0.5, 1.0);
            } else if p.particle_type == 3u { // Carbon
                p.color = vec3<f32>(0.5, 0.5, 0.5);
            } else if p.particle_type == 4u { // Water
                p.color = vec3<f32>(0.2, 0.9, 1.0);
            }

            // Energy adds glow
            p.color *= (0.6 + p.energy * 0.6);

            // Speed adds brightness
            let speed = length(p.velocity);
            p.color += vec3<f32>(speed * 0.3);
        "#.into()))

        // === PHYSICS ===

        // Mass affects movement (heavier = slower response)
        .with_rule(Rule::Custom(r#"
            // Scale velocity changes by inverse mass
            let inv_mass = 1.0 / max(p.mass, 0.1);
            // Lighter atoms are faster
            p.velocity *= mix(1.0, inv_mass * 4.0, 0.02);
        "#.into()))

        .with_rule(Rule::Drag(1.2))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 1.2 })
        .with_rule(Rule::BounceWalls)

        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.01, 0.01, 0.02));
        })
        .run();
}
