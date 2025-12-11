//! Visual configuration types for particle rendering

use glam::Vec3;
use serde::{Deserialize, Serialize};

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

    /// Convert to wgpu BlendState for render pipeline.
    pub fn to_wgpu_blend_state(&self) -> wgpu::BlendState {
        match self {
            BlendModeConfig::Alpha => wgpu::BlendState::ALPHA_BLENDING,
            BlendModeConfig::Additive => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            BlendModeConfig::Multiply => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Dst,
                    dst_factor: wgpu::BlendFactor::Zero,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            },
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
    #[serde(default = "default_connections_color")]
    pub connections_color: [f32; 3],
    pub velocity_stretch: bool,
    pub velocity_stretch_factor: f32,
    pub spatial_grid_opacity: f32,
    #[serde(default)]
    pub wireframe: WireframeMeshConfig,
    #[serde(default = "default_wireframe_thickness")]
    pub wireframe_thickness: f32,
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
            connections_color: [0.5, 0.7, 1.0],
            velocity_stretch: false,
            velocity_stretch_factor: 2.0,
            spatial_grid_opacity: 0.0,
            wireframe: WireframeMeshConfig::None,
            wireframe_thickness: 0.003,
        }
    }
}

fn default_wireframe_thickness() -> f32 {
    0.003
}

fn default_connections_color() -> [f32; 3] {
    [0.5, 0.7, 1.0]  // Light blue (matches original hardcoded value)
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
