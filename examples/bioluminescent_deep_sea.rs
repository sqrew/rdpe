//! # Bioluminescent Deep Sea
//!
//! A mysterious underwater scene where particles glow when disturbed,
//! and light ripples through the water as nearby organisms respond.
//!
//! ## What This Demonstrates
//!
//! - `Rule::Refractory` - charge depletion/regeneration (luciferin mechanic)
//! - `Rule::Diffuse` - light spreading to neighbors
//! - `mouse_world_pos()` - easy mouse-to-world coordinate mapping
//! - Custom fragment shader for soft bioluminescent glow
//! - Atmospheric post-processing (vignette, aberration)
//!
//! ## Controls
//!
//! - **Move mouse**: Disturb the water, triggering bioluminescence
//! - **Left click (hold)**: Create a stronger disturbance
//!
//! ## The Science
//!
//! Many deep-sea organisms produce light through bioluminescence using
//! a chemical called luciferin. When disturbed, they flash - but this
//! depletes their luciferin supply, requiring time to regenerate before
//! they can flash again. This creates beautiful waves of light that
//! sweep through and naturally fade, rather than staying lit forever.
//!
//! Run with: `cargo run --example bioluminescent_deep_sea`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct Plankton {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Current glow intensity (0.0 = dark, 1.0 = bright)
    glow: f32,
    /// Chemical charge available for glowing (0.0 = depleted, 1.0 = full)
    charge: f32,
    /// Size variation for organic feel
    size: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Create particles scattered throughout the volume
    let particles: Vec<Plankton> = (0..15_000)
        .map(|_| {
            let pos = Vec3::new(
                rng.gen_range(-1.5..1.5),
                rng.gen_range(-0.8..0.8),
                rng.gen_range(-1.5..1.5),
            );
            let vel = Vec3::new(
                rng.gen_range(-0.02..0.02),
                rng.gen_range(-0.01..0.01),
                rng.gen_range(-0.02..0.02),
            );

            Plankton {
                position: pos,
                velocity: vel,
                color: Vec3::new(0.02, 0.04, 0.08), // Start dark
                glow: 0.0,
                charge: 1.0, // Start fully charged
                size: rng.gen_range(0.6..1.4),
            }
        })
        .collect();

    Simulation::<Plankton>::new()
        .with_particle_count(15_000)
        .with_bounds(2.0)
        .with_particle_size(0.012)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())
        .with_spatial_config(0.15, 32)
        // Track mouse position
        .with_uniform("mouse_pos", Vec3::ZERO)
        .with_uniform("disturb_strength", 0.0f32)
        .with_update(|ctx| {
            ctx.set("mouse_pos", ctx.mouse_world_pos());
            let strength = if ctx.input.mouse_held(MouseButton::Left) {
                1.0f32
            } else {
                0.3f32
            };
            ctx.set("disturb_strength", strength);
        })
        // === Movement ===
        .with_rule(Rule::Turbulence {
            scale: 1.5,
            strength: 0.08,
        })
        .with_rule(Rule::Wander {
            strength: 0.15,
            frequency: 50.0,
        })
        .with_rule(Rule::Acceleration(Vec3::new(0.0, 0.02, 0.0)))
        .with_rule(Rule::Drag(3.0))
        .with_rule(Rule::SpeedLimit { min: 0.0, max: 0.3 })
        // === Bioluminescence: Mouse Trigger ===
        // Only triggers if particle has enough charge
        .with_rule(Rule::Custom(
            r#"
            let to_mouse = uniforms.mouse_pos - p.position;
            let mouse_dist = length(to_mouse);
            let trigger_radius = 0.4;

            if mouse_dist < trigger_radius && p.charge > 0.3 {
                let trigger_strength = (1.0 - mouse_dist / trigger_radius) * uniforms.disturb_strength;
                let flash = trigger_strength * 0.8 * p.charge;
                p.glow = max(p.glow, flash);
            }
            "#
            .into(),
        ))
        // === Bioluminescence: Light Spreading ===
        // Using Rule::Diffuse for natural light propagation to neighbors
        .with_rule(Rule::Diffuse {
            field: "glow".into(),
            radius: 0.12,
            rate: 1.5,
        })
        // === Bioluminescence: Charge System ===
        // Rule::Refractory handles the luciferin depletion/regeneration!
        .with_rule(Rule::Refractory {
            trigger: "glow".into(),
            charge: "charge".into(),
            active_threshold: 0.05,
            depletion_rate: 0.03,
            regen_rate: 0.008,
        })
        // === Glow Decay & Color Mapping ===
        .with_rule(Rule::Custom(
            r#"
            // Natural glow decay
            p.glow = p.glow * 0.92;
            if p.glow < 0.02 {
                p.glow = 0.0;
            }

            // Color: dark blue-black → cyan → bright teal
            let dark_color = vec3<f32>(0.01, 0.02, 0.05);
            let glow_color = vec3<f32>(0.1, 0.8, 0.9);
            let bright_color = vec3<f32>(0.3, 1.0, 0.95);

            if p.glow < 0.5 {
                p.color = mix(dark_color, glow_color, p.glow * 2.0);
            } else {
                p.color = mix(glow_color, bright_color, (p.glow - 0.5) * 2.0);
            }
            "#
            .into(),
        ))
        .with_rule(Rule::WrapWalls)
        // === Visuals ===
        .with_fragment_shader(
            r#"
            let dist = length(in.uv);
            let glow = 1.0 / (dist * dist * 8.0 + 0.3);
            let core = smoothstep(0.5, 0.0, dist) * 2.0;
            let intensity = (glow + core) * in.color.x * 3.0 + glow * 0.5;
            let color = in.color * intensity;
            let alpha = clamp(intensity * 0.6, 0.0, 1.0);
            return vec4<f32>(color, alpha);
            "#,
        )
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.005, 0.008, 0.02));
            v.post_process(
                r#"
                let aberration = 0.002;
                let uv_r = in.uv + vec2<f32>(aberration, aberration * 0.5);
                let uv_b = in.uv - vec2<f32>(aberration, aberration * 0.5);

                let r = textureSample(scene, scene_sampler, uv_r).r;
                let g = textureSample(scene, scene_sampler, in.uv).g;
                let b = textureSample(scene, scene_sampler, uv_b).b;
                var color = vec3<f32>(r, g, b);

                let vignette_dist = length(in.uv - vec2<f32>(0.5, 0.5));
                let vignette = 1.0 - smoothstep(0.2, 0.85, vignette_dist);
                color *= vignette;
                color = color * vec3<f32>(0.9, 0.95, 1.1);
                color = clamp((color - 0.5) * 1.1 + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));

                return vec4<f32>(color, 1.0);
                "#,
            );
        })
        .run().expect("Simulation failed");
}
