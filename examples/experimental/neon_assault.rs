//! # Neon Assault
//!
//! Aggressive 80s arcade aesthetic - Geometry Wars meets Tron meets fever dream.
//! Chaotic neon particles on a CRT grid with scanlines and chromatic explosion.
//!
//! Run with: `cargo run --example neon_assault`

use rand::Rng;
use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct GridEntity {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    /// Entity type: 0=swarm, 1=hunter, 2=runner
    entity_type: f32,
    /// Aggression/energy level
    intensity: f32,
}

fn main() {
    let mut rng = rand::thread_rng();

    // Neon color palette - pure 80s
    let neon_colors = [
        Vec3::new(1.0, 0.0, 0.5),   // Hot pink
        Vec3::new(0.0, 1.0, 1.0),   // Cyan
        Vec3::new(1.0, 1.0, 0.0),   // Electric yellow
        Vec3::new(0.5, 0.0, 1.0),   // Purple
        Vec3::new(0.0, 1.0, 0.3),   // Toxic green
        Vec3::new(1.0, 0.3, 0.0),   // Orange
    ];

    let particles: Vec<_> = (0..3500)
        .map(|i| {
            // Spawn in geometric patterns - rings, crosses, grids
            let pattern = i % 5;
            let pos = match pattern {
                0 => {
                    // Ring spawn
                    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                    let r = rng.gen_range(0.3..0.7);
                    Vec3::new(angle.cos() * r, rng.gen_range(-0.3..0.3), angle.sin() * r)
                }
                1 => {
                    // Grid spawn
                    let x = (rng.gen_range(0..20) as f32 - 10.0) * 0.08;
                    let z = (rng.gen_range(0..20) as f32 - 10.0) * 0.08;
                    Vec3::new(x, rng.gen_range(-0.2..0.2), z)
                }
                2 => {
                    // Cross pattern
                    if rng.gen_bool(0.5) {
                        Vec3::new(rng.gen_range(-0.8..0.8), rng.gen_range(-0.1..0.1), rng.gen_range(-0.05..0.05))
                    } else {
                        Vec3::new(rng.gen_range(-0.05..0.05), rng.gen_range(-0.1..0.1), rng.gen_range(-0.8..0.8))
                    }
                }
                3 => {
                    // Cluster bombs
                    let center = Vec3::new(
                        rng.gen_range(-0.5..0.5),
                        0.0,
                        rng.gen_range(-0.5..0.5),
                    );
                    center + Vec3::new(
                        rng.gen_range(-0.1..0.1),
                        rng.gen_range(-0.1..0.1),
                        rng.gen_range(-0.1..0.1),
                    )
                }
                _ => {
                    // Random chaos
                    Vec3::new(
                        rng.gen_range(-0.8..0.8),
                        rng.gen_range(-0.4..0.4),
                        rng.gen_range(-0.8..0.8),
                    )
                }
            };

            let color = neon_colors[rng.gen_range(0..neon_colors.len())];
            let entity_type = rng.gen_range(0.0_f32..3.0).floor();
            let intensity = rng.gen_range(0.7..1.5);

            // Initial velocity based on type
            let vel = match entity_type as i32 {
                0 => Vec3::new(rng.gen_range(-0.5..0.5), 0.0, rng.gen_range(-0.5..0.5)), // Swarm
                1 => Vec3::ZERO, // Hunters start still
                _ => Vec3::new(rng.gen_range(-1.0..1.0), 0.0, rng.gen_range(-1.0..1.0)), // Runners
            };

            (pos, vel, color, entity_type, intensity)
        })
        .collect();

    Simulation::<GridEntity>::new()
        .with_particle_count(3500)
        .with_bounds(2.0)
        .with_particle_size(0.018)
        .with_spawner(move |i, _| {
            let (pos, vel, color, entity_type, intensity) = particles[i as usize];
            GridEntity {
                position: pos,
                velocity: vel,
                color,
                entity_type,
                intensity,
            }
        })
        // Hard-edged neon glow - sharp cores with electric halos
        .with_fragment_shader(r#"
            let dist = length(in.uv);

            // Sharp geometric core
            let core = 1.0 - smoothstep(0.0, 0.3, dist);

            // Electric halo with interference pattern
            let halo = sin(dist * 30.0 - uniforms.time * 10.0) * 0.5 + 0.5;
            let halo_fade = exp(-dist * 3.0);
            let electric = halo * halo_fade * 0.5;

            // Outer glow
            let glow = 1.0 / (dist * dist * 10.0 + 0.2);

            // Pulsing based on time
            let pulse = sin(uniforms.time * 8.0 + length(in.color) * 20.0) * 0.3 + 1.0;

            let intensity = (core * 2.0 + electric + glow * 0.3) * pulse;

            // Color boost - oversaturate for that neon look
            let boosted_color = in.color * 1.5;

            let alpha = clamp(intensity * 0.6, 0.0, 1.0);
            return vec4<f32>(boosted_color * intensity, alpha);
        "#)
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.0, 0.0, 0.02)); // Near black with hint of blue
            v.trails(5);
            v.connections(0.08); // Tight grid connections

            // CRT arcade post-processing
            v.post_process(r#"
                // CRT barrel distortion
                let center = vec2<f32>(0.5, 0.5);
                var uv = in.uv;
                let uv_centered = uv - center;
                let dist_sq = dot(uv_centered, uv_centered);
                let barrel = 0.1;
                uv = center + uv_centered * (1.0 + barrel * dist_sq);

                // Aggressive chromatic aberration
                let aberr = 0.006;
                let r = textureSample(scene, scene_sampler, uv + vec2<f32>(aberr, 0.0)).r;
                let g = textureSample(scene, scene_sampler, uv).g;
                let b = textureSample(scene, scene_sampler, uv - vec2<f32>(aberr, 0.0)).b;
                var color = vec3<f32>(r, g, b);

                // SCANLINES - the essential CRT look
                let scanline_freq = 400.0;
                let scanline = sin(in.uv.y * scanline_freq) * 0.5 + 0.5;
                let scanline_intensity = 0.15;
                color *= 1.0 - scanline_intensity * (1.0 - scanline);

                // Horizontal noise bands (interference)
                let noise_y = floor(in.uv.y * 100.0 + uniforms.time * 50.0);
                let interference = fract(sin(noise_y * 12.9898) * 43758.5453);
                if interference > 0.97 {
                    color *= 1.3; // Bright interference line
                }

                // RGB pixel separation (like old monitors)
                let pixel_x = floor(in.uv.x * 800.0);
                let subpixel = pixel_x % 3.0;
                if subpixel < 1.0 {
                    color.g *= 0.9;
                    color.b *= 0.8;
                } else if subpixel < 2.0 {
                    color.r *= 0.9;
                    color.b *= 0.8;
                } else {
                    color.r *= 0.8;
                    color.g *= 0.9;
                }

                // Bloom - bright areas bleed
                let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
                let bloom = smoothstep(0.4, 1.0, luminance) * 0.4;
                color += color * bloom;

                // Vignette - darker corners
                let vignette_dist = length(in.uv - center);
                let vignette = 1.0 - smoothstep(0.4, 1.0, vignette_dist);
                color *= vignette;

                // Color boost - pump up saturation
                let gray = dot(color, vec3<f32>(0.3, 0.3, 0.3));
                color = mix(vec3<f32>(gray), color, 1.4);

                // Slight flicker
                let flicker = sin(uniforms.time * 60.0) * 0.02 + 1.0;
                color *= flicker;

                // Clamp and output
                color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.5)); // Allow slight HDR

                return vec4<f32>(color, 1.0);
            "#);
        })
        // Chaotic aggressive movement
        .with_rule(Rule::Custom(r#"
            let pos = p.position;
            let entity = floor(p.entity_type);

            // Base chaos - everything is always moving
            let chaos_freq = 5.0;
            let chaos = vec3<f32>(
                sin(pos.z * chaos_freq + uniforms.time * 3.0),
                sin(pos.x * chaos_freq + uniforms.time * 2.5) * 0.3,
                cos(pos.x * chaos_freq + uniforms.time * 3.5)
            );
            p.velocity += chaos * 0.15;

            // Type-specific behavior
            if entity == 0.0 {
                // SWARM - orbit aggressively around center
                let to_center = -normalize(pos);
                let tangent = cross(to_center, vec3<f32>(0.0, 1.0, 0.0));
                p.velocity += tangent * 0.4;
                // Occasionally dive toward center
                let dive = step(0.98, fract(sin(uniforms.time * 2.0 + p.intensity * 100.0) * 0.5 + 0.5));
                p.velocity += to_center * dive * 0.5;
            } else if entity == 1.0 {
                // HUNTERS - aggressive pursuit toward origin with sudden lunges
                let to_origin = -pos;
                let dist = length(to_origin);
                let hunt_dir = normalize(to_origin);
                // Lunge mechanic
                let lunge = step(0.95, fract(sin(uniforms.time * 5.0 + p.intensity * 50.0) * 0.5 + 0.5));
                p.velocity += hunt_dir * (0.2 + lunge * 1.5);
                // Orbit when close
                if dist < 0.3 {
                    let orbit = cross(hunt_dir, vec3<f32>(0.0, 1.0, 0.0));
                    p.velocity += orbit * 0.8;
                }
            } else {
                // RUNNERS - flee from center erratically
                let away = normalize(pos + vec3<f32>(0.001, 0.0, 0.0));
                p.velocity += away * 0.3;
                // Random direction changes
                let jitter = vec3<f32>(
                    sin(uniforms.time * 10.0 + p.intensity * 100.0),
                    0.0,
                    cos(uniforms.time * 12.0 + p.intensity * 80.0)
                );
                p.velocity += jitter * 0.2;
            }

            // Bounce off invisible walls aggressively
            let bounds = 0.9;
            if abs(pos.x) > bounds {
                p.velocity.x = -p.velocity.x * 1.2;
            }
            if abs(pos.z) > bounds {
                p.velocity.z = -p.velocity.z * 1.2;
            }
            if abs(pos.y) > 0.5 {
                p.velocity.y = -p.velocity.y * 0.8;
            }

            // Speed limit with high ceiling
            let speed = length(p.velocity);
            if speed > 2.0 {
                p.velocity = normalize(p.velocity) * 2.0;
            }

            // Light damping to prevent total chaos
            p.velocity *= 0.97;
        "#.into()))
        .run();
}
