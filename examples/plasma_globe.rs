//! # Plasma Globe Simulation
//!
//! Electric arcs reaching from a central electrode toward the glass sphere,
//! with particles streaming along ionized channels and interactive touch response.
//!
//! ## What This Demonstrates
//!
//! - Electric arc formation with multiple tendrils
//! - Particles flowing along curved ionization paths
//! - Mouse interaction: arcs attracted to cursor (simulating touch)
//! - Field-based glow effects
//! - Beautiful plasma color palette (purple/blue/white)
//!
//! ## Physics
//!
//! - Central electrode emits charged particles
//! - Particles follow curved paths toward sphere surface
//! - Multiple arc channels form and slowly wander
//! - Touch (mouse) attracts nearest arcs
//! - Particles respawn at center when reaching boundary
//!
//! ## Controls
//!
//! - Move mouse to attract plasma arcs (like touching the glass)
//! - Left-click to create a strong attraction point
//!
//! Run with: `cargo run --example plasma_globe --release`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct PlasmaParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Which arc channel this particle belongs to (0-7)
    arc_id: u32,
    /// Progress along the arc (0 = center, 1 = surface)
    arc_progress: f32,
    /// Intensity/brightness of this particle
    intensity: f32,
    /// Random offset for variation within the arc
    offset_seed: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    let num_particles = 30_000;
    let num_arcs = 6; // Number of arc channels

    // Pre-generate particles distributed along arcs
    let particles: Vec<PlasmaParticle> = (0..num_particles)
        .map(|i| {
            let arc_id = (i % num_arcs) as u32;
            let arc_progress = rng.gen_range(0.0_f32..1.0);
            let offset_seed = rng.gen_range(0.0_f32..1.0);

            PlasmaParticle {
                position: Vec3::ZERO, // Will be set by shader
                velocity: Vec3::ZERO,
                color: Vec3::new(0.5, 0.3, 1.0),
                arc_id,
                arc_progress,
                intensity: rng.gen_range(0.5..1.0),
                offset_seed,
            }
        })
        .collect();

    Simulation::<PlasmaParticle>::new()
        .with_particle_count(num_particles as u32)
        .with_particle_size(0.008)
        .with_bounds(1.2)
        .with_spawner(move |ctx| particles[ctx.index as usize].clone())
        // Plasma glow field
        .with_field(
            "plasma",
            FieldConfig::new(48)
                .with_extent(1.2)
                .with_decay(0.85)
                .with_blur(0.3)
                .with_blur_iterations(1),
        )
        // Volume render the plasma glow
        .with_volume_render(
            VolumeConfig::new()
                .with_field(0)
                .with_steps(32)
                .with_density_scale(5.0)
                .with_palette(Palette::Viridis)
                .with_threshold(0.02)
                .with_additive(true),
        )
        // Uniforms
        .with_uniform::<f32>("mouse_x", 0.0)
        .with_uniform::<f32>("mouse_y", 0.0)
        .with_uniform::<f32>("mouse_z", 0.0)
        .with_uniform::<f32>("mouse_active", 0.0)
        .with_uniform::<f32>("sphere_radius", 0.85)
        .with_uniform::<f32>("arc_speed", 1.5)
        .with_uniform::<f32>("arc_chaos", 0.3)
        // Update mouse position
        .with_update(|ctx| {
            let mouse = ctx.input.mouse_ndc();

            // Project mouse to 3D position on front of sphere
            let mouse_x = mouse.x * 0.8;
            let mouse_y = mouse.y * 0.8;
            // Calculate Z to put point on sphere surface
            let r_sq = mouse_x * mouse_x + mouse_y * mouse_y;
            let mouse_z = if r_sq < 0.64 {
                (0.64 - r_sq).sqrt() // On sphere surface
            } else {
                0.0
            };

            ctx.set("mouse_x", mouse_x);
            ctx.set("mouse_y", mouse_y);
            ctx.set("mouse_z", mouse_z);

            if ctx.input.mouse_held(MouseButton::Left) {
                ctx.set("mouse_active", 1.0);
            } else {
                ctx.set("mouse_active", 0.7); // Strong passive attraction (like touching glass)
            }
        })
        // Main plasma dynamics
        .with_rule(Rule::Custom(
            r#"
            let sphere_radius = uniforms.sphere_radius;
            let arc_speed = uniforms.arc_speed;
            let chaos = uniforms.arc_chaos;
            let time = uniforms.time;

            // === ARC TARGET CALCULATION ===
            // Each arc has a target point on the sphere that slowly wanders
            let arc_f = f32(p.arc_id);
            let arc_count = 6.0;

            // Base angle for this arc (evenly distributed)
            let base_theta = (arc_f / arc_count) * 6.283185;
            let base_phi = 1.0; // Start near equator

            // Wandering motion for arc targets
            let wander_speed = 0.3;
            let theta = base_theta + sin(time * wander_speed + arc_f * 1.5) * 0.8;
            let phi = base_phi + cos(time * wander_speed * 0.7 + arc_f * 2.1) * 0.4;

            // Arc target on sphere surface
            var arc_target = vec3<f32>(
                sin(phi) * cos(theta),
                cos(phi),
                sin(phi) * sin(theta)
            ) * sphere_radius;

            // === MOUSE INTERACTION ===
            // Mouse attracts nearby arcs strongly
            let mouse_pos = vec3<f32>(uniforms.mouse_x, uniforms.mouse_y, uniforms.mouse_z);
            let mouse_strength = uniforms.mouse_active;
            let mouse_len = length(mouse_pos);

            // Project mouse to sphere surface for target
            var mouse_target = mouse_pos;
            if mouse_len > 0.01 {
                mouse_target = normalize(mouse_pos) * sphere_radius;
            }

            // All arcs are attracted to mouse, strength based on proximity
            let arc_to_mouse = length(arc_target - mouse_target);
            let attraction_radius = 1.5; // Large radius - affects most arcs

            if mouse_strength > 0.1 {
                // Closer arcs get pulled more strongly
                let proximity = 1.0 - clamp(arc_to_mouse / attraction_radius, 0.0, 1.0);
                let blend = proximity * proximity * mouse_strength * 0.9; // Quadratic falloff, strong pull
                arc_target = mix(arc_target, mouse_target, blend);
            }

            // === PARTICLE POSITION ALONG ARC ===
            // Particles flow from center toward arc_target
            let progress = p.arc_progress;

            // Curved path with some chaos
            let center = vec3<f32>(0.0, 0.0, 0.0);
            let direction = normalize(arc_target);

            // Add sinusoidal deviation for curved arc shape
            let perp1 = normalize(cross(direction, vec3<f32>(0.0, 1.0, 0.0)));
            let perp2 = cross(direction, perp1);

            // Spiral/curve parameters
            let curl_freq = 3.0 + p.offset_seed * 2.0;
            let curl_amp = 0.08 * (1.0 - progress) * chaos;

            let curl_offset = perp1 * sin(progress * curl_freq * 6.283 + time * 2.0 + p.offset_seed * 6.28) * curl_amp
                            + perp2 * cos(progress * curl_freq * 6.283 + time * 2.0 + p.offset_seed * 6.28) * curl_amp;

            // Arc path: starts narrow at center, spreads at edges
            let spread = progress * 0.03;
            let noise_offset = perp1 * sin(p.offset_seed * 100.0) * spread
                             + perp2 * cos(p.offset_seed * 100.0) * spread;

            // Final position along arc
            let arc_pos = center + direction * (progress * sphere_radius) + curl_offset + noise_offset;
            p.position = arc_pos;

            // === ARC PROGRESS (flowing toward surface) ===
            p.arc_progress += uniforms.delta_time * arc_speed * (0.8 + p.offset_seed * 0.4);

            // Respawn at center when reaching surface
            if p.arc_progress > 1.0 {
                p.arc_progress = 0.0;
                p.intensity = 0.7 + fract(p.offset_seed * 7.13) * 0.3;
            }

            // === INTENSITY PULSING ===
            // Arcs flicker and pulse
            let flicker = 0.7 + 0.3 * sin(time * 15.0 + arc_f * 3.0 + progress * 10.0);
            let core_brightness = 1.0 - abs(p.offset_seed - 0.5) * 1.5; // Brighter at arc center
            let current_intensity = p.intensity * flicker * core_brightness;

            // === EMIT TO FIELD ===
            if current_intensity > 0.3 {
                field_write(0u, p.position, current_intensity * 0.4);
            }

            // === COLOR ===
            // Plasma colors: purple core -> blue mid -> white/pink tips
            var base_color: vec3<f32>;

            if progress < 0.3 {
                // Core: bright purple-white
                let t = progress / 0.3;
                base_color = mix(
                    vec3<f32>(0.9, 0.8, 1.0),   // White-purple core
                    vec3<f32>(0.6, 0.3, 1.0),   // Purple
                    t
                );
            } else if progress < 0.7 {
                // Middle: purple to blue
                let t = (progress - 0.3) / 0.4;
                base_color = mix(
                    vec3<f32>(0.6, 0.3, 1.0),   // Purple
                    vec3<f32>(0.3, 0.4, 1.0),   // Blue
                    t
                );
            } else {
                // Tips: blue to cyan-white
                let t = (progress - 0.7) / 0.3;
                base_color = mix(
                    vec3<f32>(0.3, 0.4, 1.0),   // Blue
                    vec3<f32>(0.7, 0.85, 1.0),  // Cyan-white tips
                    t
                );
            }

            // Apply intensity and flicker
            p.color = base_color * current_intensity;

            // Brighten based on mouse proximity (touch glow)
            if mouse_strength > 0.5 {
                let dist_to_mouse = length(p.position - mouse_pos);
                if dist_to_mouse < 0.3 {
                    let glow = (0.3 - dist_to_mouse) / 0.3 * 0.5;
                    p.color += vec3<f32>(glow, glow * 0.8, glow * 0.5);
                }
            }

            // Scale by intensity
            p.scale = 0.5 + current_intensity * 0.8;
            "#
            .into(),
        ))
        .with_rule(Rule::Drag(5.0))
        .with_visuals(|v| {
            v.background(Vec3::new(0.01, 0.005, 0.02)); // Very dark purple-black
            v.blend_mode(BlendMode::Additive);
            v.shape(ParticleShape::Circle);
        })
        .run();
}
