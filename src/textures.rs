//! Texture loading and configuration for shaders.
//!
//! This module provides texture support for particle simulations, allowing
//! custom textures to be loaded from files or generated procedurally and
//! sampled in fragment, post-process, and compute shaders.
//!
//! # Quick Start
//!
//! ```ignore
//! use rdpe::prelude::*;
//!
//! Simulation::<Particle>::new()
//!     .with_texture("noise", "assets/noise.png")
//!     .with_fragment_shader(r#"
//!         let n = textureSample(tex_noise, tex_noise_sampler, in.uv * 0.5 + 0.5);
//!         return vec4<f32>(in.color * n.r, 1.0);
//!     "#)
//!     .run();
//! ```
//!
//! # Texture Names in Shaders
//!
//! Each texture you add with `.with_texture("name", ...)` becomes available
//! in your shaders as:
//! - `tex_name` - the texture itself
//! - `tex_name_sampler` - the sampler for that texture
//!
//! # Supported Formats
//!
//! - PNG (recommended)
//! - JPEG

use std::path::Path;

/// Filter mode for texture sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    /// Smooth linear filtering (default). Good for gradients and noise.
    #[default]
    Linear,
    /// Sharp nearest-neighbor filtering. Good for pixel art.
    Nearest,
}

/// Address mode for texture wrapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddressMode {
    /// Clamp to edge color (default). Coordinates outside 0-1 use edge pixels.
    #[default]
    ClampToEdge,
    /// Repeat/tile the texture. Coordinates wrap around.
    Repeat,
    /// Mirror the texture at boundaries.
    MirrorRepeat,
}

/// Configuration for a single texture.
#[derive(Debug, Clone)]
pub struct TextureConfig {
    /// Raw RGBA pixel data (width * height * 4 bytes).
    pub data: Vec<u8>,
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// Filter mode for magnification/minification.
    pub filter: FilterMode,
    /// Address mode for UV coordinates outside 0-1.
    pub address_mode: AddressMode,
}

impl TextureConfig {
    /// Create a texture configuration from raw RGBA data.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw RGBA pixel data (4 bytes per pixel)
    /// * `width` - Texture width in pixels
    /// * `height` - Texture height in pixels
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Create a 2x2 checkerboard texture
    /// let data = vec![
    ///     255, 255, 255, 255,  // White
    ///     0, 0, 0, 255,        // Black
    ///     0, 0, 0, 255,        // Black
    ///     255, 255, 255, 255,  // White
    /// ];
    /// let tex = TextureConfig::from_rgba(data, 2, 2);
    /// ```
    pub fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
        assert_eq!(
            data.len(),
            (width * height * 4) as usize,
            "RGBA data size mismatch"
        );
        Self {
            data,
            width,
            height,
            filter: FilterMode::Linear,
            address_mode: AddressMode::ClampToEdge,
        }
    }

    /// Load a texture from an image file.
    ///
    /// Supports PNG, JPEG, GIF, BMP, ICO, TIFF, and WebP (if feature enabled).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the image file
    ///
    /// # Panics
    ///
    /// Panics if the file cannot be loaded or is not a supported format.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tex = TextureConfig::from_file("assets/noise.png");
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let img = image::open(path.as_ref())
            .unwrap_or_else(|e| panic!("Failed to load texture '{}': {}", path.as_ref().display(), e))
            .into_rgba8();
        let (width, height) = img.dimensions();
        Self {
            data: img.into_raw(),
            width,
            height,
            filter: FilterMode::Linear,
            address_mode: AddressMode::ClampToEdge,
        }
    }

    /// Set the filter mode.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tex = TextureConfig::from_file("sprite.png")
    ///     .with_filter(FilterMode::Nearest); // Pixel art style
    /// ```
    pub fn with_filter(mut self, filter: FilterMode) -> Self {
        self.filter = filter;
        self
    }

    /// Set the address mode for UV wrapping.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tex = TextureConfig::from_file("tile.png")
    ///     .with_address_mode(AddressMode::Repeat); // Tile the texture
    /// ```
    pub fn with_address_mode(mut self, mode: AddressMode) -> Self {
        self.address_mode = mode;
        self
    }

    /// Create a solid color texture (1x1 pixel).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let red = TextureConfig::solid(255, 0, 0, 255);
    /// ```
    pub fn solid(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            data: vec![r, g, b, a],
            width: 1,
            height: 1,
            filter: FilterMode::Nearest,
            address_mode: AddressMode::ClampToEdge,
        }
    }

    /// Create a gradient texture.
    ///
    /// Creates a horizontal gradient from `start` color to `end` color.
    ///
    /// # Arguments
    ///
    /// * `width` - Width of the gradient texture
    /// * `start` - Starting color (RGBA)
    /// * `end` - Ending color (RGBA)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Fire gradient LUT
    /// let gradient = TextureConfig::gradient(256, [0, 0, 0, 255], [255, 200, 50, 255]);
    /// ```
    pub fn gradient(width: u32, start: [u8; 4], end: [u8; 4]) -> Self {
        let mut data = Vec::with_capacity((width * 4) as usize);
        for x in 0..width {
            let t = x as f32 / (width - 1).max(1) as f32;
            data.push(lerp_u8(start[0], end[0], t));
            data.push(lerp_u8(start[1], end[1], t));
            data.push(lerp_u8(start[2], end[2], t));
            data.push(lerp_u8(start[3], end[3], t));
        }
        Self {
            data,
            width,
            height: 1,
            filter: FilterMode::Linear,
            address_mode: AddressMode::ClampToEdge,
        }
    }

    /// Create a checkerboard pattern texture.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the texture (size x size pixels)
    /// * `cell_size` - Size of each checker cell in pixels
    /// * `color1` - First color (RGBA)
    /// * `color2` - Second color (RGBA)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let checker = TextureConfig::checkerboard(64, 8, [255, 255, 255, 255], [0, 0, 0, 255]);
    /// ```
    pub fn checkerboard(size: u32, cell_size: u32, color1: [u8; 4], color2: [u8; 4]) -> Self {
        let mut data = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let cx = x / cell_size;
                let cy = y / cell_size;
                let color = if (cx + cy) % 2 == 0 { color1 } else { color2 };
                data.extend_from_slice(&color);
            }
        }
        Self {
            data,
            width: size,
            height: size,
            filter: FilterMode::Nearest,
            address_mode: AddressMode::Repeat,
        }
    }

    /// Create a simple noise texture using a basic hash function.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the texture (size x size pixels)
    /// * `seed` - Random seed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let noise = TextureConfig::noise(256, 42);
    /// ```
    pub fn noise(size: u32, seed: u32) -> Self {
        let mut data = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                // Simple hash-based noise
                let v = hash_noise(x, y, seed);
                data.push(v);
                data.push(v);
                data.push(v);
                data.push(255);
            }
        }
        Self {
            data,
            width: size,
            height: size,
            filter: FilterMode::Linear,
            address_mode: AddressMode::Repeat,
        }
    }
}

/// Convenience implementations for easy texture creation.
impl From<&str> for TextureConfig {
    fn from(path: &str) -> Self {
        TextureConfig::from_file(path)
    }
}

impl From<String> for TextureConfig {
    fn from(path: String) -> Self {
        TextureConfig::from_file(path)
    }
}

impl From<&Path> for TextureConfig {
    fn from(path: &Path) -> Self {
        TextureConfig::from_file(path)
    }
}

/// Helper function for linear interpolation of u8 values.
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = a as f32;
    let b = b as f32;
    (a + (b - a) * t).round() as u8
}

/// Simple hash-based noise function.
fn hash_noise(x: u32, y: u32, seed: u32) -> u8 {
    let mut n = x.wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(1013904223));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n = n ^ (n >> 16);
    (n & 255) as u8
}

/// Holds all texture configurations for a simulation.
#[derive(Debug, Clone, Default)]
pub struct TextureRegistry {
    /// Map of texture name to configuration.
    pub textures: Vec<(String, TextureConfig)>,
}

impl TextureRegistry {
    /// Create a new empty texture registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a texture to the registry.
    pub fn add(&mut self, name: impl Into<String>, config: impl Into<TextureConfig>) {
        self.textures.push((name.into(), config.into()));
    }

    /// Get the number of textures.
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Generate WGSL declarations for all textures.
    ///
    /// Returns shader code declaring texture and sampler bindings.
    /// Textures are bound to group 1, starting at binding 0.
    pub fn to_wgsl_declarations(&self, start_binding: u32) -> String {
        let mut code = String::new();
        let mut binding = start_binding;

        for (name, _config) in &self.textures {
            code.push_str(&format!(
                "@group(1) @binding({binding})\nvar tex_{name}: texture_2d<f32>;\n"
            ));
            binding += 1;
            code.push_str(&format!(
                "@group(1) @binding({binding})\nvar tex_{name}_sampler: sampler;\n"
            ));
            binding += 1;
        }

        code
    }
}
