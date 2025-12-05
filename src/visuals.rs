//! Visual configuration for particle rendering.
//!
//! This module provides rendering options that control how particles appear,
//! separate from the behavioral rules that control how they move.
//!
//! # Usage
//!
//! ```ignore
//! Simulation::<MyParticle>::new()
//!     .with_visuals(|v| {
//!         v.blend_mode(BlendMode::Additive);
//!         v.shape(ParticleShape::Circle);
//!         v.palette(Palette::Viridis, ColorMapping::Speed);
//!     })
//!     .with_rule(Rule::Gravity(9.8))
//!     .run();
//! ```

use glam::Vec3;

/// Pre-defined color palettes for particle rendering.
///
/// These palettes are sampled based on a [`ColorMapping`] to automatically
/// color particles without needing to set colors manually.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Palette {
    /// No palette - use particle's own color (default).
    #[default]
    None,

    /// Viridis - perceptually uniform, colorblind-friendly (purple to yellow).
    Viridis,

    /// Magma - perceptually uniform (black to yellow through red).
    Magma,

    /// Plasma - perceptually uniform (purple to yellow through pink).
    Plasma,

    /// Inferno - perceptually uniform (black to yellow through red/orange).
    Inferno,

    /// Rainbow - classic rainbow gradient (red through violet).
    Rainbow,

    /// Sunset - warm oranges and pinks.
    Sunset,

    /// Ocean - cool blues and teals.
    Ocean,

    /// Fire - black through red, orange, yellow, white.
    Fire,

    /// Ice - white through light blue to deep blue.
    Ice,

    /// Neon - vibrant cyberpunk colors (pink, cyan, purple).
    Neon,

    /// Forest - natural greens and browns.
    Forest,

    /// Grayscale - black to white.
    Grayscale,
}

impl Palette {
    /// Get the color stops for this palette (5 colors).
    pub fn colors(&self) -> [Vec3; 5] {
        match self {
            Palette::None => [
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
            ],
            Palette::Viridis => [
                Vec3::new(0.267, 0.004, 0.329), // Dark purple
                Vec3::new(0.282, 0.140, 0.458), // Purple
                Vec3::new(0.127, 0.566, 0.551), // Teal
                Vec3::new(0.369, 0.789, 0.383), // Green
                Vec3::new(0.993, 0.906, 0.144), // Yellow
            ],
            Palette::Magma => [
                Vec3::new(0.001, 0.0, 0.014),   // Black
                Vec3::new(0.329, 0.071, 0.435), // Purple
                Vec3::new(0.716, 0.215, 0.475), // Pink
                Vec3::new(0.994, 0.541, 0.380), // Orange
                Vec3::new(0.987, 0.991, 0.749), // Light yellow
            ],
            Palette::Plasma => [
                Vec3::new(0.050, 0.030, 0.528), // Dark blue
                Vec3::new(0.494, 0.012, 0.658), // Purple
                Vec3::new(0.798, 0.280, 0.470), // Pink
                Vec3::new(0.973, 0.580, 0.254), // Orange
                Vec3::new(0.940, 0.975, 0.131), // Yellow
            ],
            Palette::Inferno => [
                Vec3::new(0.001, 0.0, 0.014),   // Black
                Vec3::new(0.341, 0.063, 0.429), // Purple
                Vec3::new(0.735, 0.216, 0.330), // Red
                Vec3::new(0.988, 0.645, 0.198), // Orange
                Vec3::new(0.988, 1.0, 0.644),   // Light yellow
            ],
            Palette::Rainbow => [
                Vec3::new(1.0, 0.0, 0.0),   // Red
                Vec3::new(1.0, 1.0, 0.0),   // Yellow
                Vec3::new(0.0, 1.0, 0.0),   // Green
                Vec3::new(0.0, 1.0, 1.0),   // Cyan
                Vec3::new(0.5, 0.0, 1.0),   // Purple
            ],
            Palette::Sunset => [
                Vec3::new(0.1, 0.0, 0.2),   // Dark purple
                Vec3::new(0.5, 0.0, 0.5),   // Purple
                Vec3::new(1.0, 0.2, 0.4),   // Pink
                Vec3::new(1.0, 0.5, 0.2),   // Orange
                Vec3::new(1.0, 0.9, 0.4),   // Yellow
            ],
            Palette::Ocean => [
                Vec3::new(0.0, 0.05, 0.15), // Deep blue
                Vec3::new(0.0, 0.2, 0.4),   // Dark blue
                Vec3::new(0.0, 0.4, 0.6),   // Blue
                Vec3::new(0.2, 0.6, 0.8),   // Light blue
                Vec3::new(0.6, 0.9, 1.0),   // Cyan
            ],
            Palette::Fire => [
                Vec3::new(0.1, 0.0, 0.0),   // Dark red
                Vec3::new(0.5, 0.0, 0.0),   // Red
                Vec3::new(1.0, 0.3, 0.0),   // Orange
                Vec3::new(1.0, 0.7, 0.0),   // Yellow-orange
                Vec3::new(1.0, 1.0, 0.8),   // White-yellow
            ],
            Palette::Ice => [
                Vec3::new(1.0, 1.0, 1.0),   // White
                Vec3::new(0.8, 0.9, 1.0),   // Light blue
                Vec3::new(0.4, 0.7, 1.0),   // Blue
                Vec3::new(0.1, 0.4, 0.8),   // Medium blue
                Vec3::new(0.0, 0.1, 0.4),   // Dark blue
            ],
            Palette::Neon => [
                Vec3::new(1.0, 0.0, 0.5),   // Pink
                Vec3::new(0.5, 0.0, 1.0),   // Purple
                Vec3::new(0.0, 0.5, 1.0),   // Blue
                Vec3::new(0.0, 1.0, 1.0),   // Cyan
                Vec3::new(0.5, 1.0, 0.5),   // Green
            ],
            Palette::Forest => [
                Vec3::new(0.1, 0.05, 0.0),  // Dark brown
                Vec3::new(0.3, 0.15, 0.05), // Brown
                Vec3::new(0.2, 0.4, 0.1),   // Dark green
                Vec3::new(0.3, 0.6, 0.2),   // Green
                Vec3::new(0.5, 0.8, 0.3),   // Light green
            ],
            Palette::Grayscale => [
                Vec3::new(0.0, 0.0, 0.0),   // Black
                Vec3::new(0.25, 0.25, 0.25),
                Vec3::new(0.5, 0.5, 0.5),
                Vec3::new(0.75, 0.75, 0.75),
                Vec3::new(1.0, 1.0, 1.0),   // White
            ],
        }
    }
}

/// How to map particle properties to palette colors.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ColorMapping {
    /// Use the particle's own color field (default).
    #[default]
    None,

    /// Map particle index to color (creates bands/stripes).
    Index,

    /// Map particle speed to color (slow = start, fast = end).
    Speed {
        /// Minimum speed (maps to palette start).
        min: f32,
        /// Maximum speed (maps to palette end).
        max: f32,
    },

    /// Map particle age to color (young = start, old = end).
    Age {
        /// Maximum age for full palette range.
        max_age: f32,
    },

    /// Map Y position to color (bottom = start, top = end).
    PositionY {
        /// Minimum Y (maps to palette start).
        min: f32,
        /// Maximum Y (maps to palette end).
        max: f32,
    },

    /// Map distance from origin to color.
    Distance {
        /// Maximum distance for full palette range.
        max_dist: f32,
    },

    /// Random color per particle (uses particle index as seed).
    Random,
}

/// Blend mode for particle rendering.
///
/// Controls how particle colors combine with the background and each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// Standard alpha blending (default).
    ///
    /// Particles blend based on their alpha value. Good for solid,
    /// opaque-looking particles.
    #[default]
    Alpha,

    /// Additive blending.
    ///
    /// Particle colors are added together, creating a glowing effect.
    /// Overlapping particles become brighter. Perfect for fire, magic,
    /// energy effects, and anything that should "glow".
    Additive,

    /// Multiplicative blending.
    ///
    /// Colors are multiplied, darkening the result. Useful for shadows,
    /// smoke, or atmospheric effects.
    Multiply,
}

/// Particle shape for rendering.
///
/// Controls the visual shape of each particle. All shapes use the UV coordinate
/// system where (-1, -1) is bottom-left and (1, 1) is top-right of the particle quad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParticleShape {
    /// Soft circle with smooth falloff (default).
    #[default]
    Circle,

    /// Hard-edged circle with no falloff.
    CircleHard,

    /// Square/rectangle shape.
    Square,

    /// Ring/donut shape.
    Ring,

    /// 5-pointed star.
    Star,

    /// Equilateral triangle pointing up.
    Triangle,

    /// Regular hexagon.
    Hexagon,

    /// Diamond/rhombus shape.
    Diamond,

    /// Single pixel point (fastest, no shape calculation).
    Point,
}

impl ParticleShape {
    /// Generate the WGSL fragment shader body for this shape.
    ///
    /// The shader receives `in.uv` as vec2 in range [-1, 1] and `in.color` as vec3.
    /// It should return a vec4 color with alpha.
    pub fn to_wgsl_fragment(&self) -> &'static str {
        match self {
            ParticleShape::Circle => r#"    let dist = length(in.uv);
    if dist > 1.0 {
        discard;
    }
    let alpha = 1.0 - smoothstep(0.5, 1.0, dist);
    return vec4<f32>(in.color, alpha);"#,

            ParticleShape::CircleHard => r#"    let dist = length(in.uv);
    if dist > 1.0 {
        discard;
    }
    return vec4<f32>(in.color, 1.0);"#,

            ParticleShape::Square => r#"    return vec4<f32>(in.color, 1.0);"#,

            ParticleShape::Ring => r#"    let dist = length(in.uv);
    if dist > 1.0 || dist < 0.6 {
        discard;
    }
    let alpha = 1.0 - smoothstep(0.85, 1.0, dist);
    return vec4<f32>(in.color, alpha);"#,

            ParticleShape::Star => r#"    // 5-pointed star using polar coordinates
    let angle = atan2(in.uv.y, in.uv.x);
    let dist = length(in.uv);

    // Star shape: varies radius based on angle (5 points)
    let points = 5.0;
    let star_angle = angle + 3.14159 / 2.0; // Rotate so point faces up
    let star_factor = cos(star_angle * points) * 0.4 + 0.6;

    if dist > star_factor {
        discard;
    }
    return vec4<f32>(in.color, 1.0);"#,

            ParticleShape::Triangle => r#"    // Equilateral triangle pointing up
    let p = in.uv;

    // Triangle: use simple half-plane tests
    // Top vertex at (0, 0.8), bottom edge at y = -0.6
    // Left edge: from (-0.8, -0.6) to (0, 0.8)
    // Right edge: from (0.8, -0.6) to (0, 0.8)

    // Bottom edge
    if p.y < -0.6 {
        discard;
    }

    // Left edge: points right of line from (-0.8, -0.6) to (0, 0.8)
    // Line equation: 1.4x - 0.8y + 0.64 > 0 for inside
    let left = 1.4 * p.x - 0.8 * p.y + 0.64;
    if left < 0.0 {
        discard;
    }

    // Right edge: points left of line from (0.8, -0.6) to (0, 0.8)
    // Line equation: -1.4x - 0.8y + 0.64 > 0 for inside
    let right = -1.4 * p.x - 0.8 * p.y + 0.64;
    if right < 0.0 {
        discard;
    }

    return vec4<f32>(in.color, 1.0);"#,

            ParticleShape::Hexagon => r#"    // Regular hexagon using max of 3 axes
    let p = abs(in.uv);

    // Hexagon distance: check against 3 edge normals
    // Pointy-top hexagon
    let hex_dist = max(p.x * 0.866025 + p.y * 0.5, p.y);

    if hex_dist > 0.9 {
        discard;
    }
    return vec4<f32>(in.color, 1.0);"#,

            ParticleShape::Diamond => r#"    // Diamond/rhombus: Manhattan distance
    let dist = abs(in.uv.x) + abs(in.uv.y);
    if dist > 1.0 {
        discard;
    }
    return vec4<f32>(in.color, 1.0);"#,

            ParticleShape::Point => r#"    // Single pixel - no shape calculation needed
    return vec4<f32>(in.color, 1.0);"#,
        }
    }
}

/// Configuration for particle visuals.
///
/// Built using the closure passed to [`Simulation::with_visuals`].
#[derive(Debug, Clone)]
pub struct VisualConfig {
    /// Blend mode for particle rendering.
    pub blend_mode: BlendMode,
    /// Particle shape.
    pub shape: ParticleShape,
    /// Trail length (0 = no trails).
    pub trail_length: u32,
    /// Whether to draw connections between nearby particles.
    pub connections_enabled: bool,
    /// Radius for particle connections.
    pub connections_radius: f32,
    /// Whether to stretch particles in velocity direction.
    pub velocity_stretch: bool,
    /// Maximum stretch factor for velocity stretching.
    pub velocity_stretch_factor: f32,
    /// Color palette for automatic coloring.
    pub palette: Palette,
    /// How to map particle properties to palette colors.
    pub color_mapping: ColorMapping,
    /// Background clear color (RGB, 0.0-1.0).
    pub background_color: Vec3,
    /// Custom post-processing shader code (fragment shader body).
    pub post_process_shader: Option<String>,
    /// Spatial grid visualization opacity (0.0 = off, 1.0 = full).
    pub spatial_grid_opacity: f32,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            blend_mode: BlendMode::Alpha,
            shape: ParticleShape::Circle,
            trail_length: 0,
            connections_enabled: false,
            connections_radius: 0.1,
            velocity_stretch: false,
            velocity_stretch_factor: 2.0,
            palette: Palette::None,
            color_mapping: ColorMapping::None,
            background_color: Vec3::new(0.02, 0.02, 0.05), // Dark blue-black
            post_process_shader: None,
            spatial_grid_opacity: 0.0, // Off by default
        }
    }
}

impl VisualConfig {
    /// Create a new visual config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the blend mode.
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.blend_mode(BlendMode::Additive); // Glowy particles
    /// })
    /// ```
    pub fn blend_mode(&mut self, mode: BlendMode) -> &mut Self {
        self.blend_mode = mode;
        self
    }

    /// Set the particle shape.
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.shape(ParticleShape::Star);
    /// })
    /// ```
    pub fn shape(&mut self, shape: ParticleShape) -> &mut Self {
        self.shape = shape;
        self
    }

    /// Enable particle trails.
    ///
    /// Trails render a fading history of particle positions.
    ///
    /// # Arguments
    ///
    /// * `length` - Number of previous positions to render (0 = disabled)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.trails(10); // 10-frame trails
    /// })
    /// ```
    pub fn trails(&mut self, length: u32) -> &mut Self {
        self.trail_length = length;
        self
    }

    /// Enable connections between nearby particles.
    ///
    /// Draws lines between particles within the specified radius.
    /// Creates web-like or network visualizations.
    ///
    /// # Arguments
    ///
    /// * `radius` - Maximum distance for connections
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.connections(0.15); // Connect particles within 0.15 units
    /// })
    /// ```
    pub fn connections(&mut self, radius: f32) -> &mut Self {
        self.connections_enabled = true;
        self.connections_radius = radius;
        self
    }

    /// Enable velocity-based stretching.
    ///
    /// Particles stretch in their direction of motion, creating
    /// a speed-blur effect.
    ///
    /// # Arguments
    ///
    /// * `max_factor` - Maximum stretch multiplier (e.g., 3.0 = 3x longer)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.velocity_stretch(3.0); // Stretch up to 3x in motion direction
    /// })
    /// ```
    pub fn velocity_stretch(&mut self, max_factor: f32) -> &mut Self {
        self.velocity_stretch = true;
        self.velocity_stretch_factor = max_factor;
        self
    }

    /// Set a color palette with automatic mapping.
    ///
    /// Overrides particle colors with palette colors based on the mapping.
    ///
    /// # Arguments
    ///
    /// * `palette` - The color palette to use
    /// * `mapping` - How to map particle properties to colors
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     // Color by speed using the fire palette
    ///     v.palette(Palette::Fire, ColorMapping::Speed { min: 0.0, max: 2.0 });
    /// })
    /// ```
    pub fn palette(&mut self, palette: Palette, mapping: ColorMapping) -> &mut Self {
        self.palette = palette;
        self.color_mapping = mapping;
        self
    }

    /// Set the background clear color.
    ///
    /// # Arguments
    ///
    /// * `color` - RGB color values (0.0-1.0 range)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.background(Vec3::new(0.0, 0.0, 0.0)); // Pure black
    ///     v.background(Vec3::new(0.1, 0.0, 0.15)); // Dark purple
    /// })
    /// ```
    pub fn background(&mut self, color: Vec3) -> &mut Self {
        self.background_color = color;
        self
    }

    /// Show the spatial hash grid as a wireframe overlay.
    ///
    /// Useful for debugging spatial configuration and understanding
    /// how the neighbor query system works.
    ///
    /// # Arguments
    ///
    /// * `opacity` - Grid line opacity (0.0 = off, 1.0 = fully visible)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.spatial_grid(0.3); // Subtle grid overlay
    /// })
    /// ```
    pub fn spatial_grid(&mut self, opacity: f32) -> &mut Self {
        self.spatial_grid_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set a custom post-processing shader.
    ///
    /// The shader code runs as a fullscreen pass after particles are rendered.
    /// Your code has access to:
    ///
    /// - `in.uv` - Screen UV coordinates (0.0 to 1.0)
    /// - `scene` - Texture sampler for the rendered particle scene
    /// - `uniforms.time` - Current simulation time
    /// - `uniforms.resolution` - Screen resolution (vec2)
    ///
    /// Use `textureSample(scene, scene_sampler, uv)` to sample the scene.
    /// Must return `vec4<f32>` (RGBA color output).
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     // Vignette effect
    ///     v.post_process(r#"
    ///         let color = textureSample(scene, scene_sampler, in.uv);
    ///         let dist = length(in.uv - vec2(0.5));
    ///         let vignette = 1.0 - smoothstep(0.3, 0.7, dist);
    ///         return vec4(color.rgb * vignette, 1.0);
    ///     "#);
    /// })
    /// ```
    ///
    /// # Effects you can create
    ///
    /// - **Vignette**: Darken edges based on distance from center
    /// - **Color grading**: Adjust colors, contrast, saturation
    /// - **Chromatic aberration**: Offset RGB channels
    /// - **CRT/scanlines**: Add retro display effects
    /// - **Blur**: Sample nearby pixels (limited without multiple passes)
    pub fn post_process(&mut self, wgsl_code: &str) -> &mut Self {
        self.post_process_shader = Some(wgsl_code.to_string());
        self
    }
}
