//! Interaction matrix for type-based particle forces.
//!
//! The interaction matrix defines how different particle types affect each other.
//! This is the foundation of "particle life" simulations where complex emergent
//! behavior arises from simple attraction/repulsion rules.
//!
//! # Example
//!
//! ```ignore
//! #[derive(ParticleType, Clone, Copy)]
//! enum Species { Red, Green, Blue }
//!
//! Simulation::<Particle>::new()
//!     .with_interactions::<Species>(|m| {
//!         // Red attracts Green, repels Blue
//!         m.set(Red, Green, 1.0, 0.3);
//!         m.set(Red, Blue, -0.5, 0.2);
//!
//!         // Green attracts itself
//!         m.set(Green, Green, 0.8, 0.25);
//!     })
//!     .run();
//! ```

/// Interaction matrix storing force relationships between particle types.
///
/// Each cell `[self_type][other_type]` contains:
/// - `strength`: positive = attract, negative = repel, zero = ignore
/// - `radius`: interaction range
#[derive(Clone, Debug)]
pub struct InteractionMatrix {
    /// Flattened matrix: [self_type * num_types + other_type] = (strength, radius)
    interactions: Vec<(f32, f32)>,
    /// Number of particle types.
    num_types: usize,
    /// Maximum interaction radius (for spatial hashing).
    max_radius: f32,
}

impl InteractionMatrix {
    /// Create a new interaction matrix for `num_types` particle types.
    ///
    /// All interactions start as ignore (strength=0, radius=0).
    pub fn new(num_types: usize) -> Self {
        Self {
            interactions: vec![(0.0, 0.0); num_types * num_types],
            num_types,
            max_radius: 0.0,
        }
    }

    /// Set the interaction when `self_type` encounters `other_type`.
    ///
    /// - `strength > 0`: attraction (pulls toward)
    /// - `strength < 0`: repulsion (pushes away)
    /// - `strength = 0`: ignore
    /// - `radius`: how far the interaction reaches
    ///
    /// # Example
    ///
    /// ```ignore
    /// m.set(Predator, Prey, 2.0, 0.5);   // Predator attracted to Prey
    /// m.set(Prey, Predator, -3.0, 0.4);  // Prey repelled by Predator
    /// ```
    pub fn set<T: Into<u32>, U: Into<u32>>(
        &mut self,
        self_type: T,
        other_type: U,
        strength: f32,
        radius: f32,
    ) {
        let s = self_type.into() as usize;
        let o = other_type.into() as usize;
        if s < self.num_types && o < self.num_types {
            self.interactions[s * self.num_types + o] = (strength, radius);
            if radius > self.max_radius {
                self.max_radius = radius;
            }
        }
    }

    /// Convenience: set attraction between types.
    ///
    /// Equivalent to `set(self_type, other_type, strength.abs(), radius)`.
    pub fn attract<T: Into<u32>, U: Into<u32>>(
        &mut self,
        self_type: T,
        other_type: U,
        strength: f32,
        radius: f32,
    ) {
        self.set(self_type, other_type, strength.abs(), radius);
    }

    /// Convenience: set repulsion between types.
    ///
    /// Equivalent to `set(self_type, other_type, -strength.abs(), radius)`.
    pub fn repel<T: Into<u32>, U: Into<u32>>(
        &mut self,
        self_type: T,
        other_type: U,
        strength: f32,
        radius: f32,
    ) {
        self.set(self_type, other_type, -strength.abs(), radius);
    }

    /// Set symmetric interaction (both types affect each other the same way).
    ///
    /// Useful for mutual attraction/repulsion.
    pub fn set_symmetric<T: Into<u32> + Copy, U: Into<u32> + Copy>(
        &mut self,
        type_a: T,
        type_b: U,
        strength: f32,
        radius: f32,
    ) {
        self.set(type_a, type_b, strength, radius);
        self.set(type_b, type_a, strength, radius);
    }

    /// Get number of types in this matrix.
    pub fn num_types(&self) -> usize {
        self.num_types
    }

    /// Get the maximum interaction radius.
    pub fn max_radius(&self) -> f32 {
        self.max_radius
    }

    /// Get raw interaction data for GPU upload.
    pub fn data(&self) -> &[(f32, f32)] {
        &self.interactions
    }

    /// Generate WGSL code for initializing interaction variables.
    ///
    /// This goes before the neighbor loop.
    pub(crate) fn to_wgsl_init(&self) -> String {
        // Generate the lookup table as WGSL constants
        let mut table_entries = Vec::new();
        for s in 0..self.num_types {
            for o in 0..self.num_types {
                let (strength, radius) = self.interactions[s * self.num_types + o];
                table_entries.push(format!("vec2<f32>({strength}, {radius})"));
            }
        }

        let table_str = table_entries.join(", ");
        let num_types = self.num_types;
        let total = self.num_types * self.num_types;

        format!(
            r#"    // Interaction matrix lookup table
    let interaction_table = array<vec2<f32>, {total}>(
        {table_str}
    );
    let my_type = p.particle_type;
    var interaction_force = vec3<f32>(0.0);
    let interaction_num_types = {num_types}u;"#
        )
    }

    /// Generate WGSL code for the neighbor loop body.
    ///
    /// This runs inside the neighbor loop with access to:
    /// - `other` - the neighbor particle
    /// - `neighbor_dist` - distance to neighbor
    /// - `neighbor_dir` - direction toward self from neighbor
    pub(crate) fn to_wgsl_neighbor(&self) -> String {
        r#"            // Interaction matrix force
            let other_type = other.particle_type;
            let lookup_idx = my_type * interaction_num_types + other_type;
            let interaction = interaction_table[lookup_idx];
            let int_strength = interaction.x;
            let int_radius = interaction.y;

            if int_radius > 0.0 && neighbor_dist < int_radius && neighbor_dist > 0.001 {
                let falloff = 1.0 - (neighbor_dist / int_radius);
                let force_mag = int_strength * falloff * falloff;
                interaction_force += neighbor_dir * force_mag;
            }"#
        .to_string()
    }

    /// Generate WGSL code for applying accumulated interaction forces.
    ///
    /// This goes after the neighbor loop.
    pub(crate) fn to_wgsl_post(&self) -> String {
        "    // Apply interaction matrix forces\n    p.velocity += interaction_force * uniforms.delta_time;".to_string()
    }
}
