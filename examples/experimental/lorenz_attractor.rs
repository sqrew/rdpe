//! Lorenz Strange Attractor visualization.
//!
//! Particles trace out the famous butterfly-shaped Lorenz attractor,
//! a chaotic system discovered by Edward Lorenz in 1963 while studying
//! atmospheric convection.
//!
//! The attractor is defined by three coupled differential equations:
//!   dx/dt = σ(y - x)
//!   dy/dt = x(ρ - z) - y
//!   dz/dt = xy - βz
//!
//! With classic parameters σ=10, ρ=28, β=8/3, the system exhibits
//! deterministic chaos - nearby trajectories diverge exponentially
//! while remaining bounded within the attractor's shape.

use rdpe::prelude::*;

#[derive(Particle, Clone)]
struct LorenzParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

fn main() {
    // Lorenz attractor parameters (classic values)
    let sigma: f32 = 10.0;
    let rho: f32 = 28.0;
    let beta: f32 = 8.0 / 3.0;

    // Scale factor to fit the attractor nicely in view
    // The attractor spans roughly x,y: [-20,20], z: [0,50]
    let scale = 0.04;

    Simulation::<LorenzParticle>::new()
        .with_particle_count(20_000)
        .with_bounds(3.0)
        // Spawn particles in a small cloud near one of the attractor's lobes
        .with_spawner(move |i, count| {
            // Spread initial positions in a small region
            let t = i as f32 / count as f32;
            let angle = t * std::f32::consts::TAU * 100.0;
            let r = 0.5 + (i % 100) as f32 * 0.01;

            // Start near the attractor (offset from origin)
            let x = 1.0 + r * angle.cos() * 0.3;
            let y = 1.0 + r * angle.sin() * 0.3;
            let z = 25.0 + (i % 50) as f32 * 0.1;

            // Color based on initial position - creates gradient trails
            let hue = t * 2.0;
            let color = hue_to_rgb(hue);

            LorenzParticle {
                position: Vec3::new(x * scale, (z - 25.0) * scale, y * scale),
                velocity: Vec3::ZERO,
                color,
            }
        })
        // The Lorenz system dynamics as a custom rule
        .with_rule(Rule::Custom(format!(
            r#"
    // Lorenz attractor dynamics
    // Unscale position to get actual Lorenz coordinates
    let scale = {scale:.6};
    let lx = p.position.x / scale;
    let ly = p.position.z / scale;  // swap y/z for better 3D view
    let lz = p.position.y / scale + 25.0;  // offset z

    // Lorenz equations
    let sigma = {sigma:.6};
    let rho = {rho:.6};
    let beta = {beta:.6};

    let dx = sigma * (ly - lx);
    let dy = lx * (rho - lz) - ly;
    let dz = lx * ly - beta * lz;

    // Apply as velocity (scaled back down)
    let speed = 0.15;
    p.velocity = vec3<f32>(dx, dz, dy) * scale * speed;

    // Color based on position - creates flowing gradients through the attractor
    // Use position to create a smooth color gradient
    let wing = lx / 20.0;  // Which side of attractor (-1 to 1 roughly)
    let h = (lz - 10.0) / 40.0;  // Height in attractor (0 to 1 roughly)

    // Hue shifts based on position - blue/cyan on left, orange/red on right
    let hue = (wing + 1.0) * 0.5;  // 0 to 1

    // Convert hue to RGB (simplified)
    let r = clamp(abs(hue * 6.0 - 3.0) - 1.0, 0.0, 1.0);
    let g = clamp(2.0 - abs(hue * 6.0 - 2.0), 0.0, 1.0);
    let b = clamp(2.0 - abs(hue * 6.0 - 4.0), 0.0, 1.0);

    // Boost saturation and add height-based brightness
    let brightness = 0.7 + h * 0.3;
    p.color = vec3<f32>(r, g, b) * brightness;
"#,
            scale = scale,
            sigma = sigma,
            rho = rho,
            beta = beta,
        )))
        // Slow drag to smooth out motion
        .with_rule(Rule::Drag(0.1))
        // Visual settings
        .with_visuals(|v| {
            v.blend_mode(BlendMode::Additive);
            v.background(Vec3::new(0.01, 0.01, 0.02));
            v.trails(100); // Long fading trails
        })
        .with_particle_size(0.008)
        .run();
}

/// Convert hue (0-1) to RGB color
fn hue_to_rgb(h: f32) -> Vec3 {
    let h = h.fract();
    let r = (h * 6.0 - 3.0).abs().clamp(0.0, 1.0);
    let g = (2.0 - (h * 6.0 - 2.0).abs()).clamp(0.0, 1.0);
    let b = (2.0 - (h * 6.0 - 4.0).abs()).clamp(0.0, 1.0);
    Vec3::new(r, g, b)
}
