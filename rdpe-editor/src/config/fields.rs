//! Field system configuration

use serde::{Deserialize, Serialize};

/// Custom shader code configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CustomShaderConfig {
    /// Custom vertex shader code (injected after vertex effects).
    #[serde(default)]
    pub vertex_code: String,
    /// Custom fragment shader code (injected before final color output).
    #[serde(default)]
    pub fragment_code: String,
}

/// Field type for editor configuration.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FieldTypeConfig {
    #[default]
    Scalar,
    Vector,
}

impl FieldTypeConfig {
    pub fn variants() -> &'static [&'static str] {
        &["Scalar", "Vector"]
    }

    pub fn to_field_type(&self) -> rdpe::FieldType {
        match self {
            FieldTypeConfig::Scalar => rdpe::FieldType::Scalar,
            FieldTypeConfig::Vector => rdpe::FieldType::Vector,
        }
    }
}

/// Configuration for a single 3D spatial field.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FieldConfigEntry {
    /// Field name (for reference in code).
    pub name: String,
    /// Grid resolution per axis (8-256).
    pub resolution: u32,
    /// World-space extent (cube from -extent to +extent).
    pub extent: f32,
    /// Per-frame decay multiplier (0.0-1.0).
    pub decay: f32,
    /// Blur/diffusion strength per frame (0.0-1.0).
    pub blur: f32,
    /// Number of blur iterations per frame.
    pub blur_iterations: u32,
    /// Field type (Scalar or Vector).
    pub field_type: FieldTypeConfig,
}

impl Default for FieldConfigEntry {
    fn default() -> Self {
        Self {
            name: "field".into(),
            resolution: 64,
            extent: 1.0,
            decay: 0.98,
            blur: 0.1,
            blur_iterations: 1,
            field_type: FieldTypeConfig::Scalar,
        }
    }
}

impl FieldConfigEntry {
    pub fn to_field_config(&self) -> rdpe::FieldConfig {
        let mut config = if self.field_type == FieldTypeConfig::Vector {
            rdpe::FieldConfig::new_vector(self.resolution.clamp(8, 256))
        } else {
            rdpe::FieldConfig::new(self.resolution.clamp(8, 256))
        };
        config = config
            .with_extent(self.extent)
            .with_decay(self.decay)
            .with_blur(self.blur)
            .with_blur_iterations(self.blur_iterations);
        config
    }
}
