//! Lighting configuration for the particle simulation.
//!
//! This module defines light types (point and spot) and their properties
//! for illuminating particles with sphere impostor shading.

use serde::{Deserialize, Serialize};

/// Maximum number of lights supported in the shader.
pub const MAX_LIGHTS: usize = 8;

/// Type of light source.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LightType {
    /// Point light that emits in all directions.
    Point,
    /// Spot light with a direction and cone angle.
    Spot {
        /// Direction the spot light points (normalized).
        direction: [f32; 3],
        /// Cone angle in radians (half-angle from center).
        angle: f32,
    },
}

impl Default for LightType {
    fn default() -> Self {
        Self::Point
    }
}

impl LightType {
    /// Get the name of this light type for display.
    pub fn name(&self) -> &'static str {
        match self {
            LightType::Point => "Point",
            LightType::Spot { .. } => "Spot",
        }
    }

    /// Check if this is a spot light.
    pub fn is_spot(&self) -> bool {
        matches!(self, LightType::Spot { .. })
    }
}

/// Configuration for a single light source.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LightConfig {
    /// Display name for the light.
    pub name: String,
    /// Whether the light is enabled.
    pub enabled: bool,
    /// Type of light (point or spot).
    pub light_type: LightType,
    /// Position in world space.
    pub position: [f32; 3],
    /// Light color (RGB, 0.0-1.0).
    pub color: [f32; 3],
    /// Light intensity multiplier.
    pub intensity: f32,
    /// Distance falloff factor (higher = faster falloff).
    pub falloff: f32,
}

impl Default for LightConfig {
    fn default() -> Self {
        Self {
            name: "Light".to_string(),
            enabled: true,
            light_type: LightType::Point,
            position: [2.0, 2.0, 2.0],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            falloff: 0.5,
        }
    }
}

impl LightConfig {
    /// Create a new point light with default settings.
    pub fn new_point(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Create a new spot light with default settings.
    pub fn new_spot(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            light_type: LightType::Spot {
                direction: [0.0, -1.0, 0.0],
                angle: std::f32::consts::FRAC_PI_4, // 45 degrees
            },
            ..Default::default()
        }
    }
}

/// Configuration for the entire lighting system.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LightingConfig {
    /// Whether lighting is enabled globally.
    pub enabled: bool,
    /// Ambient light color (RGB, 0.0-1.0).
    #[serde(default = "default_ambient_color")]
    pub ambient_color: [f32; 3],
    /// Ambient light intensity.
    #[serde(default = "default_ambient_intensity")]
    pub ambient_intensity: f32,
    /// List of light sources.
    #[serde(default)]
    pub lights: Vec<LightConfig>,
}

fn default_ambient_color() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

fn default_ambient_intensity() -> f32 {
    0.2
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ambient_color: default_ambient_color(),
            ambient_intensity: default_ambient_intensity(),
            lights: Vec::new(),
        }
    }
}

impl LightingConfig {
    /// Add a new point light.
    pub fn add_point_light(&mut self) {
        let name = format!("Point Light {}", self.lights.len() + 1);
        self.lights.push(LightConfig::new_point(name));
    }

    /// Add a new spot light.
    pub fn add_spot_light(&mut self) {
        let name = format!("Spot Light {}", self.lights.len() + 1);
        self.lights.push(LightConfig::new_spot(name));
    }

    /// Remove a light at the given index.
    pub fn remove_light(&mut self, index: usize) {
        if index < self.lights.len() {
            self.lights.remove(index);
        }
    }

    /// Get the number of enabled lights (capped at MAX_LIGHTS).
    pub fn enabled_light_count(&self) -> usize {
        self.lights
            .iter()
            .filter(|l| l.enabled)
            .take(MAX_LIGHTS)
            .count()
    }
}
