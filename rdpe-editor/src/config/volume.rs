//! Volume rendering configuration

use serde::{Deserialize, Serialize};

use super::PaletteConfig;

/// Configuration for volume rendering (ray marching visualization of fields).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VolumeRenderConfig {
    /// Whether volume rendering is enabled.
    pub enabled: bool,
    /// Which field index to render (default: 0).
    pub field_index: u32,
    /// Number of ray march steps (higher = better quality, slower).
    pub steps: u32,
    /// Density multiplier (higher = more opaque).
    pub density_scale: f32,
    /// Color palette for density mapping.
    pub palette: PaletteConfig,
    /// Minimum density threshold (values below are transparent).
    pub threshold: f32,
    /// Whether to use additive blending (glow effect).
    pub additive: bool,
}

impl Default for VolumeRenderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            field_index: 0,
            steps: 64,
            density_scale: 5.0,
            palette: PaletteConfig::Inferno,
            threshold: 0.01,
            additive: true,
        }
    }
}

impl VolumeRenderConfig {
    /// Convert to rdpe::VolumeConfig.
    pub fn to_volume_config(&self) -> rdpe::VolumeConfig {
        rdpe::VolumeConfig {
            field_index: self.field_index,
            steps: self.steps,
            density_scale: self.density_scale,
            palette: self.palette.to_palette(),
            threshold: self.threshold,
            additive: self.additive,
        }
    }
}
