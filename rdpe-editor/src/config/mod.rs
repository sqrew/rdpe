//! Configuration types for RDPE simulations.
//!
//! These types represent simulation configurations that can be serialized
//! to JSON and loaded by the runner.

mod fields;
mod mouse;
mod particle_fields;
mod rules;
mod spawn;
mod uniforms;
mod visuals;
mod volume;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Re-export all types from submodules
pub use fields::{CustomShaderConfig, FieldConfigEntry, FieldTypeConfig};
pub use mouse::{MouseConfig, MousePower};
pub use particle_fields::{ParticleFieldDef, ParticleFieldInfo, ParticleFieldType, ParticleLayout};
pub use rules::{AgentStateConfig, Falloff, RuleConfig, TransitionConfig};
pub use spawn::{ColorMode, InitialVelocity, SpawnConfig, SpawnShape};
pub use uniforms::UniformValueConfig;
pub use visuals::{
    BlendModeConfig, ColorMappingConfig, PaletteConfig, ParticleShapeConfig, VertexEffectConfig,
    VisualsConfig, WireframeMeshConfig,
};
pub use volume::VolumeRenderConfig;

fn default_speed() -> f32 {
    1.0
}

/// Complete simulation configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SimConfig {
    pub name: String,
    pub particle_count: u32,
    pub bounds: f32,
    pub particle_size: f32,
    /// Simulation speed multiplier (1.0 = normal, 0.5 = half speed, 2.0 = double speed)
    #[serde(default = "default_speed")]
    pub speed: f32,
    pub spatial_cell_size: f32,
    pub spatial_resolution: u32,
    pub spawn: SpawnConfig,
    pub rules: Vec<RuleConfig>,
    #[serde(default)]
    pub vertex_effects: Vec<VertexEffectConfig>,
    #[serde(default)]
    pub visuals: VisualsConfig,
    /// Custom uniforms accessible in shaders as `uniforms.name`.
    #[serde(default)]
    pub custom_uniforms: HashMap<String, UniformValueConfig>,
    /// Custom shader code for vertex and fragment shaders.
    #[serde(default)]
    pub custom_shaders: CustomShaderConfig,
    /// 3D spatial fields for particle-environment interaction.
    #[serde(default)]
    pub fields: Vec<FieldConfigEntry>,
    /// Volume rendering configuration for field visualization.
    #[serde(default)]
    pub volume_render: VolumeRenderConfig,
    /// Custom particle fields beyond the base fields.
    ///
    /// Base fields (always present):
    /// - position: vec3<f32>
    /// - velocity: vec3<f32>
    /// - color: vec3<f32>
    /// - age: f32
    /// - alive: u32
    /// - scale: f32
    ///
    /// Custom fields are appended after the base fields.
    #[serde(default)]
    pub particle_fields: Vec<ParticleFieldDef>,
    /// Mouse interaction configuration.
    #[serde(default)]
    pub mouse: MouseConfig,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            particle_count: 5000,
            bounds: 1.0,
            particle_size: 0.015,
            speed: 1.0,
            spatial_cell_size: 0.1,
            spatial_resolution: 32,
            spawn: SpawnConfig::default(),
            rules: vec![
                RuleConfig::Gravity(2.0),
                RuleConfig::Drag(0.5),
                RuleConfig::BounceWalls,
            ],
            vertex_effects: Vec::new(),
            visuals: VisualsConfig::default(),
            custom_uniforms: HashMap::new(),
            custom_shaders: CustomShaderConfig::default(),
            fields: Vec::new(),
            volume_render: VolumeRenderConfig::default(),
            particle_fields: Vec::new(),
            mouse: MouseConfig::default(),
        }
    }
}

impl SimConfig {
    /// Save the configuration to a JSON file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load a configuration from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let json = fs::read_to_string(path)?;
        let config = serde_json::from_str(&json)?;
        Ok(config)
    }

    /// Check if the simulation needs spatial hashing.
    pub fn needs_spatial(&self) -> bool {
        self.visuals.connections_enabled
            || self.visuals.spatial_grid_opacity > 0.0
            || self.rules.iter().any(|r| r.requires_neighbors())
    }

    /// Create a FieldRegistry from the config.
    pub fn to_field_registry(&self) -> rdpe::FieldRegistry {
        let mut registry = rdpe::FieldRegistry::new();
        for field in &self.fields {
            registry.add(&field.name, field.to_field_config());
        }
        registry
    }

    /// Compute the particle memory layout based on custom fields.
    pub fn particle_layout(&self) -> ParticleLayout {
        ParticleLayout::compute(&self.particle_fields)
    }

    /// Generate the WGSL particle struct definition.
    pub fn particle_wgsl_struct(&self) -> String {
        self.particle_layout().to_wgsl_struct()
    }

    /// Check if a custom field with the given name is defined.
    pub fn has_custom_field(&self, name: &str) -> bool {
        self.particle_fields.iter().any(|f| f.name == name)
    }
}
