//! Shared particle type for the rdpe editor.
//!
//! This module defines the MetaParticle type used by both the editor
//! and the runner binary. It's a flexible particle type with common fields
//! that can handle most simulation types.

use glam::Vec3;
use rdpe::prelude::*;

/// Flexible particle type for the meta simulation.
///
/// This particle type includes common fields used across many simulation types:
/// - position/velocity: Required by all particles
/// - color: For visual customization
/// - particle_type: For typed interactions
/// - mass/energy/heat/custom: For physics and custom behavior
/// - goal: For seeking/flocking behaviors
#[derive(Particle, Clone)]
pub struct MetaParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    #[color]
    pub color: Vec3,
    pub particle_type: u32,
    pub mass: f32,
    pub energy: f32,
    pub heat: f32,
    pub custom: f32,
    pub goal: Vec3,
}

impl MetaParticle {
    /// Get the WGSL struct definition for this particle type.
    pub fn wgsl_struct() -> &'static str {
        <Self as rdpe::ParticleTrait>::WGSL_STRUCT
    }

    /// Get the color field offset.
    pub fn color_offset() -> Option<u32> {
        <Self as rdpe::ParticleTrait>::COLOR_OFFSET
    }

    /// Get the alive field offset.
    pub fn alive_offset() -> u32 {
        <Self as rdpe::ParticleTrait>::ALIVE_OFFSET
    }

    /// Get the scale field offset.
    pub fn scale_offset() -> u32 {
        <Self as rdpe::ParticleTrait>::SCALE_OFFSET
    }

    /// Get the size of the GPU struct in bytes.
    pub fn gpu_stride() -> usize {
        std::mem::size_of::<<Self as rdpe::ParticleTrait>::Gpu>()
    }
}

/// HSV to RGB conversion helper.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let h = h * 6.0;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 1.0 {
        (c, x, 0.0)
    } else if h < 2.0 {
        (x, c, 0.0)
    } else if h < 3.0 {
        (0.0, c, x)
    } else if h < 4.0 {
        (0.0, x, c)
    } else if h < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Vec3::new(r + m, g + m, b + m)
}
