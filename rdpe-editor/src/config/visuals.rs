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
    Screen,
    Overlay,
    SoftLight,
    Subtractive,
}

impl BlendModeConfig {
    pub fn variants() -> &'static [&'static str] {
        &["Alpha", "Additive", "Multiply", "Screen", "Overlay", "SoftLight", "Subtractive"]
    }

    pub fn to_blend_mode(&self) -> rdpe::BlendMode {
        match self {
            BlendModeConfig::Alpha => rdpe::BlendMode::Alpha,
            BlendModeConfig::Additive => rdpe::BlendMode::Additive,
            BlendModeConfig::Multiply => rdpe::BlendMode::Multiply,
            BlendModeConfig::Screen => rdpe::BlendMode::Screen,
            BlendModeConfig::Overlay => rdpe::BlendMode::Overlay,
            BlendModeConfig::SoftLight => rdpe::BlendMode::SoftLight,
            BlendModeConfig::Subtractive => rdpe::BlendMode::Subtractive,
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
            BlendModeConfig::Screen => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrc,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            BlendModeConfig::Overlay => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Dst,
                    dst_factor: wgpu::BlendFactor::Src,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            },
            BlendModeConfig::SoftLight => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Dst,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            },
            BlendModeConfig::Subtractive => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::ReverseSubtract,
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
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
    Custom {
        colors: [[f32; 3]; 5],
    },
}

impl PaletteConfig {
    pub fn variants() -> &'static [&'static str] {
        &["None", "Viridis", "Magma", "Plasma", "Inferno", "Rainbow", "Sunset", "Ocean", "Fire", "Ice", "Neon", "Forest", "Grayscale", "Custom"]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PaletteConfig::None => "None",
            PaletteConfig::Viridis => "Viridis",
            PaletteConfig::Magma => "Magma",
            PaletteConfig::Plasma => "Plasma",
            PaletteConfig::Inferno => "Inferno",
            PaletteConfig::Rainbow => "Rainbow",
            PaletteConfig::Sunset => "Sunset",
            PaletteConfig::Ocean => "Ocean",
            PaletteConfig::Fire => "Fire",
            PaletteConfig::Ice => "Ice",
            PaletteConfig::Neon => "Neon",
            PaletteConfig::Forest => "Forest",
            PaletteConfig::Grayscale => "Grayscale",
            PaletteConfig::Custom { .. } => "Custom",
        }
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
            PaletteConfig::Custom { colors } => rdpe::Palette::Custom([
                Vec3::from_array(colors[0]),
                Vec3::from_array(colors[1]),
                Vec3::from_array(colors[2]),
                Vec3::from_array(colors[3]),
                Vec3::from_array(colors[4]),
            ]),
        }
    }

    /// Get the 5 color stops for this palette as Vec3 RGB values.
    pub fn colors(&self) -> [glam::Vec3; 5] {
        self.to_palette().colors()
    }

    /// Create a default custom palette (rainbow-like).
    pub fn default_custom() -> Self {
        PaletteConfig::Custom {
            colors: [
                [1.0, 0.0, 0.0],   // Red
                [1.0, 0.5, 0.0],   // Orange
                [1.0, 1.0, 0.0],   // Yellow
                [0.0, 1.0, 0.0],   // Green
                [0.0, 0.5, 1.0],   // Blue
            ],
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum WireframeMeshConfig {
    #[default]
    None,
    Tetrahedron,
    Cube,
    Octahedron,
    Icosahedron,
    Diamond,
    Axes,
    Star,
    Spiral { turns: f32, segments: u32 },
    Dodecahedron,
    Pyramid,
    Prism { sides: u32 },
    Cross,
    Sphere { rings: u32, segments: u32 },
}

impl WireframeMeshConfig {
    pub fn variants() -> &'static [&'static str] {
        &["None", "Tetrahedron", "Cube", "Octahedron", "Icosahedron", "Diamond", "Axes", "Star", "Spiral", "Dodecahedron", "Pyramid", "Prism", "Cross", "Sphere"]
    }

    pub fn name(&self) -> &'static str {
        match self {
            WireframeMeshConfig::None => "None",
            WireframeMeshConfig::Tetrahedron => "Tetrahedron",
            WireframeMeshConfig::Cube => "Cube",
            WireframeMeshConfig::Octahedron => "Octahedron",
            WireframeMeshConfig::Icosahedron => "Icosahedron",
            WireframeMeshConfig::Diamond => "Diamond",
            WireframeMeshConfig::Axes => "Axes",
            WireframeMeshConfig::Star => "Star",
            WireframeMeshConfig::Spiral { .. } => "Spiral",
            WireframeMeshConfig::Dodecahedron => "Dodecahedron",
            WireframeMeshConfig::Pyramid => "Pyramid",
            WireframeMeshConfig::Prism { .. } => "Prism",
            WireframeMeshConfig::Cross => "Cross",
            WireframeMeshConfig::Sphere { .. } => "Sphere",
        }
    }

    pub fn to_mesh(&self) -> Option<rdpe::WireframeMesh> {
        match self {
            WireframeMeshConfig::None => None,
            WireframeMeshConfig::Tetrahedron => Some(rdpe::WireframeMesh::tetrahedron()),
            WireframeMeshConfig::Cube => Some(rdpe::WireframeMesh::cube()),
            WireframeMeshConfig::Octahedron => Some(rdpe::WireframeMesh::octahedron()),
            WireframeMeshConfig::Icosahedron => Some(rdpe::WireframeMesh::icosahedron()),
            WireframeMeshConfig::Diamond => Some(rdpe::WireframeMesh::diamond()),
            WireframeMeshConfig::Axes => Some(rdpe::WireframeMesh::axes()),
            WireframeMeshConfig::Star => Some(rdpe::WireframeMesh::star()),
            WireframeMeshConfig::Spiral { turns, segments } => Some(rdpe::WireframeMesh::spiral(*turns, *segments)),
            WireframeMeshConfig::Dodecahedron => Some(Self::make_dodecahedron()),
            WireframeMeshConfig::Pyramid => Some(Self::make_pyramid()),
            WireframeMeshConfig::Prism { sides } => Some(Self::make_prism(*sides)),
            WireframeMeshConfig::Cross => Some(Self::make_cross()),
            WireframeMeshConfig::Sphere { rings, segments } => Some(Self::make_sphere(*rings, *segments)),
        }
    }

    /// Dodecahedron (12 pentagonal faces, 30 edges)
    fn make_dodecahedron() -> rdpe::WireframeMesh {
        let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let s = 0.25;

        // 20 vertices
        let vertices = [
            // Cube vertices
            glam::Vec3::new(1.0, 1.0, 1.0) * s,
            glam::Vec3::new(1.0, 1.0, -1.0) * s,
            glam::Vec3::new(1.0, -1.0, 1.0) * s,
            glam::Vec3::new(1.0, -1.0, -1.0) * s,
            glam::Vec3::new(-1.0, 1.0, 1.0) * s,
            glam::Vec3::new(-1.0, 1.0, -1.0) * s,
            glam::Vec3::new(-1.0, -1.0, 1.0) * s,
            glam::Vec3::new(-1.0, -1.0, -1.0) * s,
            // Rectangle vertices
            glam::Vec3::new(0.0, phi, 1.0/phi) * s,
            glam::Vec3::new(0.0, phi, -1.0/phi) * s,
            glam::Vec3::new(0.0, -phi, 1.0/phi) * s,
            glam::Vec3::new(0.0, -phi, -1.0/phi) * s,
            glam::Vec3::new(1.0/phi, 0.0, phi) * s,
            glam::Vec3::new(-1.0/phi, 0.0, phi) * s,
            glam::Vec3::new(1.0/phi, 0.0, -phi) * s,
            glam::Vec3::new(-1.0/phi, 0.0, -phi) * s,
            glam::Vec3::new(phi, 1.0/phi, 0.0) * s,
            glam::Vec3::new(phi, -1.0/phi, 0.0) * s,
            glam::Vec3::new(-phi, 1.0/phi, 0.0) * s,
            glam::Vec3::new(-phi, -1.0/phi, 0.0) * s,
        ];

        let edges = [
            (0, 8), (0, 12), (0, 16), (1, 9), (1, 14), (1, 16),
            (2, 10), (2, 12), (2, 17), (3, 11), (3, 14), (3, 17),
            (4, 8), (4, 13), (4, 18), (5, 9), (5, 15), (5, 18),
            (6, 10), (6, 13), (6, 19), (7, 11), (7, 15), (7, 19),
            (8, 9), (10, 11), (12, 13), (14, 15), (16, 17), (18, 19),
        ];

        rdpe::WireframeMesh::custom(
            edges.iter().map(|(i, j)| (vertices[*i], vertices[*j])).collect()
        )
    }

    /// Square pyramid (5 vertices, 8 edges)
    fn make_pyramid() -> rdpe::WireframeMesh {
        let s = 0.5;
        let h = 0.6;

        let apex = glam::Vec3::new(0.0, h, 0.0);
        let b0 = glam::Vec3::new(-s, -h/2.0, -s);
        let b1 = glam::Vec3::new(s, -h/2.0, -s);
        let b2 = glam::Vec3::new(s, -h/2.0, s);
        let b3 = glam::Vec3::new(-s, -h/2.0, s);

        rdpe::WireframeMesh::custom(vec![
            // Base
            (b0, b1), (b1, b2), (b2, b3), (b3, b0),
            // Apex to base
            (apex, b0), (apex, b1), (apex, b2), (apex, b3),
        ])
    }

    /// Prism with n sides
    fn make_prism(sides: u32) -> rdpe::WireframeMesh {
        let sides = sides.max(3);
        let s = 0.4;
        let h = 0.5;

        let mut lines = Vec::new();
        let mut top_verts = Vec::new();
        let mut bot_verts = Vec::new();

        for i in 0..sides {
            let angle = (i as f32 / sides as f32) * std::f32::consts::TAU;
            let x = angle.cos() * s;
            let z = angle.sin() * s;
            top_verts.push(glam::Vec3::new(x, h, z));
            bot_verts.push(glam::Vec3::new(x, -h, z));
        }

        for i in 0..sides as usize {
            let next = (i + 1) % sides as usize;
            // Top face
            lines.push((top_verts[i], top_verts[next]));
            // Bottom face
            lines.push((bot_verts[i], bot_verts[next]));
            // Vertical edges
            lines.push((top_verts[i], bot_verts[i]));
        }

        rdpe::WireframeMesh::custom(lines)
    }

    /// 3D cross
    fn make_cross() -> rdpe::WireframeMesh {
        let s = 0.5;
        rdpe::WireframeMesh::custom(vec![
            (glam::Vec3::new(-s, 0.0, 0.0), glam::Vec3::new(s, 0.0, 0.0)),
            (glam::Vec3::new(0.0, -s, 0.0), glam::Vec3::new(0.0, s, 0.0)),
            (glam::Vec3::new(0.0, 0.0, -s), glam::Vec3::new(0.0, 0.0, s)),
        ])
    }

    /// Wireframe sphere (UV sphere)
    fn make_sphere(rings: u32, segments: u32) -> rdpe::WireframeMesh {
        let rings = rings.max(2);
        let segments = segments.max(3);
        let r = 0.5;

        let mut lines = Vec::new();

        // Generate vertices
        let mut verts: Vec<Vec<glam::Vec3>> = Vec::new();
        for i in 0..=rings {
            let phi = (i as f32 / rings as f32) * std::f32::consts::PI;
            let mut ring = Vec::new();
            for j in 0..segments {
                let theta = (j as f32 / segments as f32) * std::f32::consts::TAU;
                ring.push(glam::Vec3::new(
                    r * phi.sin() * theta.cos(),
                    r * phi.cos(),
                    r * phi.sin() * theta.sin(),
                ));
            }
            verts.push(ring);
        }

        // Horizontal rings
        for ring in &verts {
            for j in 0..segments as usize {
                let next = (j + 1) % segments as usize;
                lines.push((ring[j], ring[next]));
            }
        }

        // Vertical lines
        for j in 0..segments as usize {
            for i in 0..rings as usize {
                lines.push((verts[i][j], verts[i + 1][j]));
            }
        }

        rdpe::WireframeMesh::custom(lines)
    }
}

/// What data source to visualize with vector glyphs
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GlyphModeConfig {
    #[default]
    None,
    VectorField { field_index: usize },
    ParticleVelocity,
}

impl GlyphModeConfig {
    pub fn name(&self) -> &'static str {
        match self {
            GlyphModeConfig::None => "None",
            GlyphModeConfig::VectorField { .. } => "Vector Field",
            GlyphModeConfig::ParticleVelocity => "Particle Velocity",
        }
    }
}

/// How to color the glyphs
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GlyphColorModeConfig {
    #[default]
    Uniform,
    ByMagnitude,
    ByDirection,
}

impl GlyphColorModeConfig {
    pub fn name(&self) -> &'static str {
        match self {
            GlyphColorModeConfig::Uniform => "Uniform",
            GlyphColorModeConfig::ByMagnitude => "By Magnitude",
            GlyphColorModeConfig::ByDirection => "By Direction",
        }
    }
}

/// Configuration for vector glyphs
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GlyphsConfig {
    pub mode: GlyphModeConfig,
    pub grid_resolution: u32,
    pub scale: f32,
    pub color_mode: GlyphColorModeConfig,
    pub color: [f32; 3],
}

impl Default for GlyphsConfig {
    fn default() -> Self {
        Self {
            mode: GlyphModeConfig::None,
            grid_resolution: 8,
            scale: 0.1,
            color_mode: GlyphColorModeConfig::Uniform,
            color: [0.0, 1.0, 0.5],
        }
    }
}

impl GlyphsConfig {
    /// Convert to the core library's GlyphConfig type.
    pub fn to_glyph_config(&self) -> rdpe::GlyphConfig {
        let mode = match self.mode {
            GlyphModeConfig::None => rdpe::GlyphMode::None,
            GlyphModeConfig::VectorField { field_index } => {
                rdpe::GlyphMode::VectorField { field_index }
            }
            GlyphModeConfig::ParticleVelocity => rdpe::GlyphMode::ParticleVelocity,
        };

        let color_mode = match self.color_mode {
            GlyphColorModeConfig::Uniform => rdpe::GlyphColorMode::Uniform,
            GlyphColorModeConfig::ByMagnitude => rdpe::GlyphColorMode::ByMagnitude,
            GlyphColorModeConfig::ByDirection => rdpe::GlyphColorMode::ByDirection,
        };

        rdpe::GlyphConfig {
            mode,
            grid_resolution: self.grid_resolution,
            scale: self.scale,
            color_mode,
            color: rdpe::Vec3::new(self.color[0], self.color[1], self.color[2]),
            thickness: 1.0, // Default thickness
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
    #[serde(default)]
    pub glyphs: GlyphsConfig,
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
            glyphs: GlyphsConfig::default(),
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
