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
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// Custom shader code configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnConfig {
    pub shape: SpawnShape,
    pub velocity: InitialVelocity,
    pub mass_range: (f32, f32),
    pub energy_range: (f32, f32),
    pub color_mode: ColorMode,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            shape: SpawnShape::default(),
            velocity: InitialVelocity::default(),
            mass_range: (1.0, 1.0),
            energy_range: (1.0, 1.0),
            color_mode: ColorMode::default(),
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
}

/// Configuration for volume rendering (ray marching visualization of fields).
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
}

/// Complete simulation configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
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
        }
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
}
