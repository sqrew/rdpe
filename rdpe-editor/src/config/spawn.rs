//! Spawn configuration types

use serde::{Deserialize, Serialize};

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
