//! Particle spawning utilities for the embedded simulation.
//!
//! This module handles converting SpawnConfig to actual particle data
//! that can be uploaded to the GPU. Particles are generated as raw bytes
//! based on the dynamic ParticleLayout.

use glam::Vec3;
use crate::config::{SimConfig, SpawnShape, InitialVelocity, ColorMode, ParticleLayout, ParticleFieldType};
use crate::particle::hsv_to_rgb;
use rand::Rng;

/// Generate initial particle data from config.
///
/// Returns a byte buffer containing GPU-ready particle data.
/// The layout is determined dynamically from config.particle_layout().
pub fn generate_particles(config: &SimConfig) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let spawn = &config.spawn;
    let layout = config.particle_layout();

    let mut data = Vec::with_capacity(config.particle_count as usize * layout.stride);

    for i in 0..config.particle_count {
        // Generate position based on spawn shape
        let position = match &spawn.shape {
            SpawnShape::Cube { size } => {
                Vec3::new(
                    (rng.gen::<f32>() - 0.5) * 2.0 * size,
                    (rng.gen::<f32>() - 0.5) * 2.0 * size,
                    (rng.gen::<f32>() - 0.5) * 2.0 * size,
                )
            }
            SpawnShape::Sphere { radius } => {
                random_in_sphere(&mut rng, *radius)
            }
            SpawnShape::Shell { inner, outer } => {
                let dir = random_direction(&mut rng);
                let r = *inner + rng.gen::<f32>() * (*outer - *inner);
                dir * r
            }
            SpawnShape::Ring { radius, thickness } => {
                let angle = rng.gen::<f32>() * std::f32::consts::TAU;
                let r = *radius + (rng.gen::<f32>() - 0.5) * *thickness;
                Vec3::new(angle.cos() * r, (rng.gen::<f32>() - 0.5) * *thickness, angle.sin() * r)
            }
            SpawnShape::Point => Vec3::ZERO,
            SpawnShape::Line { length } => {
                let t = rng.gen::<f32>() - 0.5;
                Vec3::new(t * *length, 0.0, 0.0)
            }
            SpawnShape::Plane { width, depth } => {
                Vec3::new(
                    (rng.gen::<f32>() - 0.5) * *width,
                    0.0,
                    (rng.gen::<f32>() - 0.5) * *depth,
                )
            }
        };

        // Generate velocity based on config
        let velocity = match &spawn.velocity {
            InitialVelocity::Zero => Vec3::ZERO,
            InitialVelocity::RandomDirection { speed } => {
                random_direction(&mut rng) * *speed
            }
            InitialVelocity::Outward { speed } => {
                if position.length() > 0.001 {
                    position.normalize() * *speed
                } else {
                    random_direction(&mut rng) * *speed
                }
            }
            InitialVelocity::Inward { speed } => {
                if position.length() > 0.001 {
                    -position.normalize() * *speed
                } else {
                    random_direction(&mut rng) * *speed
                }
            }
            InitialVelocity::Swirl { speed } => {
                let tangent = Vec3::new(-position.z, 0.0, position.x);
                if tangent.length() > 0.001 {
                    tangent.normalize() * *speed
                } else {
                    random_direction(&mut rng) * *speed
                }
            }
            InitialVelocity::Directional { direction, speed } => {
                Vec3::from_array(*direction).normalize_or_zero() * *speed
            }
        };

        // Generate color based on config
        let color = match &spawn.color_mode {
            ColorMode::Uniform { r, g, b } => Vec3::new(*r, *g, *b),
            ColorMode::RandomHue { saturation, value } => {
                hsv_to_rgb(rng.gen::<f32>(), *saturation, *value)
            }
            ColorMode::ByPosition => {
                Vec3::new(
                    position.x * 0.5 + 0.5,
                    position.y * 0.5 + 0.5,
                    position.z * 0.5 + 0.5,
                )
            }
            ColorMode::ByVelocity => {
                let speed = velocity.length();
                hsv_to_rgb((speed * 2.0).fract(), 0.9, 0.9)
            }
            ColorMode::Gradient { start, end } => {
                let t = i as f32 / config.particle_count.max(1) as f32;
                let start = Vec3::from_array(*start);
                let end = Vec3::from_array(*end);
                start.lerp(end, t)
            }
        };

        // Pick particle type based on weights
        let particle_type = pick_particle_type(&mut rng, &spawn.type_weights);

        // Generate particle bytes
        let particle_bytes = generate_particle_bytes(
            &layout,
            position,
            velocity,
            color,
            particle_type,
        );
        data.extend_from_slice(&particle_bytes);
    }

    data
}

/// Pick a particle type based on weight distribution.
///
/// `weights` is a list where index = particle_type, value = relative weight.
/// Returns 0 if weights is empty.
fn pick_particle_type(rng: &mut impl Rng, weights: &[f32]) -> u32 {
    if weights.is_empty() || weights.len() == 1 {
        return 0;
    }

    let total: f32 = weights.iter().sum();
    if total <= 0.0 {
        return 0;
    }

    let mut r = rng.gen::<f32>() * total;
    for (i, &weight) in weights.iter().enumerate() {
        r -= weight;
        if r <= 0.0 {
            return i as u32;
        }
    }

    // Fallback to last type (shouldn't happen with proper weights)
    (weights.len() - 1) as u32
}

/// Generate bytes for a single particle based on the layout.
fn generate_particle_bytes(
    layout: &ParticleLayout,
    position: Vec3,
    velocity: Vec3,
    color: Vec3,
    particle_type: u32,
) -> Vec<u8> {
    let mut bytes = vec![0u8; layout.stride];

    // Write base fields
    write_vec3(&mut bytes, layout.position_offset, position);
    write_vec3(&mut bytes, layout.velocity_offset, velocity);
    write_vec3(&mut bytes, layout.color_offset, color);
    write_f32(&mut bytes, layout.age_offset, 0.0);
    write_u32(&mut bytes, layout.alive_offset, 1); // alive = true
    write_f32(&mut bytes, layout.scale_offset, 1.0);
    write_u32(&mut bytes, layout.particle_type_offset, particle_type);

    // Custom fields are already zero-initialized

    bytes
}

/// Write a Vec3 to bytes at the given offset.
fn write_vec3(bytes: &mut [u8], offset: usize, value: Vec3) {
    bytes[offset..offset + 4].copy_from_slice(&value.x.to_le_bytes());
    bytes[offset + 4..offset + 8].copy_from_slice(&value.y.to_le_bytes());
    bytes[offset + 8..offset + 12].copy_from_slice(&value.z.to_le_bytes());
}

/// Write an f32 to bytes at the given offset.
fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// Write a u32 to bytes at the given offset.
fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Public write functions for use by ParsedParticle::to_bytes

/// Write a Vec3 to bytes at the given offset (public).
pub fn write_vec3_pub(bytes: &mut [u8], offset: usize, value: Vec3) {
    write_vec3(bytes, offset, value);
}

/// Write an f32 to bytes at the given offset (public).
pub fn write_f32_pub(bytes: &mut [u8], offset: usize, value: f32) {
    write_f32(bytes, offset, value);
}

/// Write a u32 to bytes at the given offset (public).
pub fn write_u32_pub(bytes: &mut [u8], offset: usize, value: u32) {
    write_u32(bytes, offset, value);
}

/// Write a field value to bytes at the given offset.
pub fn write_field_value_pub(bytes: &mut [u8], offset: usize, value: &FieldValue) {
    match value {
        FieldValue::F32(v) => write_f32(bytes, offset, *v),
        FieldValue::Vec2(v) => {
            write_f32(bytes, offset, v[0]);
            write_f32(bytes, offset + 4, v[1]);
        }
        FieldValue::Vec3(v) => {
            write_f32(bytes, offset, v[0]);
            write_f32(bytes, offset + 4, v[1]);
            write_f32(bytes, offset + 8, v[2]);
        }
        FieldValue::Vec4(v) => {
            write_f32(bytes, offset, v[0]);
            write_f32(bytes, offset + 4, v[1]);
            write_f32(bytes, offset + 8, v[2]);
            write_f32(bytes, offset + 12, v[3]);
        }
        FieldValue::U32(v) => write_u32(bytes, offset, *v),
        FieldValue::I32(v) => {
            bytes[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
        }
    }
}

/// Read a Vec3 from bytes at the given offset.
pub fn read_vec3(bytes: &[u8], offset: usize) -> Vec3 {
    let x = f32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
    let y = f32::from_le_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]);
    let z = f32::from_le_bytes([bytes[offset + 8], bytes[offset + 9], bytes[offset + 10], bytes[offset + 11]]);
    Vec3::new(x, y, z)
}

/// Read an f32 from bytes at the given offset.
pub fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

/// Read a u32 from bytes at the given offset.
pub fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

/// Read an i32 from bytes at the given offset.
pub fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

/// Read a Vec2 from bytes at the given offset.
pub fn read_vec2(bytes: &[u8], offset: usize) -> [f32; 2] {
    let x = f32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
    let y = f32::from_le_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]);
    [x, y]
}

/// Read a Vec4 from bytes at the given offset.
pub fn read_vec4(bytes: &[u8], offset: usize) -> [f32; 4] {
    let x = f32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
    let y = f32::from_le_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]);
    let z = f32::from_le_bytes([bytes[offset + 8], bytes[offset + 9], bytes[offset + 10], bytes[offset + 11]]);
    let w = f32::from_le_bytes([bytes[offset + 12], bytes[offset + 13], bytes[offset + 14], bytes[offset + 15]]);
    [x, y, z, w]
}

/// Read a field value from bytes based on its type.
pub fn read_field_value(bytes: &[u8], offset: usize, field_type: ParticleFieldType) -> FieldValue {
    match field_type {
        ParticleFieldType::F32 => FieldValue::F32(read_f32(bytes, offset)),
        ParticleFieldType::Vec2 => FieldValue::Vec2(read_vec2(bytes, offset)),
        ParticleFieldType::Vec3 => {
            let v = read_vec3(bytes, offset);
            FieldValue::Vec3([v.x, v.y, v.z])
        }
        ParticleFieldType::Vec4 => FieldValue::Vec4(read_vec4(bytes, offset)),
        ParticleFieldType::U32 => FieldValue::U32(read_u32(bytes, offset)),
        ParticleFieldType::I32 => FieldValue::I32(read_i32(bytes, offset)),
    }
}

/// A field value that can hold any supported type.
#[derive(Clone, Debug)]
pub enum FieldValue {
    F32(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    U32(u32),
    I32(i32),
}

impl std::fmt::Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValue::F32(v) => write!(f, "{:.4}", v),
            FieldValue::Vec2(v) => write!(f, "[{:.3}, {:.3}]", v[0], v[1]),
            FieldValue::Vec3(v) => write!(f, "[{:.3}, {:.3}, {:.3}]", v[0], v[1], v[2]),
            FieldValue::Vec4(v) => write!(f, "[{:.3}, {:.3}, {:.3}, {:.3}]", v[0], v[1], v[2], v[3]),
            FieldValue::U32(v) => write!(f, "{}", v),
            FieldValue::I32(v) => write!(f, "{}", v),
        }
    }
}

/// Generate a random point in a sphere.
fn random_in_sphere<R: Rng>(rng: &mut R, radius: f32) -> Vec3 {
    loop {
        let x = rng.gen::<f32>() * 2.0 - 1.0;
        let y = rng.gen::<f32>() * 2.0 - 1.0;
        let z = rng.gen::<f32>() * 2.0 - 1.0;
        let v = Vec3::new(x, y, z);
        if v.length_squared() <= 1.0 {
            return v * radius;
        }
    }
}

/// Generate a random direction on the unit sphere.
fn random_direction<R: Rng>(rng: &mut R) -> Vec3 {
    loop {
        let x = rng.gen::<f32>() * 2.0 - 1.0;
        let y = rng.gen::<f32>() * 2.0 - 1.0;
        let z = rng.gen::<f32>() * 2.0 - 1.0;
        let v = Vec3::new(x, y, z);
        let len_sq = v.length_squared();
        if len_sq > 0.001 && len_sq <= 1.0 {
            return v.normalize();
        }
    }
}
