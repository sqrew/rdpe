//! Custom uniform value types for shader parameters.
//!
//! This module defines types for representing shader uniform values that can be
//! serialized to JSON and converted to bytes for GPU upload.

use serde::{Deserialize, Serialize};

/// Custom uniform value types for shader parameters.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UniformValueConfig {
    F32(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
}

impl UniformValueConfig {
    pub fn wgsl_type(&self) -> &'static str {
        match self {
            UniformValueConfig::F32(_) => "f32",
            UniformValueConfig::Vec2(_) => "vec2<f32>",
            UniformValueConfig::Vec3(_) => "vec3<f32>",
            UniformValueConfig::Vec4(_) => "vec4<f32>",
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self {
            UniformValueConfig::F32(v) => bytes.extend_from_slice(&v.to_le_bytes()),
            UniformValueConfig::Vec2(v) => {
                bytes.extend_from_slice(&v[0].to_le_bytes());
                bytes.extend_from_slice(&v[1].to_le_bytes());
            }
            UniformValueConfig::Vec3(v) => {
                bytes.extend_from_slice(&v[0].to_le_bytes());
                bytes.extend_from_slice(&v[1].to_le_bytes());
                bytes.extend_from_slice(&v[2].to_le_bytes());
            }
            UniformValueConfig::Vec4(v) => {
                bytes.extend_from_slice(&v[0].to_le_bytes());
                bytes.extend_from_slice(&v[1].to_le_bytes());
                bytes.extend_from_slice(&v[2].to_le_bytes());
                bytes.extend_from_slice(&v[3].to_le_bytes());
            }
        }
        bytes
    }

    pub fn byte_size(&self) -> usize {
        match self {
            UniformValueConfig::F32(_) => 4,
            UniformValueConfig::Vec2(_) => 8,
            UniformValueConfig::Vec3(_) => 12,
            UniformValueConfig::Vec4(_) => 16,
        }
    }

    pub fn alignment(&self) -> usize {
        match self {
            UniformValueConfig::F32(_) => 4,
            UniformValueConfig::Vec2(_) => 8,
            UniformValueConfig::Vec3(_) => 16, // vec3 aligns to 16 in std140
            UniformValueConfig::Vec4(_) => 16,
        }
    }
}
