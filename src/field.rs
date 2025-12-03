//! 3D spatial fields for particle-environment interaction.
//!
//! Fields provide persistent spatial data that particles can read from and write to.
//! Unlike the inbox system (particle-to-particle), fields are spatially indexed and
//! persist independently of particles.
//!
//! # Use Cases
//!
//! - **Pheromone trails**: Particles deposit chemicals, others follow gradients
//! - **Density fields**: Accumulate particle presence for fluid-like behavior
//! - **Temperature/heat**: Particles emit/absorb heat from spatial field
//! - **Flow fields**: Pre-computed or dynamically generated velocity guidance
//!
//! # Single Field Example
//!
//! ```ignore
//! Simulation::<Agent>::new()
//!     .with_field("pheromone", FieldConfig::new(64).with_decay(0.98).with_blur(0.1))
//!     .with_rule(Rule::Custom(r#"
//!         // Deposit pheromone at current position
//!         field_write(0u, p.position, 0.1);
//!
//!         // Sample and follow gradient
//!         let grad = field_gradient(0u, p.position, 0.05);
//!         p.velocity += normalize(grad) * 0.5 * uniforms.delta_time;
//!     "#.into()))
//!     .run();
//! ```
//!
//! # Multiple Fields Example
//!
//! Each field can have independent resolution, decay, and blur settings:
//!
//! ```ignore
//! Simulation::<Agent>::new()
//!     .with_field("food", FieldConfig::new(64).with_decay(0.99))       // Index 0
//!     .with_field("danger", FieldConfig::new(32).with_decay(0.9))      // Index 1
//!     .with_rule(Rule::Custom(r#"
//!         // Read from different fields by index
//!         let food = field_read(0u, p.position);
//!         let danger = field_read(1u, p.position);
//!
//!         // Seek food, avoid danger
//!         let food_grad = field_gradient(0u, p.position, 0.05);
//!         let danger_grad = field_gradient(1u, p.position, 0.05);
//!         p.velocity += food_grad * 2.0 - danger_grad * 5.0;
//!     "#.into()))
//!     .run();
//! ```

/// Configuration for a 3D spatial field.
///
/// Fields are 3D grids that particles can read from and write to.
/// Each frame, the field is processed: blur (diffusion), then decay.
#[derive(Clone, Debug)]
pub struct FieldConfig {
    /// Grid resolution per axis (total cells = resolution³).
    /// Higher = more detail but more memory. Typical: 32, 64, 128.
    pub resolution: u32,

    /// World-space extent of the field (cube from -extent to +extent).
    /// Should match or exceed your simulation bounds.
    pub world_extent: f32,

    /// Per-frame decay multiplier (0.0-1.0).
    /// 0.98 = slow decay, 0.5 = fast decay, 1.0 = no decay.
    pub decay: f32,

    /// Blur strength per frame (0.0-1.0).
    /// Controls diffusion rate. 0.0 = no blur, 1.0 = max blur.
    pub blur: f32,

    /// Number of blur iterations per frame.
    /// More iterations = smoother but more expensive.
    pub blur_iterations: u32,
}

impl FieldConfig {
    /// Create a new field configuration with the given resolution.
    ///
    /// Default values:
    /// - `world_extent`: 1.0 (cube from -1 to +1)
    /// - `decay`: 0.99 (slow decay)
    /// - `blur`: 0.1 (light diffusion)
    /// - `blur_iterations`: 1
    ///
    /// # Memory Usage
    ///
    /// - 32³ = 128KB
    /// - 64³ = 1MB
    /// - 128³ = 8MB
    ///
    /// # Example
    ///
    /// ```ignore
    /// let field = FieldConfig::new(64);
    /// ```
    pub fn new(resolution: u32) -> Self {
        assert!(resolution >= 8, "Field resolution must be at least 8");
        assert!(resolution <= 256, "Field resolution must be at most 256 (memory limits)");
        Self {
            resolution,
            world_extent: 1.0,
            decay: 0.99,
            blur: 0.1,
            blur_iterations: 1,
        }
    }

    /// Set the world-space extent of the field.
    ///
    /// The field covers a cube from `-extent` to `+extent` on all axes.
    /// Should match or exceed your simulation bounds.
    pub fn with_extent(mut self, extent: f32) -> Self {
        self.world_extent = extent;
        self
    }

    /// Set the decay rate (0.0-1.0).
    ///
    /// Applied each frame: `field *= decay`
    ///
    /// - 1.0 = no decay (field persists forever)
    /// - 0.99 = slow decay
    /// - 0.9 = fast decay
    /// - 0.0 = instant decay (field clears each frame)
    pub fn with_decay(mut self, decay: f32) -> Self {
        self.decay = decay.clamp(0.0, 1.0);
        self
    }

    /// Set the blur/diffusion strength (0.0-1.0).
    ///
    /// Controls how much values spread to neighboring cells each frame.
    ///
    /// - 0.0 = no diffusion
    /// - 0.1 = light spread
    /// - 0.5 = heavy spread
    /// - 1.0 = maximum diffusion
    pub fn with_blur(mut self, blur: f32) -> Self {
        self.blur = blur.clamp(0.0, 1.0);
        self
    }

    /// Set the number of blur iterations per frame.
    ///
    /// More iterations = smoother diffusion but more expensive.
    /// Usually 1-3 is sufficient.
    pub fn with_blur_iterations(mut self, iterations: u32) -> Self {
        self.blur_iterations = iterations.max(1);
        self
    }

    /// Total number of cells in the field.
    pub fn total_cells(&self) -> u32 {
        self.resolution * self.resolution * self.resolution
    }

    /// Memory size in bytes (for the main field buffer).
    pub fn memory_size(&self) -> usize {
        self.total_cells() as usize * 4 // f32 per cell
    }
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self::new(64)
    }
}

/// Registry holding all field configurations for a simulation.
#[derive(Clone, Debug, Default)]
pub struct FieldRegistry {
    /// Named fields in registration order.
    pub fields: Vec<(String, FieldConfig)>,
}

impl FieldRegistry {
    /// Create a new empty field registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field to the registry.
    ///
    /// Fields are accessed by index in shaders (0, 1, 2, ...).
    pub fn add(&mut self, name: impl Into<String>, config: FieldConfig) {
        self.fields.push((name.into(), config));
    }

    /// Get number of registered fields.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Get field index by name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|(n, _)| n == name)
    }

    /// Generate WGSL code for field access functions.
    ///
    /// Returns helper functions and buffer declarations.
    pub fn to_wgsl_declarations(&self, base_binding: u32) -> String {
        if self.fields.is_empty() {
            return String::new();
        }

        let mut code = String::new();
        let field_count = self.fields.len();

        // Generate buffer bindings for each field
        // We use two buffers per field: write (atomic) and read (f32)
        let mut binding = base_binding;
        for (i, (name, config)) in self.fields.iter().enumerate() {
            code.push_str(&format!(
                "// Field {}: '{}' ({}³ = {} cells)\n",
                i, name, config.resolution, config.total_cells()
            ));

            // Write buffer (atomic for particle deposits)
            code.push_str(&format!(
                "@group(2) @binding({})\nvar<storage, read_write> field_{}_write: array<atomic<i32>>;\n",
                binding, i
            ));
            binding += 1;

            // Read buffer (f32 for particle sampling)
            code.push_str(&format!(
                "@group(2) @binding({})\nvar<storage, read> field_{}_read: array<f32>;\n",
                binding, i
            ));
            binding += 1;

            code.push_str("\n");
        }

        // Generate field parameters struct (must match FieldParamsGpu in field_gpu.rs)
        code.push_str(r#"struct FieldParams {
    resolution: u32,
    total_cells: u32,
    extent: f32,
    decay: f32,
    blur: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};
"#);

        // Field params storage buffer (array of FieldParams for all fields)
        code.push_str(&format!(
            "\n@group(2) @binding({})\nvar<storage, read> field_params: array<FieldParams>;\n\n",
            binding
        ));

        // Generate helper functions with dynamic switch cases
        code.push_str(&self.generate_helper_functions(field_count));

        code
    }

    /// Generate WGSL helper functions for field access.
    fn generate_helper_functions(&self, field_count: usize) -> String {
        let mut code = String::new();

        // Fixed-point scale constant
        code.push_str(r#"
// Fixed-point scale for field writes (16.16 format)
const FIELD_SCALE: f32 = 65536.0;

// Convert world position to field cell index for a specific field
fn field_pos_to_idx(field_idx: u32, pos: vec3<f32>) -> u32 {
    let params = field_params[field_idx];
    let resolution = params.resolution;
    let extent = params.extent;

    // Map world position to 0..resolution
    let half_size = extent;
    let normalized = (pos + vec3<f32>(half_size)) / (2.0 * half_size);
    let clamped = clamp(normalized, vec3<f32>(0.0), vec3<f32>(0.999));
    let cell = vec3<u32>(clamped * f32(resolution));

    return cell.x + cell.y * resolution + cell.z * resolution * resolution;
}

"#);

        // Generate field_write function with switch cases for all fields
        code.push_str(r#"// Write a value to the field at the given world position (atomic accumulate)
fn field_write(field_idx: u32, pos: vec3<f32>, value: f32) {
    let idx = field_pos_to_idx(field_idx, pos);
    let scaled = i32(clamp(value, -32768.0, 32767.0) * FIELD_SCALE);

    switch field_idx {
"#);

        for i in 0..field_count {
            code.push_str(&format!(
                "        case {}u: {{ atomicAdd(&field_{}_write[idx], scaled); }}\n",
                i, i
            ));
        }

        code.push_str(r#"        default: {}
    }
}

"#);

        // Generate field_read function with switch cases for all fields
        code.push_str(r#"// Read a value from the field at the given world position (trilinear interpolation)
fn field_read(field_idx: u32, pos: vec3<f32>) -> f32 {
    let params = field_params[field_idx];
    let resolution = params.resolution;
    let extent = params.extent;

    // Map to float cell coordinates
    let half_size = extent;
    let normalized = (pos + vec3<f32>(half_size)) / (2.0 * half_size);
    let float_cell = clamp(normalized, vec3<f32>(0.0), vec3<f32>(0.999)) * f32(resolution);

    // Get integer cell and fraction
    let cell = vec3<u32>(floor(float_cell));
    let frac = fract(float_cell);

    // Sample 8 corners for trilinear interpolation
    let res = resolution;
    let c000 = cell.x + cell.y * res + cell.z * res * res;
    let c100 = min(cell.x + 1u, res - 1u) + cell.y * res + cell.z * res * res;
    let c010 = cell.x + min(cell.y + 1u, res - 1u) * res + cell.z * res * res;
    let c110 = min(cell.x + 1u, res - 1u) + min(cell.y + 1u, res - 1u) * res + cell.z * res * res;
    let c001 = cell.x + cell.y * res + min(cell.z + 1u, res - 1u) * res * res;
    let c101 = min(cell.x + 1u, res - 1u) + cell.y * res + min(cell.z + 1u, res - 1u) * res * res;
    let c011 = cell.x + min(cell.y + 1u, res - 1u) * res + min(cell.z + 1u, res - 1u) * res * res;
    let c111 = min(cell.x + 1u, res - 1u) + min(cell.y + 1u, res - 1u) * res + min(cell.z + 1u, res - 1u) * res * res;

    var v000: f32; var v100: f32; var v010: f32; var v110: f32;
    var v001: f32; var v101: f32; var v011: f32; var v111: f32;

    switch field_idx {
"#);

        for i in 0..field_count {
            code.push_str(&format!(
                r#"        case {}u: {{
            v000 = field_{}_read[c000]; v100 = field_{}_read[c100];
            v010 = field_{}_read[c010]; v110 = field_{}_read[c110];
            v001 = field_{}_read[c001]; v101 = field_{}_read[c101];
            v011 = field_{}_read[c011]; v111 = field_{}_read[c111];
        }}
"#,
                i, i, i, i, i, i, i, i, i
            ));
        }

        code.push_str(r#"        default: {
            v000 = 0.0; v100 = 0.0; v010 = 0.0; v110 = 0.0;
            v001 = 0.0; v101 = 0.0; v011 = 0.0; v111 = 0.0;
        }
    }

    // Trilinear interpolation
    let v00 = mix(v000, v100, frac.x);
    let v10 = mix(v010, v110, frac.x);
    let v01 = mix(v001, v101, frac.x);
    let v11 = mix(v011, v111, frac.x);
    let v0 = mix(v00, v10, frac.y);
    let v1 = mix(v01, v11, frac.y);
    return mix(v0, v1, frac.z);
}

"#);

        // field_gradient function (uses field_read, so no changes needed)
        code.push_str(r#"// Sample field gradient (for steering toward higher values)
fn field_gradient(field_idx: u32, pos: vec3<f32>, epsilon: f32) -> vec3<f32> {
    let dx = field_read(field_idx, pos + vec3<f32>(epsilon, 0.0, 0.0))
           - field_read(field_idx, pos - vec3<f32>(epsilon, 0.0, 0.0));
    let dy = field_read(field_idx, pos + vec3<f32>(0.0, epsilon, 0.0))
           - field_read(field_idx, pos - vec3<f32>(0.0, epsilon, 0.0));
    let dz = field_read(field_idx, pos + vec3<f32>(0.0, 0.0, epsilon))
           - field_read(field_idx, pos - vec3<f32>(0.0, 0.0, epsilon));
    return vec3<f32>(dx, dy, dz) / (2.0 * epsilon);
}
"#);

        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== FieldConfig Tests ==========

    #[test]
    fn test_field_config_new() {
        let config = FieldConfig::new(64);
        assert_eq!(config.resolution, 64);
        assert_eq!(config.world_extent, 1.0);
        assert!((config.decay - 0.99).abs() < 0.001);
        assert!((config.blur - 0.1).abs() < 0.001);
        assert_eq!(config.blur_iterations, 1);
    }

    #[test]
    fn test_field_config_builder() {
        let config = FieldConfig::new(32)
            .with_extent(2.0)
            .with_decay(0.95)
            .with_blur(0.3)
            .with_blur_iterations(2);

        assert_eq!(config.resolution, 32);
        assert!((config.world_extent - 2.0).abs() < 0.001);
        assert!((config.decay - 0.95).abs() < 0.001);
        assert!((config.blur - 0.3).abs() < 0.001);
        assert_eq!(config.blur_iterations, 2);
    }

    #[test]
    fn test_field_config_total_cells() {
        let config = FieldConfig::new(32);
        assert_eq!(config.total_cells(), 32 * 32 * 32);

        let config = FieldConfig::new(64);
        assert_eq!(config.total_cells(), 64 * 64 * 64);
    }

    #[test]
    fn test_field_config_memory_size() {
        let config = FieldConfig::new(32);
        assert_eq!(config.memory_size(), 32 * 32 * 32 * 4); // f32 = 4 bytes
    }

    #[test]
    fn test_field_config_decay_clamping() {
        let config = FieldConfig::new(32).with_decay(1.5);
        assert!((config.decay - 1.0).abs() < 0.001);

        let config = FieldConfig::new(32).with_decay(-0.5);
        assert!((config.decay - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_field_config_blur_clamping() {
        let config = FieldConfig::new(32).with_blur(2.0);
        assert!((config.blur - 1.0).abs() < 0.001);

        let config = FieldConfig::new(32).with_blur(-0.5);
        assert!((config.blur - 0.0).abs() < 0.001);
    }

    #[test]
    #[should_panic(expected = "resolution must be at least 8")]
    fn test_field_config_min_resolution() {
        FieldConfig::new(4);
    }

    #[test]
    #[should_panic(expected = "resolution must be at most 256")]
    fn test_field_config_max_resolution() {
        FieldConfig::new(512);
    }

    // ========== FieldRegistry Tests ==========

    #[test]
    fn test_field_registry_new() {
        let registry = FieldRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_field_registry_add() {
        let mut registry = FieldRegistry::new();
        registry.add("pheromone", FieldConfig::new(64));

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_field_registry_index_of() {
        let mut registry = FieldRegistry::new();
        registry.add("food", FieldConfig::new(32));
        registry.add("danger", FieldConfig::new(64));
        registry.add("heat", FieldConfig::new(48));

        assert_eq!(registry.index_of("food"), Some(0));
        assert_eq!(registry.index_of("danger"), Some(1));
        assert_eq!(registry.index_of("heat"), Some(2));
        assert_eq!(registry.index_of("unknown"), None);
    }

    #[test]
    fn test_field_registry_multiple_fields() {
        let mut registry = FieldRegistry::new();
        registry.add("field_a", FieldConfig::new(32));
        registry.add("field_b", FieldConfig::new(64).with_decay(0.9));
        registry.add("field_c", FieldConfig::new(48).with_blur(0.5));

        assert_eq!(registry.len(), 3);
    }

    // ========== WGSL Generation Tests ==========

    #[test]
    fn test_empty_registry_wgsl() {
        let registry = FieldRegistry::new();
        let wgsl = registry.to_wgsl_declarations(0);
        assert!(wgsl.is_empty());
    }

    #[test]
    fn test_single_field_wgsl_structure() {
        let mut registry = FieldRegistry::new();
        registry.add("pheromone", FieldConfig::new(64));

        let wgsl = registry.to_wgsl_declarations(0);

        // Should contain buffer declarations
        assert!(wgsl.contains("field_0_write"));
        assert!(wgsl.contains("field_0_read"));
        assert!(wgsl.contains("array<atomic<i32>>"));
        assert!(wgsl.contains("array<f32>"));

        // Should contain FieldParams struct
        assert!(wgsl.contains("struct FieldParams"));
        assert!(wgsl.contains("resolution: u32"));
        assert!(wgsl.contains("extent: f32"));
        assert!(wgsl.contains("decay: f32"));
        assert!(wgsl.contains("blur: f32"));

        // Should contain helper functions
        assert!(wgsl.contains("fn field_write"));
        assert!(wgsl.contains("fn field_read"));
        assert!(wgsl.contains("fn field_gradient"));
        assert!(wgsl.contains("fn field_pos_to_idx"));
    }

    #[test]
    fn test_multi_field_wgsl_buffers() {
        let mut registry = FieldRegistry::new();
        registry.add("food", FieldConfig::new(32));
        registry.add("danger", FieldConfig::new(64));
        registry.add("heat", FieldConfig::new(48));

        let wgsl = registry.to_wgsl_declarations(0);

        // Should have buffers for all 3 fields
        assert!(wgsl.contains("field_0_write"));
        assert!(wgsl.contains("field_0_read"));
        assert!(wgsl.contains("field_1_write"));
        assert!(wgsl.contains("field_1_read"));
        assert!(wgsl.contains("field_2_write"));
        assert!(wgsl.contains("field_2_read"));

        // Should have switch cases for all fields
        assert!(wgsl.contains("case 0u"));
        assert!(wgsl.contains("case 1u"));
        assert!(wgsl.contains("case 2u"));
    }

    #[test]
    fn test_field_wgsl_binding_numbers() {
        let mut registry = FieldRegistry::new();
        registry.add("a", FieldConfig::new(32));
        registry.add("b", FieldConfig::new(32));

        let wgsl = registry.to_wgsl_declarations(0);

        // Bindings should be sequential: 0, 1, 2, 3, 4
        // Field 0: binding 0 (write), binding 1 (read)
        // Field 1: binding 2 (write), binding 3 (read)
        // Params: binding 4
        assert!(wgsl.contains("@binding(0)"));
        assert!(wgsl.contains("@binding(1)"));
        assert!(wgsl.contains("@binding(2)"));
        assert!(wgsl.contains("@binding(3)"));
        assert!(wgsl.contains("@binding(4)"));
    }

    #[test]
    fn test_field_wgsl_base_binding_offset() {
        let mut registry = FieldRegistry::new();
        registry.add("test", FieldConfig::new(32));

        let wgsl = registry.to_wgsl_declarations(5);

        // Bindings should start at 5
        assert!(wgsl.contains("@binding(5)"));
        assert!(wgsl.contains("@binding(6)"));
        assert!(wgsl.contains("@binding(7)"));
    }

    /// Validates WGSL code using naga.
    fn validate_wgsl(code: &str) -> Result<(), String> {
        let module = naga::front::wgsl::parse_str(code)
            .map_err(|e| format!("WGSL parse error: {:?}", e))?;

        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .map_err(|e| format!("WGSL validation error: {:?}", e))?;

        Ok(())
    }

    /// Wraps field WGSL in a minimal compute shader for validation.
    fn wrap_field_wgsl(field_wgsl: &str) -> String {
        format!(
            r#"
struct Particle {{
    position: vec3<f32>,
    velocity: vec3<f32>,
}};

struct Uniforms {{
    delta_time: f32,
    time: f32,
}};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

{field_wgsl}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let index = global_id.x;
    var p = particles[index];

    // Test field operations
    field_write(0u, p.position, 0.1);
    let val = field_read(0u, p.position);
    let grad = field_gradient(0u, p.position, 0.01);

    p.velocity += grad * val;
    particles[index] = p;
}}
"#,
            field_wgsl = field_wgsl
        )
    }

    #[test]
    fn test_single_field_wgsl_validates() {
        let mut registry = FieldRegistry::new();
        registry.add("pheromone", FieldConfig::new(64));

        let wgsl = registry.to_wgsl_declarations(0);
        let shader = wrap_field_wgsl(&wgsl);

        validate_wgsl(&shader).expect("Single field WGSL should be valid");
    }

    #[test]
    fn test_multi_field_wgsl_validates() {
        let mut registry = FieldRegistry::new();
        registry.add("food", FieldConfig::new(32));
        registry.add("danger", FieldConfig::new(64));

        let wgsl = registry.to_wgsl_declarations(0);

        // Create a shader that uses both fields
        let shader = format!(
            r#"
struct Particle {{
    position: vec3<f32>,
    velocity: vec3<f32>,
}};

struct Uniforms {{
    delta_time: f32,
    time: f32,
}};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

{wgsl}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let index = global_id.x;
    var p = particles[index];

    // Write to both fields
    field_write(0u, p.position, 0.1);
    field_write(1u, p.position, 0.2);

    // Read from both fields
    let food = field_read(0u, p.position);
    let danger = field_read(1u, p.position);

    // Use gradients from both
    let food_grad = field_gradient(0u, p.position, 0.01);
    let danger_grad = field_gradient(1u, p.position, 0.01);

    p.velocity += food_grad * food - danger_grad * danger;
    particles[index] = p;
}}
"#,
            wgsl = wgsl
        );

        validate_wgsl(&shader).expect("Multi-field WGSL should be valid");
    }

    #[test]
    fn test_field_names_in_comments() {
        let mut registry = FieldRegistry::new();
        registry.add("pheromone_trail", FieldConfig::new(64));

        let wgsl = registry.to_wgsl_declarations(0);

        // Field name should appear in comments
        assert!(wgsl.contains("pheromone_trail"));
    }
}
