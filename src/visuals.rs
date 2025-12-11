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
    /// Color for particle connections (RGB, 0.0-1.0).
    pub connections_color: Vec3,
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
    /// Wireframe mesh for 3D particle shapes (None = use billboard shapes).
    pub wireframe_mesh: Option<WireframeMesh>,
    /// Line thickness for wireframe rendering (in clip space, ~0.001-0.01).
    pub wireframe_thickness: f32,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            blend_mode: BlendMode::Alpha,
            shape: ParticleShape::Circle,
            trail_length: 0,
            connections_enabled: false,
            connections_radius: 0.1,
            connections_color: Vec3::new(0.5, 0.7, 1.0),
            velocity_stretch: false,
            velocity_stretch_factor: 2.0,
            palette: Palette::None,
            color_mapping: ColorMapping::None,
            background_color: Vec3::new(0.02, 0.02, 0.05), // Dark blue-black
            post_process_shader: None,
            spatial_grid_opacity: 0.0, // Off by default
            wireframe_mesh: None,
            wireframe_thickness: 0.003, // Default line thickness
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

    /// Set the color for particle connections.
    ///
    /// # Arguments
    ///
    /// * `color` - RGB color (0.0-1.0)
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.connections(0.15)
    ///      .connections_color(Vec3::new(1.0, 0.5, 0.0)); // Orange connections
    /// })
    /// ```
    pub fn connections_color(&mut self, color: Vec3) -> &mut Self {
        self.connections_color = color;
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

    /// Set a wireframe mesh for 3D particle shapes.
    ///
    /// Instead of rendering particles as billboards (flat shapes facing the camera),
    /// this renders each particle as a 3D wireframe mesh. The mesh is scaled by
    /// the particle's scale and colored by its color.
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_visuals(|v| {
    ///     v.wireframe(WireframeMesh::cube(), 0.002); // Cube with 0.002 line thickness
    /// })
    /// ```
    pub fn wireframe(&mut self, mesh: WireframeMesh, line_thickness: f32) -> &mut Self {
        self.wireframe_mesh = Some(mesh);
        self.wireframe_thickness = line_thickness;
        self
    }

    /// Compare this config with another to determine what kind of rebuild is needed.
    ///
    /// Returns a `ConfigDiff` describing which changes can be hot-swapped and
    /// which require pipeline rebuilds.
    pub fn diff(&self, other: &VisualConfig) -> ConfigDiff {
        let mut hot_swappable = Vec::new();

        // Check hot-swappable changes
        if self.background_color != other.background_color {
            hot_swappable.push(HotSwapChange::BackgroundColor(other.background_color));
        }
        if self.spatial_grid_opacity != other.spatial_grid_opacity {
            hot_swappable.push(HotSwapChange::GridOpacity(other.spatial_grid_opacity));
        }

        // Check if render pipeline rebuild is needed
        let needs_render_rebuild = self.blend_mode != other.blend_mode
            || self.shape != other.shape
            || self.palette != other.palette
            || self.color_mapping != other.color_mapping
            || self.trail_length != other.trail_length
            || self.connections_enabled != other.connections_enabled
            || self.connections_radius != other.connections_radius
            || self.velocity_stretch != other.velocity_stretch
            || self.velocity_stretch_factor != other.velocity_stretch_factor
            || self.wireframe_mesh != other.wireframe_mesh
            || self.wireframe_thickness != other.wireframe_thickness
            || self.post_process_shader != other.post_process_shader;

        ConfigDiff {
            needs_render_rebuild,
            hot_swappable,
        }
    }
}

/// Result of comparing two `VisualConfig`s.
///
/// Describes what kind of updates are needed when changing visual settings.
#[derive(Debug, Clone)]
pub struct ConfigDiff {
    /// Whether the render pipeline needs to be rebuilt.
    ///
    /// This is required for changes to blend mode, shape, palette, trails, etc.
    pub needs_render_rebuild: bool,

    /// Changes that can be applied without rebuilding pipelines.
    pub hot_swappable: Vec<HotSwapChange>,
}

impl ConfigDiff {
    /// Returns true if no changes are needed.
    pub fn is_empty(&self) -> bool {
        !self.needs_render_rebuild && self.hot_swappable.is_empty()
    }

    /// Returns true if any changes require a rebuild.
    pub fn needs_rebuild(&self) -> bool {
        self.needs_render_rebuild
    }
}

/// A visual change that can be applied at runtime without rebuilding pipelines.
#[derive(Debug, Clone)]
pub enum HotSwapChange {
    /// Change the background clear color.
    BackgroundColor(Vec3),
    /// Change the spatial grid debug opacity.
    GridOpacity(f32),
}

/// A wireframe mesh for 3D particle rendering.
///
/// Instead of rendering particles as flat billboards, wireframe meshes render
/// each particle as a 3D shape made of line segments. This is purely visual -
/// it doesn't affect the physics simulation.
///
/// # Built-in Shapes
///
/// Use the constructor methods for common shapes:
///
/// ```ignore
/// WireframeMesh::tetrahedron() // 4 triangular faces
/// WireframeMesh::cube()        // Classic cube
/// WireframeMesh::octahedron()  // 8 triangular faces
/// WireframeMesh::diamond()     // Two pyramids joined at base
/// WireframeMesh::axes()        // XYZ axis indicator
/// ```
///
/// # Custom Shapes
///
/// Create custom wireframes from line segment pairs:
///
/// ```ignore
/// WireframeMesh::custom(vec![
///     (Vec3::ZERO, Vec3::X),           // Line from origin to +X
///     (Vec3::ZERO, Vec3::Y),           // Line from origin to +Y
///     (Vec3::X, Vec3::new(1.0, 1.0, 0.0)), // Diagonal
/// ])
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct WireframeMesh {
    /// Line segments as pairs of endpoints (start, end).
    pub lines: Vec<(Vec3, Vec3)>,
}

impl WireframeMesh {
    /// Create a custom wireframe from line segments.
    pub fn custom(lines: Vec<(Vec3, Vec3)>) -> Self {
        Self { lines }
    }

    /// Tetrahedron (4 triangular faces, 6 edges).
    pub fn tetrahedron() -> Self {
        let s = 0.5;
        // Vertices of a regular tetrahedron
        let v0 = Vec3::new(s, s, s);
        let v1 = Vec3::new(s, -s, -s);
        let v2 = Vec3::new(-s, s, -s);
        let v3 = Vec3::new(-s, -s, s);

        Self {
            lines: vec![
                (v0, v1),
                (v0, v2),
                (v0, v3),
                (v1, v2),
                (v1, v3),
                (v2, v3),
            ],
        }
    }

    /// Cube (6 faces, 12 edges).
    pub fn cube() -> Self {
        let s = 0.5;
        // 8 vertices of a cube
        let v000 = Vec3::new(-s, -s, -s);
        let v001 = Vec3::new(-s, -s, s);
        let v010 = Vec3::new(-s, s, -s);
        let v011 = Vec3::new(-s, s, s);
        let v100 = Vec3::new(s, -s, -s);
        let v101 = Vec3::new(s, -s, s);
        let v110 = Vec3::new(s, s, -s);
        let v111 = Vec3::new(s, s, s);

        Self {
            lines: vec![
                // Bottom face
                (v000, v100),
                (v100, v101),
                (v101, v001),
                (v001, v000),
                // Top face
                (v010, v110),
                (v110, v111),
                (v111, v011),
                (v011, v010),
                // Vertical edges
                (v000, v010),
                (v100, v110),
                (v101, v111),
                (v001, v011),
            ],
        }
    }

    /// Octahedron (8 triangular faces, 12 edges).
    pub fn octahedron() -> Self {
        let s = 0.5;
        // 6 vertices at axis extremes
        let px = Vec3::new(s, 0.0, 0.0);
        let nx = Vec3::new(-s, 0.0, 0.0);
        let py = Vec3::new(0.0, s, 0.0);
        let ny = Vec3::new(0.0, -s, 0.0);
        let pz = Vec3::new(0.0, 0.0, s);
        let nz = Vec3::new(0.0, 0.0, -s);

        Self {
            lines: vec![
                // Top pyramid
                (py, px),
                (py, nx),
                (py, pz),
                (py, nz),
                // Bottom pyramid
                (ny, px),
                (ny, nx),
                (ny, pz),
                (ny, nz),
                // Equator
                (px, pz),
                (pz, nx),
                (nx, nz),
                (nz, px),
            ],
        }
    }

    /// Diamond shape (two pyramids joined at base, 8 edges).
    pub fn diamond() -> Self {
        let s = 0.5;
        let h = 0.7; // Height of each pyramid half

        // 6 vertices: top, bottom, and 4 at equator
        let top = Vec3::new(0.0, h, 0.0);
        let bot = Vec3::new(0.0, -h, 0.0);
        let e0 = Vec3::new(s, 0.0, 0.0);
        let e1 = Vec3::new(0.0, 0.0, s);
        let e2 = Vec3::new(-s, 0.0, 0.0);
        let e3 = Vec3::new(0.0, 0.0, -s);

        Self {
            lines: vec![
                // Top to equator
                (top, e0),
                (top, e1),
                (top, e2),
                (top, e3),
                // Bottom to equator
                (bot, e0),
                (bot, e1),
                (bot, e2),
                (bot, e3),
            ],
        }
    }

    /// Icosahedron (20 triangular faces, 30 edges).
    pub fn icosahedron() -> Self {
        // Golden ratio
        let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let s = 0.3; // Scale down to fit

        // 12 vertices
        let vertices = [
            Vec3::new(-1.0, phi, 0.0) * s,
            Vec3::new(1.0, phi, 0.0) * s,
            Vec3::new(-1.0, -phi, 0.0) * s,
            Vec3::new(1.0, -phi, 0.0) * s,
            Vec3::new(0.0, -1.0, phi) * s,
            Vec3::new(0.0, 1.0, phi) * s,
            Vec3::new(0.0, -1.0, -phi) * s,
            Vec3::new(0.0, 1.0, -phi) * s,
            Vec3::new(phi, 0.0, -1.0) * s,
            Vec3::new(phi, 0.0, 1.0) * s,
            Vec3::new(-phi, 0.0, -1.0) * s,
            Vec3::new(-phi, 0.0, 1.0) * s,
        ];

        // 30 edges (each unique edge of the icosahedron)
        let edges = [
            (0, 1), (0, 5), (0, 7), (0, 10), (0, 11),
            (1, 5), (1, 7), (1, 8), (1, 9),
            (2, 3), (2, 4), (2, 6), (2, 10), (2, 11),
            (3, 4), (3, 6), (3, 8), (3, 9),
            (4, 5), (4, 9), (4, 11),
            (5, 9), (5, 11),
            (6, 7), (6, 8), (6, 10),
            (7, 8), (7, 10),
            (8, 9),
            (10, 11),
        ];

        Self {
            lines: edges.iter().map(|(i, j)| (vertices[*i], vertices[*j])).collect(),
        }
    }

    /// XYZ axes indicator.
    pub fn axes() -> Self {
        let s = 0.5;
        Self {
            lines: vec![
                (Vec3::ZERO, Vec3::new(s, 0.0, 0.0)),  // X axis
                (Vec3::ZERO, Vec3::new(0.0, s, 0.0)),  // Y axis
                (Vec3::ZERO, Vec3::new(0.0, 0.0, s)),  // Z axis
            ],
        }
    }

    /// Star shape (spiky, 6 points).
    pub fn star() -> Self {
        let inner = 0.2;
        let outer = 0.5;
        let points = 6;

        let mut lines = Vec::new();
        for i in 0..points {
            let angle = (i as f32 / points as f32) * std::f32::consts::TAU;
            let outer_point = Vec3::new(angle.cos() * outer, angle.sin() * outer, 0.0);

            // Connect to center
            lines.push((Vec3::ZERO, outer_point));

            // Connect to adjacent inner points
            let angle_prev = ((i as f32 - 0.5) / points as f32) * std::f32::consts::TAU;
            let angle_next = ((i as f32 + 0.5) / points as f32) * std::f32::consts::TAU;
            let inner_prev = Vec3::new(angle_prev.cos() * inner, angle_prev.sin() * inner, 0.0);
            let inner_next = Vec3::new(angle_next.cos() * inner, angle_next.sin() * inner, 0.0);

            lines.push((outer_point, inner_prev));
            lines.push((outer_point, inner_next));
        }

        Self { lines }
    }

    /// Spiral shape.
    pub fn spiral(turns: f32, segments: u32) -> Self {
        let mut lines = Vec::new();
        let height = 0.5;
        let radius = 0.3;

        for i in 0..segments {
            let t0 = i as f32 / segments as f32;
            let t1 = (i + 1) as f32 / segments as f32;

            let angle0 = t0 * turns * std::f32::consts::TAU;
            let angle1 = t1 * turns * std::f32::consts::TAU;

            let p0 = Vec3::new(
                angle0.cos() * radius,
                t0 * height - height / 2.0,
                angle0.sin() * radius,
            );
            let p1 = Vec3::new(
                angle1.cos() * radius,
                t1 * height - height / 2.0,
                angle1.sin() * radius,
            );

            lines.push((p0, p1));
        }

        Self { lines }
    }

    /// Get the total number of line segments.
    pub fn line_count(&self) -> u32 {
        self.lines.len() as u32
    }

    /// Get vertices as flat f32 array for GPU buffer.
    /// Each line is 6 floats: [x0, y0, z0, x1, y1, z1]
    pub fn to_vertices(&self) -> Vec<f32> {
        self.lines
            .iter()
            .flat_map(|(a, b)| [a.x, a.y, a.z, b.x, b.y, b.z])
            .collect()
    }
}

/// Vertex shader effects for particle rendering.
///
/// Pre-built, composable effects that modify particle vertex transformations.
/// Effects stack together and generate optimized WGSL code. Use these instead
/// of raw [`Simulation::with_vertex_shader`] for common effects.
///
/// # Example
///
/// ```ignore
/// Simulation::<Ball>::new()
///     .with_vertex_effect(VertexEffect::Rotate { speed: 2.0 })
///     .with_vertex_effect(VertexEffect::Wobble {
///         frequency: 3.0,
///         amplitude: 0.05,
///     })
///     .with_vertex_effect(VertexEffect::Pulse {
///         frequency: 4.0,
///         amplitude: 0.3,
///     })
///     .run();
/// ```
///
/// Effects are applied in order and compose naturally.
#[derive(Debug, Clone)]
pub enum VertexEffect {
    /// Rotate particles around their facing axis.
    ///
    /// Each particle spins at the specified speed, with a per-particle
    /// phase offset based on instance index for variety.
    Rotate {
        /// Rotation speed in radians per second.
        speed: f32,
    },

    /// Wobble particles with sinusoidal position offset.
    ///
    /// Creates a gentle floating/swaying motion.
    Wobble {
        /// Oscillation frequency (higher = faster wobble).
        frequency: f32,
        /// Maximum offset distance.
        amplitude: f32,
    },

    /// Pulse particle size over time.
    ///
    /// Particles grow and shrink rhythmically.
    Pulse {
        /// Pulse frequency (higher = faster pulsing).
        frequency: f32,
        /// Pulse amplitude (0.3 = +/- 30% size variation).
        amplitude: f32,
    },

    /// Wave effect coordinated across particles.
    ///
    /// Creates a wave pattern that travels through the particle field.
    Wave {
        /// Wave direction (normalized).
        direction: Vec3,
        /// Wave frequency (spatial).
        frequency: f32,
        /// Wave speed (temporal).
        speed: f32,
        /// Wave amplitude (offset distance).
        amplitude: f32,
    },

    /// Random per-frame jitter/shake.
    ///
    /// Adds noise-like motion to particles.
    Jitter {
        /// Maximum jitter offset.
        amplitude: f32,
    },

    /// Stretch particles in their velocity direction.
    ///
    /// Requires velocity data passed to vertex shader.
    /// Note: This is a visual hint only - actual velocity stretching
    /// requires velocity in the vertex attributes.
    StretchToVelocity {
        /// Maximum stretch multiplier.
        max_stretch: f32,
    },

    /// Scale particles based on distance from a point.
    ///
    /// Particles closer to the point are larger.
    ScaleByDistance {
        /// Center point.
        center: Vec3,
        /// Scale at center (closest).
        min_scale: f32,
        /// Scale at max_distance (farthest).
        max_scale: f32,
        /// Distance at which max_scale is reached.
        max_distance: f32,
    },

    /// Fade particles based on distance from camera/origin.
    ///
    /// Modifies alpha based on distance for depth-based fading.
    FadeByDistance {
        /// Distance at which particles are fully visible.
        near: f32,
        /// Distance at which particles are fully transparent.
        far: f32,
    },

    /// Cylindrical billboarding - rotate only around one axis.
    ///
    /// Useful for grass, trees, flames - things that should stay upright
    /// but still face the camera horizontally.
    BillboardCylindrical {
        /// The axis to stay fixed (typically Y for upright sprites).
        axis: Vec3,
    },

    /// Fixed orientation - disable billboarding entirely.
    ///
    /// Particles maintain a fixed orientation in world space.
    /// Useful for debris, leaves, or when combined with Rotate for tumbling.
    BillboardFixed {
        /// Forward direction of the quad in world space.
        forward: Vec3,
        /// Up direction of the quad in world space.
        up: Vec3,
    },

    /// Orient particles to face a specific point.
    ///
    /// All particles rotate to look at the target point.
    FacePoint {
        /// The point all particles should face toward.
        target: Vec3,
    },
}

impl VertexEffect {
    /// Generate WGSL code that modifies transformation variables.
    ///
    /// Available variables to read/modify:
    /// - `pos_offset: vec3<f32>` - position offset (starts at 0)
    /// - `rotated_quad: vec2<f32>` - quad coordinates (starts at quad_pos)
    /// - `size_mult: f32` - size multiplier (starts at 1.0)
    /// - `color_mod: vec3<f32>` - color modifier (starts at particle_color)
    ///
    /// Also available (read-only):
    /// - `particle_pos`, `particle_size`, `scale`
    /// - `uniforms.time`, `instance_index`, `vertex_index`
    pub fn to_wgsl(&self) -> String {
        match self {
            VertexEffect::Rotate { speed } => format!(
                r#"
    // Rotate effect
    {{
        let rot_speed = {speed}f;
        let rot_angle = uniforms.time * rot_speed + f32(instance_index) * 0.1;
        let cos_a = cos(rot_angle);
        let sin_a = sin(rot_angle);
        let rx = rotated_quad.x;
        let ry = rotated_quad.y;
        rotated_quad = vec2<f32>(
            rx * cos_a - ry * sin_a,
            rx * sin_a + ry * cos_a
        );
    }}"#
            ),

            VertexEffect::Wobble { frequency, amplitude } => format!(
                r#"
    // Wobble effect
    {{
        let wobble_freq = {frequency}f;
        let wobble_amp = {amplitude}f;
        let phase = f32(instance_index) * 0.5;
        pos_offset += vec3<f32>(
            sin(uniforms.time * wobble_freq + phase) * wobble_amp,
            cos(uniforms.time * wobble_freq * 1.3 + phase * 0.7) * wobble_amp,
            sin(uniforms.time * wobble_freq * 0.7 + phase * 0.3) * wobble_amp
        );
    }}"#
            ),

            VertexEffect::Pulse { frequency, amplitude } => format!(
                r#"
    // Pulse effect
    {{
        let pulse_freq = {frequency}f;
        let pulse_amp = {amplitude}f;
        let phase = f32(instance_index) * 0.2;
        size_mult *= 1.0 + sin(uniforms.time * pulse_freq + phase) * pulse_amp;
    }}"#
            ),

            VertexEffect::Wave { direction, frequency, speed, amplitude } => format!(
                r#"
    // Wave effect
    {{
        let wave_dir = vec3<f32>({}f, {}f, {}f);
        let wave_freq = {frequency}f;
        let wave_speed = {speed}f;
        let wave_amp = {amplitude}f;
        let wave_phase = dot(particle_pos, wave_dir) * wave_freq - uniforms.time * wave_speed;
        pos_offset += wave_dir * sin(wave_phase) * wave_amp;
    }}"#,
                direction.x, direction.y, direction.z
            ),

            VertexEffect::Jitter { amplitude } => format!(
                r#"
    // Jitter effect
    {{
        let jitter_amp = {amplitude}f;
        let seed = u32(uniforms.time * 60.0) + instance_index * 12345u;
        let jx = fract(sin(f32(seed) * 12.9898) * 43758.5453) * 2.0 - 1.0;
        let jy = fract(sin(f32(seed + 1u) * 12.9898) * 43758.5453) * 2.0 - 1.0;
        let jz = fract(sin(f32(seed + 2u) * 12.9898) * 43758.5453) * 2.0 - 1.0;
        pos_offset += vec3<f32>(jx, jy, jz) * jitter_amp;
    }}"#
            ),

            VertexEffect::StretchToVelocity { max_stretch } => format!(
                r#"
    // Stretch to velocity effect (approximated from position delta)
    {{
        let stretch_max = {max_stretch}f;
        // Note: This is a visual approximation. For true velocity stretching,
        // use with_vertex_shader() with velocity passed as attribute.
        let stretch_dir = normalize(particle_pos + vec3<f32>(0.001, 0.001, 0.001));
        let stretch_factor = 1.0 + length(particle_pos) * (stretch_max - 1.0);
        // Stretch quad in the direction of motion
        let stretch_dot = dot(normalize(rotated_quad), stretch_dir.xy);
        size_mult *= mix(1.0, stretch_factor, abs(stretch_dot));
    }}"#
            ),

            VertexEffect::ScaleByDistance { center, min_scale, max_scale, max_distance } => format!(
                r#"
    // Scale by distance effect
    {{
        let scale_center = vec3<f32>({}f, {}f, {}f);
        let scale_min = {min_scale}f;
        let scale_max = {max_scale}f;
        let scale_max_dist = {max_distance}f;
        let dist = length(particle_pos - scale_center);
        let t = clamp(dist / scale_max_dist, 0.0, 1.0);
        size_mult *= mix(scale_min, scale_max, t);
    }}"#,
                center.x, center.y, center.z
            ),

            VertexEffect::FadeByDistance { near, far } => format!(
                r#"
    // Fade by distance effect
    {{
        let fade_near = {near}f;
        let fade_far = {far}f;
        let dist = length(particle_pos);
        let fade = 1.0 - clamp((dist - fade_near) / (fade_far - fade_near), 0.0, 1.0);
        color_mod *= fade;
    }}"#
            ),

            VertexEffect::BillboardCylindrical { axis } => format!(
                r#"
    // Cylindrical billboard - fixed axis, face camera horizontally
    {{
        let fixed_axis = normalize(vec3<f32>({}f, {}f, {}f));
        // Camera is at origin looking at particles, so camera_dir approximates view
        let to_camera = normalize(-particle_pos);
        // Project to_camera onto plane perpendicular to fixed_axis
        let camera_flat = normalize(to_camera - fixed_axis * dot(to_camera, fixed_axis));
        // Right vector is perpendicular to both
        let right = cross(fixed_axis, camera_flat);
        // Build world-space offset from quad coordinates
        billboard_right = right;
        billboard_up = fixed_axis;
        use_world_billboard = true;
    }}"#,
                axis.x, axis.y, axis.z
            ),

            VertexEffect::BillboardFixed { forward, up } => format!(
                r#"
    // Fixed billboard - no camera facing
    {{
        let fwd = normalize(vec3<f32>({}f, {}f, {}f));
        let up_dir = normalize(vec3<f32>({}f, {}f, {}f));
        let right = cross(up_dir, fwd);
        billboard_right = right;
        billboard_up = up_dir;
        use_world_billboard = true;
    }}"#,
                forward.x, forward.y, forward.z,
                up.x, up.y, up.z
            ),

            VertexEffect::FacePoint { target } => format!(
                r#"
    // Face point - orient toward target
    {{
        let target_pos = vec3<f32>({}f, {}f, {}f);
        let to_target = normalize(target_pos - particle_pos);
        // Use world up as reference
        let world_up = vec3<f32>(0.0, 1.0, 0.0);
        var right = cross(world_up, to_target);
        if length(right) < 0.001 {{
            right = vec3<f32>(1.0, 0.0, 0.0);
        }} else {{
            right = normalize(right);
        }}
        let up_dir = cross(to_target, right);
        billboard_right = right;
        billboard_up = up_dir;
        use_world_billboard = true;
    }}"#,
                target.x, target.y, target.z
            ),
        }
    }
}

/// Combine multiple vertex effects into a single WGSL vertex shader body.
pub fn combine_vertex_effects(effects: &[VertexEffect], color_expr: &str) -> String {
    if effects.is_empty() {
        // Default vertex body when no effects
        return format!(
            r#"    let world_pos = vec4<f32>(particle_pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;

    clip_pos.x += quad_pos.x * particle_size * clip_pos.w;
    clip_pos.y += quad_pos.y * particle_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = {color_expr};
    out.uv = quad_pos;

    return out;"#
        );
    }

    // Check if any billboard effects are used
    let has_billboard = effects.iter().any(|e| matches!(e,
        VertexEffect::BillboardCylindrical { .. } |
        VertexEffect::BillboardFixed { .. } |
        VertexEffect::FacePoint { .. }
    ));

    // Generate effect code
    let effects_code: String = effects.iter().map(|e| e.to_wgsl()).collect();

    // Billboard variables initialization (only if needed)
    let billboard_vars = if has_billboard {
        r#"
    var use_world_billboard = false;
    var billboard_right = vec3<f32>(1.0, 0.0, 0.0);
    var billboard_up = vec3<f32>(0.0, 1.0, 0.0);"#
    } else {
        ""
    };

    // Final transformation - use world billboard if enabled
    let final_transform = if has_billboard {
        r#"    // Final transformation
    let final_pos = particle_pos + pos_offset;
    let final_size = particle_size * size_mult;

    let world_pos = vec4<f32>(final_pos, 1.0);

    if use_world_billboard {
        // World-space billboarding
        let world_offset = billboard_right * rotated_quad.x * final_size
                         + billboard_up * rotated_quad.y * final_size;
        let offset_world_pos = vec4<f32>(final_pos + world_offset, 1.0);
        out.clip_position = uniforms.view_proj * offset_world_pos;
    } else {
        // Screen-space billboarding (default)
        var clip_pos = uniforms.view_proj * world_pos;
        clip_pos.x += rotated_quad.x * final_size * clip_pos.w;
        clip_pos.y += rotated_quad.y * final_size * clip_pos.w;
        out.clip_position = clip_pos;
    }

    out.color = color_mod;
    out.uv = rotated_quad;

    return out;"#
    } else {
        r#"    // Final transformation
    let final_pos = particle_pos + pos_offset;
    let final_size = particle_size * size_mult;

    let world_pos = vec4<f32>(final_pos, 1.0);
    var clip_pos = uniforms.view_proj * world_pos;

    clip_pos.x += rotated_quad.x * final_size * clip_pos.w;
    clip_pos.y += rotated_quad.y * final_size * clip_pos.w;

    out.clip_position = clip_pos;
    out.color = color_mod;
    out.uv = rotated_quad;

    return out;"#
    };

    format!(
        r#"    // Initialize effect variables
    var pos_offset = vec3<f32>(0.0);
    var rotated_quad = quad_pos;
    var size_mult = 1.0;
    var color_mod = {color_expr};{billboard_vars}

    // Apply vertex effects
{effects_code}

{final_transform}"#
    )
}
