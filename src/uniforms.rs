//! Custom uniforms for passing runtime data to shaders.
//!
//! Custom uniforms let you pass dynamic values to your particle rules,
//! enabling interactive and reactive simulations.
//!
//! # Example
//!
//! ```ignore
//! Simulation::<Particle>::new()
//!     .with_uniform("attractor", Vec3::ZERO)
//!     .with_uniform("strength", 1.0f32)
//!     .with_update(|ctx| {
//!         // Update uniforms based on input
//!         if let Some(pos) = ctx.mouse_world_position() {
//!             ctx.set("attractor", pos);
//!         }
//!         ctx.set("strength", (ctx.time() * 2.0).sin() * 0.5 + 1.0);
//!     })
//!     .with_rule(Rule::Custom(r#"
//!         let dir = uniforms.attractor - p.position;
//!         p.velocity += normalize(dir) * uniforms.strength * uniforms.delta_time;
//!     "#.into()))
//!     .run();
//! ```

use glam::{Vec2, Vec3, Vec4};
use std::collections::HashMap;

/// Supported uniform value types.
#[derive(Clone, Copy, Debug)]
pub enum UniformValue {
    F32(f32),
    I32(i32),
    U32(u32),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
}

impl UniformValue {
    /// Get the WGSL type name for this value.
    pub fn wgsl_type(&self) -> &'static str {
        match self {
            UniformValue::F32(_) => "f32",
            UniformValue::I32(_) => "i32",
            UniformValue::U32(_) => "u32",
            UniformValue::Vec2(_) => "vec2<f32>",
            UniformValue::Vec3(_) => "vec3<f32>",
            UniformValue::Vec4(_) => "vec4<f32>",
        }
    }

    /// Get the byte size of this value (without trailing padding).
    pub fn byte_size(&self) -> usize {
        match self {
            UniformValue::F32(_) => 4,
            UniformValue::I32(_) => 4,
            UniformValue::U32(_) => 4,
            UniformValue::Vec2(_) => 8,
            UniformValue::Vec3(_) => 12, // 12 bytes, aligned to 16
            UniformValue::Vec4(_) => 16,
        }
    }

    /// Write this value to a byte buffer.
    pub fn write_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            UniformValue::F32(v) => buf.extend_from_slice(&v.to_le_bytes()),
            UniformValue::I32(v) => buf.extend_from_slice(&v.to_le_bytes()),
            UniformValue::U32(v) => buf.extend_from_slice(&v.to_le_bytes()),
            UniformValue::Vec2(v) => {
                buf.extend_from_slice(&v.x.to_le_bytes());
                buf.extend_from_slice(&v.y.to_le_bytes());
            }
            UniformValue::Vec3(v) => {
                buf.extend_from_slice(&v.x.to_le_bytes());
                buf.extend_from_slice(&v.y.to_le_bytes());
                buf.extend_from_slice(&v.z.to_le_bytes());
                // No padding here - next value handles its own alignment
                // In std140, scalars can fit in vec3's trailing bytes
            }
            UniformValue::Vec4(v) => {
                buf.extend_from_slice(&v.x.to_le_bytes());
                buf.extend_from_slice(&v.y.to_le_bytes());
                buf.extend_from_slice(&v.z.to_le_bytes());
                buf.extend_from_slice(&v.w.to_le_bytes());
            }
        }
    }
}

// Conversion traits for ergonomic API
impl From<f32> for UniformValue {
    fn from(v: f32) -> Self {
        UniformValue::F32(v)
    }
}

impl From<i32> for UniformValue {
    fn from(v: i32) -> Self {
        UniformValue::I32(v)
    }
}

impl From<u32> for UniformValue {
    fn from(v: u32) -> Self {
        UniformValue::U32(v)
    }
}

impl From<Vec2> for UniformValue {
    fn from(v: Vec2) -> Self {
        UniformValue::Vec2(v)
    }
}

impl From<Vec3> for UniformValue {
    fn from(v: Vec3) -> Self {
        UniformValue::Vec3(v)
    }
}

impl From<Vec4> for UniformValue {
    fn from(v: Vec4) -> Self {
        UniformValue::Vec4(v)
    }
}

/// Collection of custom uniform values.
#[derive(Clone, Debug, Default)]
pub struct CustomUniforms {
    /// Ordered list of (name, value) pairs.
    /// Order matters for WGSL struct layout.
    values: Vec<(String, UniformValue)>,
    /// Quick lookup by name.
    indices: HashMap<String, usize>,
}

impl CustomUniforms {
    /// Create empty custom uniforms.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update a uniform value.
    pub fn set<V: Into<UniformValue>>(&mut self, name: &str, value: V) {
        let value = value.into();
        if let Some(&idx) = self.indices.get(name) {
            self.values[idx].1 = value;
        } else {
            let idx = self.values.len();
            self.values.push((name.to_string(), value));
            self.indices.insert(name.to_string(), idx);
        }
    }

    /// Get a uniform value by name.
    pub fn get(&self, name: &str) -> Option<&UniformValue> {
        self.indices.get(name).map(|&idx| &self.values[idx].1)
    }

    /// Check if any custom uniforms are defined.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get the number of custom uniforms.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Iterate over all uniforms.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &UniformValue)> {
        self.values.iter().map(|(n, v)| (n.as_str(), v))
    }

    /// Generate WGSL struct fields for custom uniforms.
    pub(crate) fn to_wgsl_fields(&self) -> String {
        self.values
            .iter()
            .map(|(name, value)| format!("    {}: {},", name, value.wgsl_type()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Serialize all values to bytes for GPU upload.
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for (_, value) in &self.values {
            // Add padding for alignment
            let align = match value {
                UniformValue::Vec4(_) | UniformValue::Vec3(_) => 16,
                UniformValue::Vec2(_) => 8,
                _ => 4,
            };
            while buf.len() % align != 0 {
                buf.push(0);
            }
            value.write_bytes(&mut buf);
        }
        buf
    }

    /// Calculate total byte size with alignment.
    pub(crate) fn byte_size(&self) -> usize {
        let bytes = self.to_bytes();
        // Round up to 16-byte alignment for uniform buffer
        (bytes.len() + 15) & !15
    }
}

/// Context passed to the update callback each frame.
///
/// Provides access to input state and allows updating custom uniforms.
pub struct UpdateContext<'a> {
    /// Custom uniforms that can be modified.
    pub(crate) uniforms: &'a mut CustomUniforms,
    /// Current simulation time in seconds.
    pub(crate) time: f32,
    /// Time since last frame in seconds.
    pub(crate) delta_time: f32,
    /// Current mouse position in normalized device coordinates (-1 to 1).
    pub(crate) mouse_ndc: Option<Vec2>,
    /// Is the left mouse button pressed?
    pub(crate) mouse_pressed: bool,
}

impl<'a> UpdateContext<'a> {
    /// Get the current simulation time in seconds.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Get the time since last frame in seconds.
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    /// Get the mouse position in normalized device coordinates (-1 to 1).
    ///
    /// Returns `None` if the mouse is outside the window.
    pub fn mouse_ndc(&self) -> Option<Vec2> {
        self.mouse_ndc
    }

    /// Check if the left mouse button is pressed.
    pub fn mouse_pressed(&self) -> bool {
        self.mouse_pressed
    }

    /// Set a custom uniform value.
    pub fn set<V: Into<UniformValue>>(&mut self, name: &str, value: V) {
        self.uniforms.set(name, value);
    }

    /// Get a custom uniform value.
    pub fn get(&self, name: &str) -> Option<&UniformValue> {
        self.uniforms.get(name)
    }
}
