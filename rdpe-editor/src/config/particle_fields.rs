//! Particle field type definitions and layout computation.
//!
//! This module defines the types used to represent custom particle fields and compute
//! memory layouts for particle data structures. It includes:
//! - `ParticleFieldType`: Enum representing available WGSL field types
//! - `ParticleFieldDef`: Definition of a custom particle field
//! - `ParticleFieldInfo`: Runtime information about a field's layout
//! - `ParticleLayout`: Complete particle memory layout with std430 alignment

use serde::{Deserialize, Serialize};

/// Field types for custom particle fields.
///
/// These represent the WGSL types available for particle state.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParticleFieldType {
    /// Single 32-bit float
    #[default]
    F32,
    /// Two-component float vector
    Vec2,
    /// Three-component float vector
    Vec3,
    /// Four-component float vector
    Vec4,
    /// Unsigned 32-bit integer
    U32,
    /// Signed 32-bit integer
    I32,
}

impl ParticleFieldType {
    /// Get the WGSL type name for this field type.
    pub fn wgsl_type(&self) -> &'static str {
        match self {
            ParticleFieldType::F32 => "f32",
            ParticleFieldType::Vec2 => "vec2<f32>",
            ParticleFieldType::Vec3 => "vec3<f32>",
            ParticleFieldType::Vec4 => "vec4<f32>",
            ParticleFieldType::U32 => "u32",
            ParticleFieldType::I32 => "i32",
        }
    }

    /// Get the size in bytes for this field type.
    pub fn byte_size(&self) -> usize {
        match self {
            ParticleFieldType::F32 => 4,
            ParticleFieldType::Vec2 => 8,
            ParticleFieldType::Vec3 => 12,
            ParticleFieldType::Vec4 => 16,
            ParticleFieldType::U32 => 4,
            ParticleFieldType::I32 => 4,
        }
    }

    /// Get the alignment requirement in bytes (std430 layout).
    ///
    /// In std430:
    /// - Scalars align to their size (4 bytes)
    /// - vec2 aligns to 8 bytes
    /// - vec3 and vec4 align to 16 bytes
    pub fn alignment(&self) -> usize {
        match self {
            ParticleFieldType::F32 => 4,
            ParticleFieldType::Vec2 => 8,
            ParticleFieldType::Vec3 => 16,
            ParticleFieldType::Vec4 => 16,
            ParticleFieldType::U32 => 4,
            ParticleFieldType::I32 => 4,
        }
    }

    /// Get all available field type variants.
    pub fn variants() -> &'static [&'static str] {
        &["f32", "vec2", "vec3", "vec4", "u32", "i32"]
    }

    /// Parse from variant string.
    pub fn from_variant(s: &str) -> Option<Self> {
        match s {
            "f32" => Some(ParticleFieldType::F32),
            "vec2" => Some(ParticleFieldType::Vec2),
            "vec3" => Some(ParticleFieldType::Vec3),
            "vec4" => Some(ParticleFieldType::Vec4),
            "u32" => Some(ParticleFieldType::U32),
            "i32" => Some(ParticleFieldType::I32),
            _ => None,
        }
    }

    /// Get the display name for this type.
    pub fn display_name(&self) -> &'static str {
        match self {
            ParticleFieldType::F32 => "f32",
            ParticleFieldType::Vec2 => "vec2",
            ParticleFieldType::Vec3 => "vec3",
            ParticleFieldType::Vec4 => "vec4",
            ParticleFieldType::U32 => "u32",
            ParticleFieldType::I32 => "i32",
        }
    }
}

/// Definition of a custom particle field.
///
/// Custom fields allow users to add arbitrary state to particles
/// beyond the base fields (position, velocity, color, age, alive, scale).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ParticleFieldDef {
    /// Field name (must be a valid WGSL identifier).
    pub name: String,
    /// Field type.
    pub field_type: ParticleFieldType,
}

impl ParticleFieldDef {
    /// Create a new field definition.
    pub fn new(name: impl Into<String>, field_type: ParticleFieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
        }
    }

    /// Create an f32 field.
    pub fn f32(name: impl Into<String>) -> Self {
        Self::new(name, ParticleFieldType::F32)
    }

    /// Create a vec2 field.
    pub fn vec2(name: impl Into<String>) -> Self {
        Self::new(name, ParticleFieldType::Vec2)
    }

    /// Create a vec3 field.
    pub fn vec3(name: impl Into<String>) -> Self {
        Self::new(name, ParticleFieldType::Vec3)
    }

    /// Create a vec4 field.
    pub fn vec4(name: impl Into<String>) -> Self {
        Self::new(name, ParticleFieldType::Vec4)
    }

    /// Create a u32 field.
    pub fn u32(name: impl Into<String>) -> Self {
        Self::new(name, ParticleFieldType::U32)
    }

    /// Create an i32 field.
    pub fn i32(name: impl Into<String>) -> Self {
        Self::new(name, ParticleFieldType::I32)
    }

    /// Validate that the field name is a valid WGSL identifier.
    pub fn is_valid_name(&self) -> bool {
        let Some(first) = self.name.chars().next() else {
            return false; // Empty name
        };
        if !first.is_ascii_alphabetic() && first != '_' {
            return false;
        }
        self.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
}

impl Default for ParticleFieldDef {
    fn default() -> Self {
        Self {
            name: "custom".into(),
            field_type: ParticleFieldType::F32,
        }
    }
}

/// Information about a single field in the particle layout.
#[derive(Clone, Debug)]
pub struct ParticleFieldInfo {
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: ParticleFieldType,
    /// Byte offset from start of struct.
    pub offset: usize,
    /// Whether this is a base field (vs custom).
    pub is_base: bool,
    /// Whether this field is marked as the color field.
    pub is_color: bool,
}

/// Complete particle memory layout with all field offsets and stride.
///
/// This is computed from the base fields plus any custom fields defined
/// in the config. The layout follows std430 alignment rules.
#[derive(Clone, Debug)]
pub struct ParticleLayout {
    /// All fields in memory order.
    pub fields: Vec<ParticleFieldInfo>,
    /// Total stride (size) of one particle in bytes.
    pub stride: usize,
    /// Offset of the position field.
    pub position_offset: usize,
    /// Offset of the velocity field.
    pub velocity_offset: usize,
    /// Offset of the color field.
    pub color_offset: usize,
    /// Offset of the age field.
    pub age_offset: usize,
    /// Offset of the alive field.
    pub alive_offset: usize,
    /// Offset of the scale field.
    pub scale_offset: usize,
    /// Offset of the particle_type field.
    pub particle_type_offset: usize,
}

impl ParticleLayout {
    /// Add a field to the layout with proper alignment.
    fn add_field(
        fields: &mut Vec<ParticleFieldInfo>,
        offset: &mut usize,
        name: &str,
        field_type: ParticleFieldType,
        is_base: bool,
        is_color: bool,
    ) -> usize {
        let alignment = field_type.alignment();
        // Align offset
        *offset = (*offset).div_ceil(alignment) * alignment;

        let field_offset = *offset;

        fields.push(ParticleFieldInfo {
            name: name.to_string(),
            field_type,
            offset: field_offset,
            is_base,
            is_color,
        });

        *offset += field_type.byte_size();
        field_offset
    }

    /// Compute the particle layout from custom field definitions.
    ///
    /// Base fields are laid out first in this order:
    /// - position: vec3<f32>
    /// - velocity: vec3<f32>
    /// - color: vec3<f32>
    /// - age: f32
    /// - alive: u32
    /// - scale: f32
    /// - particle_type: u32
    ///
    /// Custom fields are appended after, sorted by alignment (largest first)
    /// to minimize padding.
    pub fn compute(custom_fields: &[ParticleFieldDef]) -> Self {
        let mut fields = Vec::new();
        let mut offset = 0usize;

        // Base fields (always present, in fixed order for vertex buffer compatibility)
        let position_offset = Self::add_field(&mut fields, &mut offset, "position", ParticleFieldType::Vec3, true, false);
        let velocity_offset = Self::add_field(&mut fields, &mut offset, "velocity", ParticleFieldType::Vec3, true, false);
        let color_offset = Self::add_field(&mut fields, &mut offset, "color", ParticleFieldType::Vec3, true, true);
        let age_offset = Self::add_field(&mut fields, &mut offset, "age", ParticleFieldType::F32, true, false);
        let alive_offset = Self::add_field(&mut fields, &mut offset, "alive", ParticleFieldType::U32, true, false);
        let scale_offset = Self::add_field(&mut fields, &mut offset, "scale", ParticleFieldType::F32, true, false);
        let particle_type_offset = Self::add_field(&mut fields, &mut offset, "particle_type", ParticleFieldType::U32, true, false);

        // Sort custom fields by alignment (descending) to minimize padding
        let mut sorted_custom: Vec<_> = custom_fields.iter().collect();
        sorted_custom.sort_by(|a, b| b.field_type.alignment().cmp(&a.field_type.alignment()));

        // Add custom fields
        for field in sorted_custom {
            Self::add_field(&mut fields, &mut offset, &field.name, field.field_type, false, false);
        }

        // Compute final stride (must be multiple of largest alignment in struct)
        // For structs with vec3, this is 16 bytes
        let max_alignment = 16; // vec3 requires 16-byte struct alignment
        let stride = offset.div_ceil(max_alignment) * max_alignment;

        Self {
            fields,
            stride,
            position_offset,
            velocity_offset,
            color_offset,
            age_offset,
            alive_offset,
            scale_offset,
            particle_type_offset,
        }
    }

    /// Get the offset of a field by name.
    pub fn field_offset(&self, name: &str) -> Option<usize> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.offset)
    }

    /// Get field info by name.
    pub fn field_info(&self, name: &str) -> Option<&ParticleFieldInfo> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Generate the WGSL struct definition.
    pub fn to_wgsl_struct(&self) -> String {
        let mut wgsl = String::from("struct Particle {\n");

        for field in &self.fields {
            wgsl.push_str(&format!(
                "    {}: {},\n",
                field.name,
                field.field_type.wgsl_type()
            ));
        }

        wgsl.push_str("}\n");
        wgsl
    }

    /// Generate zero-initialized bytes for one particle.
    pub fn zero_bytes(&self) -> Vec<u8> {
        vec![0u8; self.stride]
    }

    /// Get all custom (non-base) fields.
    pub fn custom_fields(&self) -> impl Iterator<Item = &ParticleFieldInfo> {
        self.fields.iter().filter(|f| !f.is_base)
    }

    /// Get all base fields.
    pub fn base_fields(&self) -> impl Iterator<Item = &ParticleFieldInfo> {
        self.fields.iter().filter(|f| f.is_base)
    }
}
