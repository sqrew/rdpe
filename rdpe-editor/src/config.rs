//! Configuration types for RDPE simulations.
//!
//! These types represent simulation configurations that can be serialized
//! to JSON and loaded by the runner.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

/// Field types for custom particle fields.
///
/// These represent the WGSL types available for particle state.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParticleFieldType {
    /// Single 32-bit float
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

impl Default for ParticleFieldType {
    fn default() -> Self {
        ParticleFieldType::F32
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
        if self.name.is_empty() {
            return false;
        }
        let first = self.name.chars().next().unwrap();
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

/// Spawn shape configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SpawnShape {
    Cube { size: f32 },
    Sphere { radius: f32 },
    Shell { inner: f32, outer: f32 },
    Ring { radius: f32, thickness: f32 },
    Point,
    Line { length: f32 },
    Plane { width: f32, depth: f32 },
}

impl Default for SpawnShape {
    fn default() -> Self {
        SpawnShape::Sphere { radius: 0.5 }
    }
}

impl SpawnShape {
    pub fn name(&self) -> &'static str {
        match self {
            SpawnShape::Cube { .. } => "Cube",
            SpawnShape::Sphere { .. } => "Sphere",
            SpawnShape::Shell { .. } => "Shell",
            SpawnShape::Ring { .. } => "Ring",
            SpawnShape::Point => "Point",
            SpawnShape::Line { .. } => "Line",
            SpawnShape::Plane { .. } => "Plane",
        }
    }

    pub fn variants() -> &'static [&'static str] {
        &["Cube", "Sphere", "Shell", "Ring", "Point", "Line", "Plane"]
    }
}

/// Initial velocity configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InitialVelocity {
    Zero,
    RandomDirection { speed: f32 },
    Outward { speed: f32 },
    Inward { speed: f32 },
    Swirl { speed: f32 },
    Directional { direction: [f32; 3], speed: f32 },
}

impl Default for InitialVelocity {
    fn default() -> Self {
        InitialVelocity::RandomDirection { speed: 0.1 }
    }
}

impl InitialVelocity {
    pub fn name(&self) -> &'static str {
        match self {
            InitialVelocity::Zero => "Zero",
            InitialVelocity::RandomDirection { .. } => "Random",
            InitialVelocity::Outward { .. } => "Outward",
            InitialVelocity::Inward { .. } => "Inward",
            InitialVelocity::Swirl { .. } => "Swirl",
            InitialVelocity::Directional { .. } => "Directional",
        }
    }

    pub fn variants() -> &'static [&'static str] {
        &["Zero", "Random", "Outward", "Inward", "Swirl", "Directional"]
    }
}

/// How to assign particle colors
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ColorMode {
    Uniform { r: f32, g: f32, b: f32 },
    RandomHue { saturation: f32, value: f32 },
    ByPosition,
    ByVelocity,
    Gradient { start: [f32; 3], end: [f32; 3] },
}

impl Default for ColorMode {
    fn default() -> Self {
        ColorMode::RandomHue {
            saturation: 0.8,
            value: 0.9,
        }
    }
}

impl ColorMode {
    pub fn name(&self) -> &'static str {
        match self {
            ColorMode::Uniform { .. } => "Uniform",
            ColorMode::RandomHue { .. } => "Random Hue",
            ColorMode::ByPosition => "By Position",
            ColorMode::ByVelocity => "By Velocity",
            ColorMode::Gradient { .. } => "Gradient",
        }
    }

    pub fn variants() -> &'static [&'static str] {
        &["Uniform", "Random Hue", "By Position", "By Velocity", "Gradient"]
    }
}

/// Configuration for spawning particles
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpawnConfig {
    pub shape: SpawnShape,
    pub velocity: InitialVelocity,
    pub mass_range: (f32, f32),
    pub energy_range: (f32, f32),
    pub color_mode: ColorMode,
    /// Spawn weight for each particle type.
    /// Index = particle_type, value = relative weight.
    /// Empty or `[1.0]` means all particles are type 0.
    /// `[0.8, 0.2]` means 80% type 0, 20% type 1.
    #[serde(default)]
    pub type_weights: Vec<f32>,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            shape: SpawnShape::default(),
            velocity: InitialVelocity::default(),
            mass_range: (1.0, 1.0),
            energy_range: (1.0, 1.0),
            color_mode: ColorMode::default(),
            type_weights: vec![1.0], // All type 0 by default
        }
    }
}

/// Falloff function for distance-based effects
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Falloff {
    Constant,
    Linear,
    Inverse,
    InverseSquare,
    Smooth,
}

impl Default for Falloff {
    fn default() -> Self {
        Falloff::InverseSquare
    }
}

impl Falloff {
    pub fn variants() -> &'static [&'static str] {
        &["Constant", "Linear", "Inverse", "InverseSquare", "Smooth"]
    }
}

/// Blend mode for particle rendering
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BlendModeConfig {
    #[default]
    Alpha,
    Additive,
    Multiply,
}

impl BlendModeConfig {
    pub fn variants() -> &'static [&'static str] {
        &["Alpha", "Additive", "Multiply"]
    }

    pub fn to_blend_mode(&self) -> rdpe::BlendMode {
        match self {
            BlendModeConfig::Alpha => rdpe::BlendMode::Alpha,
            BlendModeConfig::Additive => rdpe::BlendMode::Additive,
            BlendModeConfig::Multiply => rdpe::BlendMode::Multiply,
        }
    }
}

/// Particle shape for rendering
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ParticleShapeConfig {
    #[default]
    Circle,
    CircleHard,
    Square,
    Ring,
    Star,
    Triangle,
    Hexagon,
    Diamond,
    Point,
}

impl ParticleShapeConfig {
    pub fn variants() -> &'static [&'static str] {
        &["Circle", "CircleHard", "Square", "Ring", "Star", "Triangle", "Hexagon", "Diamond", "Point"]
    }

    pub fn to_shape(&self) -> rdpe::ParticleShape {
        match self {
            ParticleShapeConfig::Circle => rdpe::ParticleShape::Circle,
            ParticleShapeConfig::CircleHard => rdpe::ParticleShape::CircleHard,
            ParticleShapeConfig::Square => rdpe::ParticleShape::Square,
            ParticleShapeConfig::Ring => rdpe::ParticleShape::Ring,
            ParticleShapeConfig::Star => rdpe::ParticleShape::Star,
            ParticleShapeConfig::Triangle => rdpe::ParticleShape::Triangle,
            ParticleShapeConfig::Hexagon => rdpe::ParticleShape::Hexagon,
            ParticleShapeConfig::Diamond => rdpe::ParticleShape::Diamond,
            ParticleShapeConfig::Point => rdpe::ParticleShape::Point,
        }
    }
}

/// Color palette for automatic coloring
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PaletteConfig {
    #[default]
    None,
    Viridis,
    Magma,
    Plasma,
    Inferno,
    Rainbow,
    Sunset,
    Ocean,
    Fire,
    Ice,
    Neon,
    Forest,
    Grayscale,
}

impl PaletteConfig {
    pub fn variants() -> &'static [&'static str] {
        &["None", "Viridis", "Magma", "Plasma", "Inferno", "Rainbow", "Sunset", "Ocean", "Fire", "Ice", "Neon", "Forest", "Grayscale"]
    }

    pub fn to_palette(&self) -> rdpe::Palette {
        match self {
            PaletteConfig::None => rdpe::Palette::None,
            PaletteConfig::Viridis => rdpe::Palette::Viridis,
            PaletteConfig::Magma => rdpe::Palette::Magma,
            PaletteConfig::Plasma => rdpe::Palette::Plasma,
            PaletteConfig::Inferno => rdpe::Palette::Inferno,
            PaletteConfig::Rainbow => rdpe::Palette::Rainbow,
            PaletteConfig::Sunset => rdpe::Palette::Sunset,
            PaletteConfig::Ocean => rdpe::Palette::Ocean,
            PaletteConfig::Fire => rdpe::Palette::Fire,
            PaletteConfig::Ice => rdpe::Palette::Ice,
            PaletteConfig::Neon => rdpe::Palette::Neon,
            PaletteConfig::Forest => rdpe::Palette::Forest,
            PaletteConfig::Grayscale => rdpe::Palette::Grayscale,
        }
    }

    /// Get the 5 color stops for this palette as Vec3 RGB values.
    pub fn colors(&self) -> [glam::Vec3; 5] {
        self.to_palette().colors()
    }
}

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

/// How to map particle properties to palette colors
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum ColorMappingConfig {
    #[default]
    None,
    Index,
    Speed { min: f32, max: f32 },
    Age { max_age: f32 },
    PositionY { min: f32, max: f32 },
    Distance { max_dist: f32 },
    Random,
}

impl ColorMappingConfig {
    pub fn name(&self) -> &'static str {
        match self {
            ColorMappingConfig::None => "None",
            ColorMappingConfig::Index => "Index",
            ColorMappingConfig::Speed { .. } => "Speed",
            ColorMappingConfig::Age { .. } => "Age",
            ColorMappingConfig::PositionY { .. } => "Position Y",
            ColorMappingConfig::Distance { .. } => "Distance",
            ColorMappingConfig::Random => "Random",
        }
    }

    pub fn variants() -> &'static [&'static str] {
        &["None", "Index", "Speed", "Age", "Position Y", "Distance", "Random"]
    }

    pub fn to_color_mapping(&self) -> rdpe::ColorMapping {
        match self {
            ColorMappingConfig::None => rdpe::ColorMapping::None,
            ColorMappingConfig::Index => rdpe::ColorMapping::Index,
            ColorMappingConfig::Speed { min, max } => rdpe::ColorMapping::Speed { min: *min, max: *max },
            ColorMappingConfig::Age { max_age } => rdpe::ColorMapping::Age { max_age: *max_age },
            ColorMappingConfig::PositionY { min, max } => rdpe::ColorMapping::PositionY { min: *min, max: *max },
            ColorMappingConfig::Distance { max_dist } => rdpe::ColorMapping::Distance { max_dist: *max_dist },
            ColorMappingConfig::Random => rdpe::ColorMapping::Random,
        }
    }
}

/// Wireframe mesh for 3D particle rendering
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum WireframeMeshConfig {
    #[default]
    None,
    Tetrahedron,
    Cube,
    Octahedron,
    Icosahedron,
}

impl WireframeMeshConfig {
    pub fn to_mesh(&self) -> Option<rdpe::WireframeMesh> {
        match self {
            WireframeMeshConfig::None => None,
            WireframeMeshConfig::Tetrahedron => Some(rdpe::WireframeMesh::tetrahedron()),
            WireframeMeshConfig::Cube => Some(rdpe::WireframeMesh::cube()),
            WireframeMeshConfig::Octahedron => Some(rdpe::WireframeMesh::octahedron()),
            WireframeMeshConfig::Icosahedron => Some(rdpe::WireframeMesh::icosahedron()),
        }
    }
}

/// Visual configuration for particle rendering
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VisualsConfig {
    pub blend_mode: BlendModeConfig,
    pub shape: ParticleShapeConfig,
    pub palette: PaletteConfig,
    pub color_mapping: ColorMappingConfig,
    pub background_color: [f32; 3],
    pub trail_length: u32,
    pub connections_enabled: bool,
    pub connections_radius: f32,
    pub velocity_stretch: bool,
    pub velocity_stretch_factor: f32,
    pub spatial_grid_opacity: f32,
    #[serde(default)]
    pub wireframe: WireframeMeshConfig,
    #[serde(default = "default_wireframe_thickness")]
    pub wireframe_thickness: f32,
}

fn default_wireframe_thickness() -> f32 {
    0.003
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            blend_mode: BlendModeConfig::Alpha,
            shape: ParticleShapeConfig::Circle,
            palette: PaletteConfig::None,
            color_mapping: ColorMappingConfig::None,
            background_color: [0.02, 0.02, 0.05],
            trail_length: 0,
            connections_enabled: false,
            connections_radius: 0.1,
            velocity_stretch: false,
            velocity_stretch_factor: 2.0,
            spatial_grid_opacity: 0.0,
            wireframe: WireframeMeshConfig::None,
            wireframe_thickness: 0.003,
        }
    }
}

/// Vertex effect configuration for visual enhancements
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VertexEffectConfig {
    Rotate { speed: f32 },
    Wobble { frequency: f32, amplitude: f32 },
    Pulse { frequency: f32, amplitude: f32 },
    Wave { direction: [f32; 3], frequency: f32, speed: f32, amplitude: f32 },
    Jitter { amplitude: f32 },
    StretchToVelocity { max_stretch: f32 },
    ScaleByDistance { center: [f32; 3], min_scale: f32, max_scale: f32, max_distance: f32 },
    FadeByDistance { near: f32, far: f32 },
    BillboardCylindrical { axis: [f32; 3] },
    BillboardFixed { forward: [f32; 3], up: [f32; 3] },
    FacePoint { target: [f32; 3] },
}

impl VertexEffectConfig {
    pub fn name(&self) -> &'static str {
        match self {
            VertexEffectConfig::Rotate { .. } => "Rotate",
            VertexEffectConfig::Wobble { .. } => "Wobble",
            VertexEffectConfig::Pulse { .. } => "Pulse",
            VertexEffectConfig::Wave { .. } => "Wave",
            VertexEffectConfig::Jitter { .. } => "Jitter",
            VertexEffectConfig::StretchToVelocity { .. } => "Stretch To Velocity",
            VertexEffectConfig::ScaleByDistance { .. } => "Scale By Distance",
            VertexEffectConfig::FadeByDistance { .. } => "Fade By Distance",
            VertexEffectConfig::BillboardCylindrical { .. } => "Billboard Cylindrical",
            VertexEffectConfig::BillboardFixed { .. } => "Billboard Fixed",
            VertexEffectConfig::FacePoint { .. } => "Face Point",
        }
    }

    pub fn to_effect(&self) -> rdpe::VertexEffect {
        use rdpe::VertexEffect;
        match self {
            VertexEffectConfig::Rotate { speed } => VertexEffect::Rotate { speed: *speed },
            VertexEffectConfig::Wobble { frequency, amplitude } => VertexEffect::Wobble {
                frequency: *frequency,
                amplitude: *amplitude,
            },
            VertexEffectConfig::Pulse { frequency, amplitude } => VertexEffect::Pulse {
                frequency: *frequency,
                amplitude: *amplitude,
            },
            VertexEffectConfig::Wave { direction, frequency, speed, amplitude } => VertexEffect::Wave {
                direction: Vec3::from_array(*direction),
                frequency: *frequency,
                speed: *speed,
                amplitude: *amplitude,
            },
            VertexEffectConfig::Jitter { amplitude } => VertexEffect::Jitter { amplitude: *amplitude },
            VertexEffectConfig::StretchToVelocity { max_stretch } => VertexEffect::StretchToVelocity {
                max_stretch: *max_stretch,
            },
            VertexEffectConfig::ScaleByDistance { center, min_scale, max_scale, max_distance } => {
                VertexEffect::ScaleByDistance {
                    center: Vec3::from_array(*center),
                    min_scale: *min_scale,
                    max_scale: *max_scale,
                    max_distance: *max_distance,
                }
            }
            VertexEffectConfig::FadeByDistance { near, far } => VertexEffect::FadeByDistance {
                near: *near,
                far: *far,
            },
            VertexEffectConfig::BillboardCylindrical { axis } => VertexEffect::BillboardCylindrical {
                axis: Vec3::from_array(*axis),
            },
            VertexEffectConfig::BillboardFixed { forward, up } => VertexEffect::BillboardFixed {
                forward: Vec3::from_array(*forward),
                up: Vec3::from_array(*up),
            },
            VertexEffectConfig::FacePoint { target } => VertexEffect::FacePoint {
                target: Vec3::from_array(*target),
            },
        }
    }
}

/// Serializable rule configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RuleConfig {
    // === Basic Forces ===
    Gravity(f32),
    Drag(f32),
    Acceleration { direction: [f32; 3] },

    // === Boundaries ===
    BounceWalls,
    WrapWalls,

    // === Point Forces ===
    AttractTo { point: [f32; 3], strength: f32 },
    RepelFrom { point: [f32; 3], strength: f32, radius: f32 },
    PointGravity { point: [f32; 3], strength: f32, softening: f32 },
    Orbit { center: [f32; 3], strength: f32 },
    Spring { anchor: [f32; 3], stiffness: f32, damping: f32 },
    Radial { point: [f32; 3], strength: f32, radius: f32, falloff: Falloff },
    Vortex { center: [f32; 3], axis: [f32; 3], strength: f32 },
    Pulse { point: [f32; 3], strength: f32, frequency: f32, radius: f32 },

    // === Noise & Flow ===
    Turbulence { scale: f32, strength: f32 },
    Curl { scale: f32, strength: f32 },
    Wind { direction: [f32; 3], strength: f32, turbulence: f32 },
    PositionNoise { scale: f32, strength: f32, speed: f32 },

    // === Steering ===
    Seek { target: [f32; 3], max_speed: f32, max_force: f32 },
    Flee { target: [f32; 3], max_speed: f32, max_force: f32, panic_radius: f32 },
    Arrive { target: [f32; 3], max_speed: f32, max_force: f32, slowing_radius: f32 },
    Wander { strength: f32, frequency: f32 },

    // === Boids / Flocking ===
    Separate { radius: f32, strength: f32 },
    Cohere { radius: f32, strength: f32 },
    Align { radius: f32, strength: f32 },
    Flock { radius: f32, separation: f32, cohesion: f32, alignment: f32 },
    Avoid { radius: f32, strength: f32 },

    // === Physics ===
    Collide { radius: f32, restitution: f32 },
    NBodyGravity { strength: f32, softening: f32, radius: f32 },
    LennardJones { epsilon: f32, sigma: f32, cutoff: f32 },
    Viscosity { radius: f32, strength: f32 },
    Pressure { radius: f32, strength: f32, target_density: f32 },
    SurfaceTension { radius: f32, strength: f32, threshold: f32 },
    Magnetism { radius: f32, strength: f32, same_repel: bool },

    // === Constraints ===
    SpeedLimit { min: f32, max: f32 },
    Buoyancy { surface_y: f32, density: f32 },
    Friction { ground_y: f32, strength: f32, threshold: f32 },

    // === Lifecycle ===
    Age,
    Lifetime(f32),
    FadeOut(f32),
    ShrinkOut(f32),
    ColorOverLife { start: [f32; 3], end: [f32; 3], duration: f32 },
    ColorBySpeed { slow_color: [f32; 3], fast_color: [f32; 3], max_speed: f32 },
    ColorByAge { young_color: [f32; 3], old_color: [f32; 3], max_age: f32 },
    ScaleBySpeed { min_scale: f32, max_scale: f32, max_speed: f32 },

    // === Typed Interactions ===
    Chase { self_type: u32, target_type: u32, radius: f32, strength: f32 },
    Evade { self_type: u32, threat_type: u32, radius: f32, strength: f32 },
    Convert { from_type: u32, trigger_type: u32, to_type: u32, radius: f32, probability: f32 },

    // === Events ===
    Shockwave { origin: [f32; 3], speed: f32, width: f32, strength: f32, repeat: f32 },
    Oscillate { axis: [f32; 3], amplitude: f32, frequency: f32, spatial_scale: f32 },
    RespawnBelow { threshold_y: f32, spawn_y: f32, reset_velocity: bool },

    // === Conditional ===
    Maybe { probability: f32, action: String },
    Trigger { condition: String, action: String },

    // === Custom WGSL ===
    Custom { code: String },
    NeighborCustom { code: String },
    OnCollision { radius: f32, response: String },
    CustomDynamic { code: String, params: Vec<(String, f32)> },
    NeighborCustomDynamic { code: String, params: Vec<(String, f32)> },

    // === Event Hooks ===
    OnCondition { condition: String, action: String },
    OnDeath { action: String },
    OnInterval { interval: f32, action: String },
    OnSpawn { action: String },

    // === Growth & Decay ===
    Grow { rate: f32, min: f32, max: f32 },
    Decay { field: String, rate: f32 },
    Die { condition: String },
    DLA { seed_type: u32, mobile_type: u32, stick_radius: f32, diffusion_strength: f32 },

    // === Field Operations ===
    CopyField { from: String, to: String },
    Current { field: String, strength: f32 },

    // === Math / Signal ===
    Lerp { field: String, target: f32, rate: f32 },
    Clamp { field: String, min: f32, max: f32 },
    Remap { field: String, in_min: f32, in_max: f32, out_min: f32, out_max: f32 },
    Quantize { field: String, step: f32 },
    Noise { field: String, amplitude: f32, frequency: f32 },
}

impl RuleConfig {
    pub fn name(&self) -> &'static str {
        match self {
            // Basic Forces
            RuleConfig::Gravity(_) => "Gravity",
            RuleConfig::Drag(_) => "Drag",
            RuleConfig::Acceleration { .. } => "Acceleration",
            // Boundaries
            RuleConfig::BounceWalls => "Bounce Walls",
            RuleConfig::WrapWalls => "Wrap Walls",
            // Point Forces
            RuleConfig::AttractTo { .. } => "Attract To",
            RuleConfig::RepelFrom { .. } => "Repel From",
            RuleConfig::PointGravity { .. } => "Point Gravity",
            RuleConfig::Orbit { .. } => "Orbit",
            RuleConfig::Spring { .. } => "Spring",
            RuleConfig::Radial { .. } => "Radial",
            RuleConfig::Vortex { .. } => "Vortex",
            RuleConfig::Pulse { .. } => "Pulse",
            // Noise & Flow
            RuleConfig::Turbulence { .. } => "Turbulence",
            RuleConfig::Curl { .. } => "Curl",
            RuleConfig::Wind { .. } => "Wind",
            RuleConfig::PositionNoise { .. } => "Position Noise",
            // Steering
            RuleConfig::Seek { .. } => "Seek",
            RuleConfig::Flee { .. } => "Flee",
            RuleConfig::Arrive { .. } => "Arrive",
            RuleConfig::Wander { .. } => "Wander",
            // Boids
            RuleConfig::Separate { .. } => "Separate",
            RuleConfig::Cohere { .. } => "Cohere",
            RuleConfig::Align { .. } => "Align",
            RuleConfig::Flock { .. } => "Flock",
            RuleConfig::Avoid { .. } => "Avoid",
            // Physics
            RuleConfig::Collide { .. } => "Collide",
            RuleConfig::NBodyGravity { .. } => "N-Body Gravity",
            RuleConfig::LennardJones { .. } => "Lennard-Jones",
            RuleConfig::Viscosity { .. } => "Viscosity",
            RuleConfig::Pressure { .. } => "Pressure",
            RuleConfig::SurfaceTension { .. } => "Surface Tension",
            RuleConfig::Magnetism { .. } => "Magnetism",
            // Constraints
            RuleConfig::SpeedLimit { .. } => "Speed Limit",
            RuleConfig::Buoyancy { .. } => "Buoyancy",
            RuleConfig::Friction { .. } => "Friction",
            // Lifecycle
            RuleConfig::Age => "Age",
            RuleConfig::Lifetime(_) => "Lifetime",
            RuleConfig::FadeOut(_) => "Fade Out",
            RuleConfig::ShrinkOut(_) => "Shrink Out",
            RuleConfig::ColorOverLife { .. } => "Color Over Life",
            RuleConfig::ColorBySpeed { .. } => "Color By Speed",
            RuleConfig::ColorByAge { .. } => "Color By Age",
            RuleConfig::ScaleBySpeed { .. } => "Scale By Speed",
            // Typed
            RuleConfig::Chase { .. } => "Chase",
            RuleConfig::Evade { .. } => "Evade",
            RuleConfig::Convert { .. } => "Convert",
            // Events
            RuleConfig::Shockwave { .. } => "Shockwave",
            RuleConfig::Oscillate { .. } => "Oscillate",
            RuleConfig::RespawnBelow { .. } => "Respawn Below",
            // Conditional
            RuleConfig::Maybe { .. } => "Maybe",
            RuleConfig::Trigger { .. } => "Trigger",
            // Custom
            RuleConfig::Custom { .. } => "Custom WGSL",
            RuleConfig::NeighborCustom { .. } => "Neighbor Custom",
            RuleConfig::OnCollision { .. } => "On Collision",
            RuleConfig::CustomDynamic { .. } => "Custom Dynamic",
            RuleConfig::NeighborCustomDynamic { .. } => "Neighbor Custom Dynamic",
            // Event Hooks
            RuleConfig::OnCondition { .. } => "On Condition",
            RuleConfig::OnDeath { .. } => "On Death",
            RuleConfig::OnInterval { .. } => "On Interval",
            RuleConfig::OnSpawn { .. } => "On Spawn",
            // Growth & Decay
            RuleConfig::Grow { .. } => "Grow",
            RuleConfig::Decay { .. } => "Decay",
            RuleConfig::Die { .. } => "Die",
            RuleConfig::DLA { .. } => "DLA",
            // Field Operations
            RuleConfig::CopyField { .. } => "Copy Field",
            RuleConfig::Current { .. } => "Current",
            // Math / Signal
            RuleConfig::Lerp { .. } => "Lerp",
            RuleConfig::Clamp { .. } => "Clamp",
            RuleConfig::Remap { .. } => "Remap",
            RuleConfig::Quantize { .. } => "Quantize",
            RuleConfig::Noise { .. } => "Noise",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            RuleConfig::Gravity(_) | RuleConfig::Drag(_) | RuleConfig::Acceleration { .. } => "Forces",
            RuleConfig::BounceWalls | RuleConfig::WrapWalls => "Boundaries",
            RuleConfig::AttractTo { .. } | RuleConfig::RepelFrom { .. } | RuleConfig::PointGravity { .. } |
            RuleConfig::Orbit { .. } | RuleConfig::Spring { .. } | RuleConfig::Radial { .. } |
            RuleConfig::Vortex { .. } | RuleConfig::Pulse { .. } => "Point Forces",
            RuleConfig::Turbulence { .. } | RuleConfig::Curl { .. } | RuleConfig::Wind { .. } |
            RuleConfig::PositionNoise { .. } => "Noise & Flow",
            RuleConfig::Seek { .. } | RuleConfig::Flee { .. } | RuleConfig::Arrive { .. } |
            RuleConfig::Wander { .. } => "Steering",
            RuleConfig::Separate { .. } | RuleConfig::Cohere { .. } | RuleConfig::Align { .. } |
            RuleConfig::Flock { .. } | RuleConfig::Avoid { .. } => "Flocking",
            RuleConfig::Collide { .. } | RuleConfig::NBodyGravity { .. } | RuleConfig::LennardJones { .. } |
            RuleConfig::Viscosity { .. } | RuleConfig::Pressure { .. } | RuleConfig::SurfaceTension { .. } |
            RuleConfig::Magnetism { .. } => "Physics",
            RuleConfig::SpeedLimit { .. } | RuleConfig::Buoyancy { .. } | RuleConfig::Friction { .. } => "Constraints",
            RuleConfig::Age | RuleConfig::Lifetime(_) | RuleConfig::FadeOut(_) | RuleConfig::ShrinkOut(_) |
            RuleConfig::ColorOverLife { .. } | RuleConfig::ColorBySpeed { .. } | RuleConfig::ColorByAge { .. } |
            RuleConfig::ScaleBySpeed { .. } => "Lifecycle",
            RuleConfig::Chase { .. } | RuleConfig::Evade { .. } | RuleConfig::Convert { .. } => "Typed",
            RuleConfig::Shockwave { .. } | RuleConfig::Oscillate { .. } | RuleConfig::RespawnBelow { .. } => "Events",
            RuleConfig::Maybe { .. } | RuleConfig::Trigger { .. } => "Conditional",
            RuleConfig::Custom { .. } | RuleConfig::NeighborCustom { .. } | RuleConfig::OnCollision { .. } |
            RuleConfig::CustomDynamic { .. } => "Custom",
            RuleConfig::NeighborCustomDynamic { .. } => "Custom",
            // New categories
            RuleConfig::OnCondition { .. } | RuleConfig::OnDeath { .. } | RuleConfig::OnInterval { .. } |
            RuleConfig::OnSpawn { .. } => "Event Hooks",
            RuleConfig::Grow { .. } | RuleConfig::Decay { .. } | RuleConfig::Die { .. } |
            RuleConfig::DLA { .. } => "Growth & Decay",
            RuleConfig::CopyField { .. } | RuleConfig::Current { .. } => "Fields",
            RuleConfig::Lerp { .. } | RuleConfig::Clamp { .. } | RuleConfig::Remap { .. } |
            RuleConfig::Quantize { .. } | RuleConfig::Noise { .. } => "Math",
        }
    }

    /// Convert to rdpe::Rule
    pub fn to_rule(&self) -> rdpe::Rule {
        use rdpe::Rule;
        match self {
            RuleConfig::Gravity(g) => Rule::Gravity(*g),
            RuleConfig::Drag(d) => Rule::Drag(*d),
            RuleConfig::Acceleration { direction } => Rule::Acceleration(Vec3::from_array(*direction)),
            RuleConfig::BounceWalls => Rule::BounceWalls,
            RuleConfig::WrapWalls => Rule::WrapWalls,
            RuleConfig::AttractTo { point, strength } => Rule::AttractTo {
                point: Vec3::from_array(*point),
                strength: *strength,
            },
            RuleConfig::RepelFrom { point, strength, radius } => Rule::RepelFrom {
                point: Vec3::from_array(*point),
                strength: *strength,
                radius: *radius,
            },
            RuleConfig::PointGravity { point, strength, softening } => Rule::PointGravity {
                point: Vec3::from_array(*point),
                strength: *strength,
                softening: *softening,
            },
            RuleConfig::Orbit { center, strength } => Rule::Orbit {
                center: Vec3::from_array(*center),
                strength: *strength,
            },
            RuleConfig::Spring { anchor, stiffness, damping } => Rule::Spring {
                anchor: Vec3::from_array(*anchor),
                stiffness: *stiffness,
                damping: *damping,
            },
            RuleConfig::Radial { point, strength, radius, falloff } => Rule::Radial {
                point: Vec3::from_array(*point),
                strength: *strength,
                radius: *radius,
                falloff: match falloff {
                    Falloff::Constant => rdpe::Falloff::Constant,
                    Falloff::Linear => rdpe::Falloff::Linear,
                    Falloff::Inverse => rdpe::Falloff::Inverse,
                    Falloff::InverseSquare => rdpe::Falloff::InverseSquare,
                    Falloff::Smooth => rdpe::Falloff::Smooth,
                },
            },
            RuleConfig::Vortex { center, axis, strength } => Rule::Vortex {
                center: Vec3::from_array(*center),
                axis: Vec3::from_array(*axis),
                strength: *strength,
            },
            RuleConfig::Pulse { point, strength, frequency, radius } => Rule::Pulse {
                point: Vec3::from_array(*point),
                strength: *strength,
                frequency: *frequency,
                radius: *radius,
            },
            RuleConfig::Turbulence { scale, strength } => Rule::Turbulence {
                scale: *scale,
                strength: *strength,
            },
            RuleConfig::Curl { scale, strength } => Rule::Curl {
                scale: *scale,
                strength: *strength,
            },
            RuleConfig::Wind { direction, strength, turbulence } => Rule::Wind {
                direction: Vec3::from_array(*direction),
                strength: *strength,
                turbulence: *turbulence,
            },
            RuleConfig::PositionNoise { scale, strength, speed } => Rule::PositionNoise {
                scale: *scale,
                strength: *strength,
                speed: *speed,
            },
            RuleConfig::Seek { target, max_speed, max_force } => Rule::Seek {
                target: Vec3::from_array(*target),
                max_speed: *max_speed,
                max_force: *max_force,
            },
            RuleConfig::Flee { target, max_speed, max_force, panic_radius } => Rule::Flee {
                target: Vec3::from_array(*target),
                max_speed: *max_speed,
                max_force: *max_force,
                panic_radius: *panic_radius,
            },
            RuleConfig::Arrive { target, max_speed, max_force, slowing_radius } => Rule::Arrive {
                target: Vec3::from_array(*target),
                max_speed: *max_speed,
                max_force: *max_force,
                slowing_radius: *slowing_radius,
            },
            RuleConfig::Wander { strength, frequency } => Rule::Wander {
                strength: *strength,
                frequency: *frequency,
            },
            RuleConfig::Separate { radius, strength } => Rule::Separate {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Cohere { radius, strength } => Rule::Cohere {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Align { radius, strength } => Rule::Align {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Flock { radius, separation, cohesion, alignment } => Rule::Flock {
                radius: *radius,
                separation: *separation,
                cohesion: *cohesion,
                alignment: *alignment,
            },
            RuleConfig::Avoid { radius, strength } => Rule::Avoid {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Collide { radius, restitution } => Rule::Collide {
                radius: *radius,
                restitution: *restitution,
            },
            RuleConfig::NBodyGravity { strength, softening, radius } => Rule::NBodyGravity {
                strength: *strength,
                softening: *softening,
                radius: *radius,
            },
            RuleConfig::LennardJones { epsilon, sigma, cutoff } => Rule::LennardJones {
                epsilon: *epsilon,
                sigma: *sigma,
                cutoff: *cutoff,
            },
            RuleConfig::Viscosity { radius, strength } => Rule::Viscosity {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Pressure { radius, strength, target_density } => Rule::Pressure {
                radius: *radius,
                strength: *strength,
                target_density: *target_density,
            },
            RuleConfig::SurfaceTension { radius, strength, threshold } => Rule::SurfaceTension {
                radius: *radius,
                strength: *strength,
                threshold: *threshold,
            },
            RuleConfig::Magnetism { radius, strength, same_repel } => Rule::Magnetism {
                radius: *radius,
                strength: *strength,
                same_repel: *same_repel,
            },
            RuleConfig::SpeedLimit { min, max } => Rule::SpeedLimit {
                min: *min,
                max: *max,
            },
            RuleConfig::Buoyancy { surface_y, density } => Rule::Buoyancy {
                surface_y: *surface_y,
                density: *density,
            },
            RuleConfig::Friction { ground_y, strength, threshold } => Rule::Friction {
                ground_y: *ground_y,
                strength: *strength,
                threshold: *threshold,
            },
            RuleConfig::Age => Rule::Age,
            RuleConfig::Lifetime(t) => Rule::Lifetime(*t),
            RuleConfig::FadeOut(t) => Rule::FadeOut(*t),
            RuleConfig::ShrinkOut(t) => Rule::ShrinkOut(*t),
            RuleConfig::ColorOverLife { start, end, duration } => Rule::ColorOverLife {
                start: Vec3::from_array(*start),
                end: Vec3::from_array(*end),
                duration: *duration,
            },
            RuleConfig::ColorBySpeed { slow_color, fast_color, max_speed } => Rule::ColorBySpeed {
                slow_color: Vec3::from_array(*slow_color),
                fast_color: Vec3::from_array(*fast_color),
                max_speed: *max_speed,
            },
            RuleConfig::ColorByAge { young_color, old_color, max_age } => Rule::ColorByAge {
                young_color: Vec3::from_array(*young_color),
                old_color: Vec3::from_array(*old_color),
                max_age: *max_age,
            },
            RuleConfig::ScaleBySpeed { min_scale, max_scale, max_speed } => Rule::ScaleBySpeed {
                min_scale: *min_scale,
                max_scale: *max_scale,
                max_speed: *max_speed,
            },
            RuleConfig::Chase { self_type, target_type, radius, strength } => Rule::Chase {
                self_type: *self_type,
                target_type: *target_type,
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Evade { self_type, threat_type, radius, strength } => Rule::Evade {
                self_type: *self_type,
                threat_type: *threat_type,
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Convert { from_type, trigger_type, to_type, radius, probability } => Rule::Convert {
                from_type: *from_type,
                trigger_type: *trigger_type,
                to_type: *to_type,
                radius: *radius,
                probability: *probability,
            },
            RuleConfig::Shockwave { origin, speed, width, strength, repeat } => Rule::Shockwave {
                origin: Vec3::from_array(*origin),
                speed: *speed,
                width: *width,
                strength: *strength,
                repeat: *repeat,
            },
            RuleConfig::Oscillate { axis, amplitude, frequency, spatial_scale } => Rule::Oscillate {
                axis: Vec3::from_array(*axis),
                amplitude: *amplitude,
                frequency: *frequency,
                spatial_scale: *spatial_scale,
            },
            RuleConfig::RespawnBelow { threshold_y, spawn_y, reset_velocity } => Rule::RespawnBelow {
                threshold_y: *threshold_y,
                spawn_y: *spawn_y,
                reset_velocity: *reset_velocity,
            },
            RuleConfig::Maybe { probability, action } => Rule::Maybe {
                probability: *probability,
                action: action.clone(),
            },
            RuleConfig::Trigger { condition, action } => Rule::Trigger {
                condition: condition.clone(),
                action: action.clone(),
            },
            RuleConfig::Custom { code } => Rule::Custom(code.clone()),
            RuleConfig::NeighborCustom { code } => Rule::NeighborCustom(code.clone()),
            RuleConfig::OnCollision { radius, response } => Rule::OnCollision {
                radius: *radius,
                response: response.clone(),
            },
            RuleConfig::CustomDynamic { code, params } => {
                let mut builder = Rule::custom_dynamic(code.clone());
                for (name, value) in params {
                    builder = builder.with_param(name, *value);
                }
                builder.into()
            }
            RuleConfig::NeighborCustomDynamic { code, params } => {
                let mut builder = Rule::neighbor_custom_dynamic(code.clone());
                for (name, value) in params {
                    builder = builder.with_param(name, *value);
                }
                builder.into()
            }
            // Event Hooks
            RuleConfig::OnCondition { condition, action } => Rule::OnCondition {
                condition: condition.clone(),
                action: action.clone(),
            },
            RuleConfig::OnDeath { action } => Rule::OnDeath {
                action: action.clone(),
            },
            RuleConfig::OnInterval { interval, action } => Rule::OnInterval {
                interval: *interval,
                action: action.clone(),
            },
            RuleConfig::OnSpawn { action } => Rule::OnSpawn {
                action: action.clone(),
            },
            // Growth & Decay
            RuleConfig::Grow { rate, min, max } => Rule::Grow {
                rate: *rate,
                min: *min,
                max: *max,
            },
            RuleConfig::Decay { field, rate } => Rule::Decay {
                field: field.clone(),
                rate: *rate,
            },
            RuleConfig::Die { condition } => Rule::Die {
                condition: condition.clone(),
                field: "alive".into(),
            },
            RuleConfig::DLA { seed_type, mobile_type, stick_radius, diffusion_strength } => Rule::DLA {
                seed_type: *seed_type,
                mobile_type: *mobile_type,
                stick_radius: *stick_radius,
                diffusion_strength: *diffusion_strength,
            },
            // Field Operations
            RuleConfig::CopyField { from, to } => Rule::CopyField {
                from: from.clone(),
                to: to.clone(),
            },
            RuleConfig::Current { field, strength } => Rule::Current {
                field: Box::leak(field.clone().into_boxed_str()),
                strength: *strength,
            },
            // Math / Signal
            RuleConfig::Lerp { field, target, rate } => Rule::Lerp {
                field: field.clone(),
                target: *target,
                rate: *rate,
            },
            RuleConfig::Clamp { field, min, max } => Rule::Clamp {
                field: field.clone(),
                min: *min,
                max: *max,
            },
            RuleConfig::Remap { field, in_min, in_max, out_min, out_max } => Rule::Remap {
                field: field.clone(),
                in_min: *in_min,
                in_max: *in_max,
                out_min: *out_min,
                out_max: *out_max,
            },
            RuleConfig::Quantize { field, step } => Rule::Quantize {
                field: field.clone(),
                step: *step,
            },
            RuleConfig::Noise { field, amplitude, frequency } => Rule::Noise {
                field: field.clone(),
                amplitude: *amplitude,
                frequency: *frequency,
                time_scale: 1.0,
            },
        }
    }

    /// Check if this rule requires spatial hashing
    pub fn requires_neighbors(&self) -> bool {
        matches!(self,
            RuleConfig::Separate { .. } | RuleConfig::Cohere { .. } | RuleConfig::Align { .. } |
            RuleConfig::Flock { .. } | RuleConfig::Avoid { .. } | RuleConfig::Collide { .. } |
            RuleConfig::NBodyGravity { .. } | RuleConfig::LennardJones { .. } |
            RuleConfig::Viscosity { .. } | RuleConfig::Pressure { .. } |
            RuleConfig::SurfaceTension { .. } | RuleConfig::Magnetism { .. } |
            RuleConfig::Chase { .. } | RuleConfig::Evade { .. } | RuleConfig::Convert { .. } |
            RuleConfig::NeighborCustom { .. } | RuleConfig::OnCollision { .. } |
            RuleConfig::DLA { .. }
        )
    }

    /// Generate custom neighbor WGSL for rules that need editor-specific handling.
    ///
    /// Returns `Some(wgsl)` if this rule needs custom code generation in the editor,
    /// `None` to use the core library's default implementation.
    ///
    /// Note: `particle_type` is now a base field, always present.
    pub fn to_neighbor_wgsl(&self) -> Option<String> {
        match self {
            RuleConfig::Magnetism { radius, strength, same_repel } => {
                let same_sign = if *same_repel { "1.0" } else { "-1.0" };
                Some(format!(
                    r#"            // Magnetism
            if neighbor_dist < {radius} && neighbor_dist > 0.001 {{
                let same_type = select(-1.0, 1.0, p.particle_type == other.particle_type);
                let force_dir = same_type * {same_sign}; // +1 = repel, -1 = attract
                let falloff = 1.0 - neighbor_dist / {radius};
                p.velocity += neighbor_dir * force_dir * falloff * {strength} * uniforms.delta_time;
            }}"#
                ))
            }

            RuleConfig::Chase { self_type, target_type, radius, .. } => {
                Some(format!(
                    r#"            // Chase: track nearest target
            if p.particle_type == {self_type}u && other.particle_type == {target_type}u && neighbor_dist < {radius} {{
                if neighbor_dist < chase_nearest_dist {{
                    chase_nearest_dist = neighbor_dist;
                    chase_nearest_pos = neighbor_pos;
                }}
            }}"#
                ))
            }

            RuleConfig::Evade { self_type, threat_type, radius, .. } => {
                Some(format!(
                    r#"            // Evade: track nearest threat
            if p.particle_type == {self_type}u && other.particle_type == {threat_type}u && neighbor_dist < {radius} {{
                if neighbor_dist < evade_nearest_dist {{
                    evade_nearest_dist = neighbor_dist;
                    evade_nearest_pos = neighbor_pos;
                }}
            }}"#
                ))
            }

            RuleConfig::Convert { from_type, trigger_type, to_type, radius, probability } => {
                Some(format!(
                    r#"            // Convert type {from_type} -> {to_type} (triggered by {trigger_type})
            if p.particle_type == {from_type}u && other.particle_type == {trigger_type}u && neighbor_dist < {radius} {{
                let hash_input = index ^ (other_idx * 1103515245u) ^ u32(uniforms.time * 1000.0);
                let hash = (hash_input ^ (hash_input >> 16u)) * 0x45d9f3bu;
                let rand = f32(hash & 0xFFFFu) / 65535.0;
                if rand < {probability} {{
                    p.particle_type = {to_type}u;
                }}
            }}"#
                ))
            }

            RuleConfig::DLA { seed_type, mobile_type, stick_radius, diffusion_strength } => {
                Some(format!(
                    r#"            // Diffusion-Limited Aggregation
            if p.particle_type == {mobile_type}u && other.particle_type == {seed_type}u {{
                if neighbor_dist < {stick_radius} {{
                    p.particle_type = {seed_type}u;
                    p.velocity = vec3<f32>(0.0, 0.0, 0.0);
                }}
            }}
            if p.particle_type == {mobile_type}u {{
                let diff_seed = index * 1103515245u + u32(uniforms.time * 1000.0);
                let hx = (diff_seed ^ (diff_seed >> 15u)) * 0x45d9f3bu;
                let hy = ((diff_seed + 1u) ^ ((diff_seed + 1u) >> 15u)) * 0x45d9f3bu;
                let hz = ((diff_seed + 2u) ^ ((diff_seed + 2u) >> 15u)) * 0x45d9f3bu;
                let diff_force = vec3<f32>(
                    f32(hx & 0xFFFFu) / 32768.0 - 1.0,
                    f32(hy & 0xFFFFu) / 32768.0 - 1.0,
                    f32(hz & 0xFFFFu) / 32768.0 - 1.0
                );
                p.velocity += diff_force * {diffusion_strength} * uniforms.delta_time;
            }}"#
                ))
            }

            _ => None,
        }
    }

    /// Generate custom post-neighbor WGSL for rules that need editor-specific handling.
    ///
    /// Returns `Some(wgsl)` if this rule needs custom post-neighbor code,
    /// `None` to use the core library's default implementation.
    pub fn to_post_neighbor_wgsl(&self) -> Option<String> {
        match self {
            RuleConfig::Chase { self_type, strength, .. } => {
                Some(format!(
                    r#"    // Apply chase steering
    if p.particle_type == {self_type}u && chase_nearest_dist < 1000.0 {{
        let to_target = chase_nearest_pos - p.position;
        let dist = length(to_target);
        if dist > 0.001 {{
            p.velocity += normalize(to_target) * {strength} * uniforms.delta_time;
        }}
    }}"#
                ))
            }

            RuleConfig::Evade { self_type, strength, .. } => {
                Some(format!(
                    r#"    // Apply evade steering
    if p.particle_type == {self_type}u && evade_nearest_dist < 1000.0 {{
        let away_from_threat = p.position - evade_nearest_pos;
        let dist = length(away_from_threat);
        if dist > 0.001 {{
            p.velocity += normalize(away_from_threat) * {strength} * uniforms.delta_time;
        }}
    }}"#
                ))
            }

            _ => None,
        }
    }
}

/// Complete simulation configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SimConfig {
    pub name: String,
    pub particle_count: u32,
    pub bounds: f32,
    pub particle_size: f32,
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
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            particle_count: 5000,
            bounds: 1.0,
            particle_size: 0.015,
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
        *offset = (*offset + alignment - 1) / alignment * alignment;

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
        let stride = (offset + max_alignment - 1) / max_alignment * max_alignment;

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

impl SimConfig {
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let json = fs::read_to_string(path)?;
        let config = serde_json::from_str(&json)?;
        Ok(config)
    }

    /// Check if any rules require spatial hashing
    pub fn needs_spatial(&self) -> bool {
        self.rules.iter().any(|r| r.requires_neighbors())
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
