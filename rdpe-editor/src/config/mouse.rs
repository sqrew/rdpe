//! Mouse interaction configuration

use serde::{Deserialize, Serialize};

/// Mouse interaction power types
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum MousePower {
    /// No mouse interaction
    #[default]
    None,
    /// Pull particles toward cursor
    Attract,
    /// Push particles away from cursor
    Repel,
    /// Swirl particles around cursor
    Vortex,
    /// Burst particles outward on click
    Explode,
    /// Strong point gravity at cursor
    GravityWell,
    /// Color particles near cursor
    Paint,
    /// Add chaos/noise near cursor
    Turbulence,
    /// Slow particles in radius
    Freeze,
    /// Destroy particles on click
    Kill,
    /// Spawn particles at cursor
    Spawn,
    /// Suck particles in and destroy at center
    BlackHole,
    /// Make particles orbit around cursor
    Orbit,
    /// Random velocity impulse
    Scatter,
    /// Directional push force
    Wind,
    /// Rhythmic expanding wave
    Pulse,
    /// Ring-shaped outward push
    Repulsor,
    /// Spiral inward like a drain
    SpiralIn,
    /// Randomize particle velocities
    RandomVelocity,
}

impl MousePower {
    pub fn variants() -> &'static [&'static str] {
        &[
            "None", "Attract", "Repel", "Vortex", "Explode", "GravityWell",
            "Paint", "Turbulence", "Freeze", "Kill", "Spawn", "BlackHole",
            "Orbit", "Scatter", "Wind", "Pulse", "Repulsor", "SpiralIn", "RandomVelocity",
        ]
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => MousePower::None,
            1 => MousePower::Attract,
            2 => MousePower::Repel,
            3 => MousePower::Vortex,
            4 => MousePower::Explode,
            5 => MousePower::GravityWell,
            6 => MousePower::Paint,
            7 => MousePower::Turbulence,
            8 => MousePower::Freeze,
            9 => MousePower::Kill,
            10 => MousePower::Spawn,
            11 => MousePower::BlackHole,
            12 => MousePower::Orbit,
            13 => MousePower::Scatter,
            14 => MousePower::Wind,
            15 => MousePower::Pulse,
            16 => MousePower::Repulsor,
            17 => MousePower::SpiralIn,
            18 => MousePower::RandomVelocity,
            _ => MousePower::None,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            MousePower::None => 0,
            MousePower::Attract => 1,
            MousePower::Repel => 2,
            MousePower::Vortex => 3,
            MousePower::Explode => 4,
            MousePower::GravityWell => 5,
            MousePower::Paint => 6,
            MousePower::Turbulence => 7,
            MousePower::Freeze => 8,
            MousePower::Kill => 9,
            MousePower::Spawn => 10,
            MousePower::BlackHole => 11,
            MousePower::Orbit => 12,
            MousePower::Scatter => 13,
            MousePower::Wind => 14,
            MousePower::Pulse => 15,
            MousePower::Repulsor => 16,
            MousePower::SpiralIn => 17,
            MousePower::RandomVelocity => 18,
        }
    }

    /// Generate WGSL code for this mouse power
    pub fn to_wgsl(&self) -> String {
        match self {
            MousePower::None => String::new(),
            MousePower::Attract => r#"
    // Mouse Attract
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist > 0.001 && dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            p.velocity += normalize(to_mouse) * mstrength * delta_time;
        }
    }
"#.into(),
            MousePower::Repel => r#"
    // Mouse Repel
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist > 0.001 && dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            p.velocity -= normalize(to_mouse) * mstrength * delta_time;
        }
    }
"#.into(),
            MousePower::Vortex => r#"
    // Mouse Vortex
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist > 0.001 && dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            // Perpendicular force for swirl (in XZ plane)
            let tangent = vec3<f32>(-to_mouse.z, 0.0, to_mouse.x);
            p.velocity += normalize(tangent) * mstrength * delta_time;
            // Slight inward pull
            p.velocity += normalize(to_mouse) * mstrength * 0.3 * delta_time;
        }
    }
"#.into(),
            MousePower::Explode => r#"
    // Mouse Explode (impulse while held)
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist > 0.001 && dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            p.velocity -= normalize(to_mouse) * mstrength * delta_time * 10.0;
        }
    }
"#.into(),
            MousePower::GravityWell => r#"
    // Mouse Gravity Well
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist > 0.01) {
            let mstrength = mouse_strength / (dist * dist + 0.01);
            p.velocity += normalize(to_mouse) * mstrength * delta_time;
        }
    }
"#.into(),
            MousePower::Paint => r#"
    // Mouse Paint
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist < mouse_radius) {
            let t = 1.0 - dist / mouse_radius;
            p.color = mix(p.color, mouse_color, t * mouse_strength * delta_time);
        }
    }
"#.into(),
            MousePower::Turbulence => r#"
    // Mouse Turbulence
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            let noise_input = p.position * 10.0 + vec3<f32>(time * 3.0);
            let noise_val = vec3<f32>(
                fract(sin(dot(noise_input, vec3<f32>(12.9898, 78.233, 45.164))) * 43758.5453) - 0.5,
                fract(sin(dot(noise_input, vec3<f32>(93.989, 67.345, 12.456))) * 28462.6342) - 0.5,
                fract(sin(dot(noise_input, vec3<f32>(45.164, 12.987, 93.123))) * 63829.2847) - 0.5
            );
            p.velocity += noise_val * mstrength * delta_time * 5.0;
        }
    }
"#.into(),
            MousePower::Freeze => r#"
    // Mouse Freeze
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist < mouse_radius) {
            let freeze_amount = mouse_strength * (1.0 - dist / mouse_radius) * delta_time * 5.0;
            p.velocity *= max(0.0, 1.0 - freeze_amount);
        }
    }
"#.into(),
            MousePower::Kill => r#"
    // Mouse Kill
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist < mouse_radius) {
            p.alive = 0u;
        }
    }
"#.into(),
            MousePower::Spawn => String::new(),  // Spawn is handled in to_early_wgsl()
            MousePower::BlackHole => r#"
    // Mouse Black Hole
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist > 0.01) {
            // Strong gravity pull
            let mstrength = mouse_strength * 3.0 / (dist * dist + 0.01);
            p.velocity += normalize(to_mouse) * mstrength * delta_time;
            // Kill if too close
            if (dist < mouse_radius * 0.1) {
                p.alive = 0u;
            }
        }
    }
"#.into(),
            MousePower::Orbit => r#"
    // Mouse Orbit - stable circular orbits around cursor
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist > 0.01 && dist < mouse_radius * 2.0) {
            // Calculate orbital velocity (perpendicular to radius)
            let tangent = normalize(vec3<f32>(-to_mouse.z, 0.0, to_mouse.x));
            let orbital_speed = sqrt(mouse_strength / (dist + 0.1));
            // Blend toward orbital velocity
            let target_vel = tangent * orbital_speed;
            p.velocity = mix(p.velocity, target_vel, delta_time * 3.0);
            // Slight correction toward ideal orbit distance
            let ideal_dist = mouse_radius;
            let correction = (dist - ideal_dist) * 0.5;
            p.velocity += normalize(to_mouse) * correction * delta_time;
        }
    }
"#.into(),
            MousePower::Scatter => r#"
    // Mouse Scatter - random velocity impulse
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            // Random direction based on particle index and time
            let hash1 = fract(sin(f32(index) * 12.9898 + time * 43.233) * 43758.5453);
            let hash2 = fract(sin(f32(index) * 78.233 + time * 12.989) * 28462.6342);
            let hash3 = fract(sin(f32(index) * 45.164 + time * 93.123) * 63829.2847);
            let random_dir = normalize(vec3<f32>(hash1 - 0.5, hash2 - 0.5, hash3 - 0.5));
            p.velocity += random_dir * mstrength * delta_time * 10.0;
        }
    }
"#.into(),
            MousePower::Wind => r#"
    // Mouse Wind - directional push (blows toward +X direction, rotates with camera)
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            // Wind blows away from cursor center (outward but horizontal)
            var wind_dir = normalize(vec3<f32>(to_mouse.x, 0.0, to_mouse.z));
            if (length(vec3<f32>(to_mouse.x, 0.0, to_mouse.z)) < 0.01) {
                wind_dir = vec3<f32>(1.0, 0.0, 0.0);
            }
            p.velocity += wind_dir * mstrength * delta_time * 5.0;
        }
    }
"#.into(),
            MousePower::Pulse => r#"
    // Mouse Pulse - rhythmic expanding wave
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        // Create expanding rings
        let wave_speed = 2.0;
        let wave_width = mouse_radius * 0.3;
        let wave_pos = fract(time * wave_speed) * mouse_radius * 2.0;
        let wave_dist = abs(dist - wave_pos);
        if (wave_dist < wave_width && dist < mouse_radius * 2.0) {
            let wave_strength = (1.0 - wave_dist / wave_width) * mouse_strength;
            p.velocity += normalize(-to_mouse) * wave_strength * delta_time * 5.0;
        }
    }
"#.into(),
            MousePower::Repulsor => r#"
    // Mouse Repulsor - ring-shaped outward push
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        // Only affect particles near the ring edge
        let ring_inner = mouse_radius * 0.7;
        let ring_outer = mouse_radius * 1.0;
        if (dist > ring_inner && dist < ring_outer) {
            let ring_strength = 1.0 - abs(dist - (ring_inner + ring_outer) * 0.5) / ((ring_outer - ring_inner) * 0.5);
            let mstrength = mouse_strength * ring_strength;
            p.velocity += normalize(-to_mouse) * mstrength * delta_time * 5.0;
        }
    }
"#.into(),
            MousePower::SpiralIn => r#"
    // Mouse Spiral In - drain/vortex pulling inward
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist > 0.01 && dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            // Tangential component (spin)
            let tangent = normalize(vec3<f32>(-to_mouse.z, 0.0, to_mouse.x));
            p.velocity += tangent * mstrength * delta_time * 3.0;
            // Inward component (pull) - stronger as you get closer
            let inward = normalize(to_mouse) * mstrength * delta_time * 2.0;
            p.velocity += inward;
        }
    }
"#.into(),
            MousePower::RandomVelocity => r#"
    // Mouse Random Velocity - randomize velocities
    if (mouse_down > 0.5) {
        let to_mouse = mouse_pos - p.position;
        let dist = length(to_mouse);
        if (dist < mouse_radius) {
            let mstrength = mouse_strength * (1.0 - dist / mouse_radius);
            // Generate random velocity
            let hash1 = fract(sin(f32(index) * 12.9898 + time * 127.1) * 43758.5453) * 2.0 - 1.0;
            let hash2 = fract(sin(f32(index) * 78.233 + time * 311.7) * 28462.6342) * 2.0 - 1.0;
            let hash3 = fract(sin(f32(index) * 45.164 + time * 269.5) * 63829.2847) * 2.0 - 1.0;
            let random_vel = vec3<f32>(hash1, hash2, hash3) * mstrength;
            // Blend toward random velocity
            p.velocity = mix(p.velocity, random_vel, delta_time * 5.0);
        }
    }
"#.into(),
        }
    }

    /// Generate WGSL code that runs BEFORE the dead particle skip.
    /// This is needed for powers like Spawn that operate on dead particles.
    pub fn to_early_wgsl(&self) -> String {
        match self {
            MousePower::Spawn => r#"
    // Mouse Spawn (respawn dead particles at mouse) - runs before alive check
    if (mouse_down > 0.5 && p.alive == 0u) {
        // Random offset within radius
        let hash_val = fract(sin(f32(index) * 12.9898 + time * 100.0) * 43758.5453);
        if (hash_val < mouse_strength * delta_time * 10.0) {
            let angle1 = hash_val * 6.28318;
            let angle2 = fract(hash_val * 7.461) * 3.14159;
            let r = fract(hash_val * 3.752) * mouse_radius * 0.5;
            p.position = mouse_pos + vec3<f32>(
                sin(angle2) * cos(angle1) * r,
                cos(angle2) * r,
                sin(angle2) * sin(angle1) * r
            );
            p.velocity = vec3<f32>(0.0);
            p.alive = 1u;
            p.age = 0.0;
            p.color = mouse_color;
        }
    }
"#.into(),
            _ => String::new(),
        }
    }
}

/// Mouse interaction configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MouseConfig {
    /// The active mouse power
    pub power: MousePower,
    /// Effect radius
    pub radius: f32,
    /// Effect strength
    pub strength: f32,
    /// Color for paint/spawn effects
    pub color: [f32; 3],
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            power: MousePower::None,
            radius: 0.5,  // World space units
            strength: 5.0,
            color: [1.0, 0.5, 0.2], // Orange
        }
    }
}
