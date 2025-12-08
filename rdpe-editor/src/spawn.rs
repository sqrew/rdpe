//! Particle spawning utilities for the embedded simulation.
//!
//! This module handles converting SpawnConfig to actual particle data
//! that can be uploaded to the GPU.

use glam::Vec3;
use rdpe::ParticleTrait;
use crate::config::{SimConfig, SpawnShape, InitialVelocity, ColorMode};
use crate::particle::{MetaParticle, hsv_to_rgb};
use rand::Rng;

/// Generate initial particle data from config.
///
/// Returns a byte buffer containing GPU-ready particle data.
pub fn generate_particles(config: &SimConfig) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let spawn = &config.spawn;

    let mut particles = Vec::with_capacity(config.particle_count as usize);

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

        // Generate mass and energy
        let mass = spawn.mass_range.0 + rng.gen::<f32>() * (spawn.mass_range.1 - spawn.mass_range.0);
        let energy = spawn.energy_range.0 + rng.gen::<f32>() * (spawn.energy_range.1 - spawn.energy_range.0);

        particles.push(MetaParticle {
            position,
            velocity,
            color,
            particle_type: 0,
            mass,
            energy,
            heat: 0.0,
            custom: 0.0,
            goal: Vec3::ZERO,
        });
    }

    // Convert to GPU format
    particles_to_bytes(&particles)
}

/// Convert particles to GPU byte format.
fn particles_to_bytes(particles: &[MetaParticle]) -> Vec<u8> {
    let stride = MetaParticle::gpu_stride();
    let mut data = Vec::with_capacity(particles.len() * stride);

    for particle in particles {
        let gpu = particle.to_gpu();
        let bytes = bytemuck::bytes_of(&gpu);
        data.extend_from_slice(bytes);
    }

    data
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
