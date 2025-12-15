//! Post-processing configuration types

use serde::{Deserialize, Serialize};

/// Post-processing effect presets
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PostProcessPreset {
    /// No post-processing (passthrough)
    #[default]
    None,
    /// Bloom effect - bright areas glow
    Bloom,
    /// Chromatic aberration - RGB color fringing
    ChromaticAberration,
    /// Vignette - darkened edges
    Vignette,
    /// CRT/Retro scanlines effect
    CRT,
    /// Custom user-defined shader
    Custom,
}

impl PostProcessPreset {
    pub fn variants() -> &'static [&'static str] {
        &["None", "Bloom", "Chromatic Aberration", "Vignette", "CRT", "Custom"]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PostProcessPreset::None => "None",
            PostProcessPreset::Bloom => "Bloom",
            PostProcessPreset::ChromaticAberration => "Chromatic Aberration",
            PostProcessPreset::Vignette => "Vignette",
            PostProcessPreset::CRT => "CRT",
            PostProcessPreset::Custom => "Custom",
        }
    }

    /// Get the default shader code for this preset
    pub fn default_shader(&self) -> &'static str {
        match self {
            PostProcessPreset::None => SHADER_NONE,
            PostProcessPreset::Bloom => SHADER_BLOOM,
            PostProcessPreset::ChromaticAberration => SHADER_CHROMATIC_ABERRATION,
            PostProcessPreset::Vignette => SHADER_VIGNETTE,
            PostProcessPreset::CRT => SHADER_CRT,
            PostProcessPreset::Custom => SHADER_CUSTOM_DEFAULT,
        }
    }
}

/// Post-processing configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PostProcessConfig {
    /// Enable post-processing
    pub enabled: bool,
    /// Current preset (determines default shader if custom_shader is empty)
    pub preset: PostProcessPreset,
    /// Custom shader code (if empty, uses preset default)
    #[serde(default)]
    pub custom_shader: String,
    /// Intensity/strength of the effect (0.0 - 1.0)
    #[serde(default = "default_intensity")]
    pub intensity: f32,
}

fn default_intensity() -> f32 {
    1.0
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: PostProcessPreset::None,
            custom_shader: String::new(),
            intensity: 1.0,
        }
    }
}

impl PostProcessConfig {
    /// Get the shader code to use (custom if provided, otherwise preset default)
    pub fn shader_code(&self) -> &str {
        if self.custom_shader.is_empty() {
            self.preset.default_shader()
        } else {
            &self.custom_shader
        }
    }
}

// Preset shader implementations

const SHADER_NONE: &str = r#"// Passthrough - no effect
let color = textureSample(input_texture, input_sampler, uv);
output_color = color;
"#;

const SHADER_BLOOM: &str = r#"// Bloom effect - bright areas glow
let color = textureSample(input_texture, input_sampler, uv);

// Simple box blur for bloom
let blur_size = 0.003 * pp_intensity;
var bloom = vec3<f32>(0.0);
let samples = 9.0;
for (var x = -1; x <= 1; x++) {
    for (var y = -1; y <= 1; y++) {
        let offset = vec2<f32>(f32(x), f32(y)) * blur_size;
        let sample_color = textureSample(input_texture, input_sampler, uv + offset).rgb;
        // Only add bright parts
        let brightness = max(max(sample_color.r, sample_color.g), sample_color.b);
        let threshold = 0.7;
        if brightness > threshold {
            bloom += sample_color * (brightness - threshold) / (1.0 - threshold);
        }
    }
}
bloom /= samples;

// Add bloom to original
output_color = vec4<f32>(color.rgb + bloom * 1.5, color.a);
"#;

const SHADER_CHROMATIC_ABERRATION: &str = r#"// Chromatic aberration - RGB color fringing
let offset = 0.004 * pp_intensity;

// Sample RGB channels at offset positions
let r = textureSample(input_texture, input_sampler, uv + vec2<f32>(offset, 0.0)).r;
let g = textureSample(input_texture, input_sampler, uv).g;
let b = textureSample(input_texture, input_sampler, uv - vec2<f32>(offset, 0.0)).b;
let a = textureSample(input_texture, input_sampler, uv).a;

output_color = vec4<f32>(r, g, b, a);
"#;

const SHADER_VIGNETTE: &str = r#"// Vignette - darkened edges
let color = textureSample(input_texture, input_sampler, uv);

// Calculate distance from center
let center = vec2<f32>(0.5, 0.5);
let dist = distance(uv, center);

// Vignette falloff
let vignette = 1.0 - smoothstep(0.3, 0.9, dist * pp_intensity);

output_color = vec4<f32>(color.rgb * vignette, color.a);
"#;

const SHADER_CRT: &str = r#"// CRT/Retro effect with scanlines
let color = textureSample(input_texture, input_sampler, uv);

// Scanlines
let scanline = sin(uv.y * pp_resolution.y * 3.14159) * 0.5 + 0.5;
let scanline_strength = 0.15 * pp_intensity;
let scan_color = color.rgb * (1.0 - scanline_strength + scanline * scanline_strength);

// Slight RGB offset for CRT look
let offset = 0.001 * pp_intensity;
let r = textureSample(input_texture, input_sampler, uv + vec2<f32>(offset, 0.0)).r;
let b = textureSample(input_texture, input_sampler, uv - vec2<f32>(offset, 0.0)).b;

// Vignette for CRT curvature feel
let center = vec2<f32>(0.5, 0.5);
let dist = distance(uv, center);
let vignette = 1.0 - smoothstep(0.5, 1.0, dist);

output_color = vec4<f32>(r * vignette, scan_color.g * vignette, b * vignette, color.a);
"#;

const SHADER_CUSTOM_DEFAULT: &str = r#"// Custom post-process shader
// Available inputs:
//   uv: vec2<f32>           - Screen coordinates (0-1)
//   input_texture           - The rendered scene
//   input_sampler           - Sampler for the texture
//   pp_time: f32            - Time in seconds
//   pp_intensity: f32       - Effect intensity (0-1)
//   pp_resolution: vec2<f32> - Screen resolution in pixels

let color = textureSample(input_texture, input_sampler, uv);

// Your custom effect here!
// Example: invert colors
// output_color = vec4<f32>(1.0 - color.rgb, color.a);

output_color = color;
"#;
