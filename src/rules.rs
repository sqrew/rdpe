//! Particle behavior rules

use glam::Vec3;

/// Rules that can be applied to particles each frame.
/// These are translated into WGSL compute shader code.
#[derive(Clone, Debug)]
pub enum Rule {
    /// Apply constant gravity (negative Y acceleration)
    Gravity(f32),

    /// Bounce off bounding box walls
    BounceWalls,

    /// Apply drag/friction to velocity (0.0 = no drag, 1.0 = full stop)
    Drag(f32),

    /// Constant acceleration in a direction
    Acceleration(Vec3),

    /// Attract towards a point with given strength
    AttractTo { point: Vec3, strength: f32 },

    /// Repel from a point with given strength and radius
    RepelFrom { point: Vec3, strength: f32, radius: f32 },

    // --- Neighbor-based rules (require spatial hashing) ---

    /// Particle-particle collision with radius and response strength
    Collide { radius: f32, response: f32 },

    /// Separation (avoid crowding neighbors)
    Separate { radius: f32, strength: f32 },

    /// Cohesion (steer towards average position of neighbors)
    Cohere { radius: f32, strength: f32 },

    /// Alignment (match velocity of neighbors)
    Align { radius: f32, strength: f32 },

    /// Custom WGSL code snippet (advanced users)
    Custom(String),
}

impl Rule {
    /// Returns true if this rule requires spatial hashing (neighbor queries)
    pub fn requires_neighbors(&self) -> bool {
        matches!(
            self,
            Rule::Collide { .. }
                | Rule::Separate { .. }
                | Rule::Cohere { .. }
                | Rule::Align { .. }
        )
    }

    /// Generate WGSL code for this rule (non-neighbor rules only)
    pub(crate) fn to_wgsl(&self, bounds: f32) -> String {
        match self {
            Rule::Gravity(g) => format!(
                "    // Gravity\n    p.velocity.y -= {} * uniforms.delta_time;",
                g
            ),

            Rule::BounceWalls => format!(
                r#"    // Bounce off walls
    if p.position.x < -{bounds} {{
        p.position.x = -{bounds};
        p.velocity.x = abs(p.velocity.x);
    }} else if p.position.x > {bounds} {{
        p.position.x = {bounds};
        p.velocity.x = -abs(p.velocity.x);
    }}
    if p.position.y < -{bounds} {{
        p.position.y = -{bounds};
        p.velocity.y = abs(p.velocity.y);
    }} else if p.position.y > {bounds} {{
        p.position.y = {bounds};
        p.velocity.y = -abs(p.velocity.y);
    }}
    if p.position.z < -{bounds} {{
        p.position.z = -{bounds};
        p.velocity.z = abs(p.velocity.z);
    }} else if p.position.z > {bounds} {{
        p.position.z = {bounds};
        p.velocity.z = -abs(p.velocity.z);
    }}"#,
                bounds = bounds
            ),

            Rule::Drag(d) => format!(
                "    // Drag\n    p.velocity *= 1.0 - ({} * uniforms.delta_time);",
                d
            ),

            Rule::Acceleration(acc) => format!(
                "    // Acceleration\n    p.velocity += vec3<f32>({}, {}, {}) * uniforms.delta_time;",
                acc.x, acc.y, acc.z
            ),

            Rule::AttractTo { point, strength } => format!(
                r#"    // Attract to point
    {{
        let attract_dir = vec3<f32>({}, {}, {}) - p.position;
        let dist = length(attract_dir);
        if dist > 0.001 {{
            p.velocity += normalize(attract_dir) * {} * uniforms.delta_time;
        }}
    }}"#,
                point.x, point.y, point.z, strength
            ),

            Rule::RepelFrom { point, strength, radius } => format!(
                r#"    // Repel from point
    {{
        let repel_dir = p.position - vec3<f32>({}, {}, {});
        let dist = length(repel_dir);
        if dist < {} && dist > 0.001 {{
            let force = ({} - dist) / {} * {};
            p.velocity += normalize(repel_dir) * force * uniforms.delta_time;
        }}
    }}"#,
                point.x, point.y, point.z, radius, radius, radius, strength
            ),

            Rule::Custom(code) => format!("    // Custom rule\n{}", code),

            // Neighbor rules generate code through to_neighbor_wgsl
            Rule::Collide { .. }
            | Rule::Separate { .. }
            | Rule::Cohere { .. }
            | Rule::Align { .. } => String::new(),
        }
    }

    /// Generate WGSL code for neighbor-based rules
    /// This code runs inside the neighbor iteration loop
    pub(crate) fn to_neighbor_wgsl(&self) -> String {
        match self {
            Rule::Collide { radius, response } => format!(
                r#"            // Collision
            if neighbor_dist < {radius} && neighbor_dist > 0.0001 {{
                let overlap = {radius} - neighbor_dist;
                let push = neighbor_dir * (overlap * {response});
                p.velocity += push;
            }}"#
            ),

            Rule::Separate { radius, strength } => format!(
                r#"            // Separation
            if neighbor_dist < {radius} && neighbor_dist > 0.0001 {{
                let force = ({radius} - neighbor_dist) / {radius};
                p.velocity += neighbor_dir * force * {strength} * uniforms.delta_time;
            }}"#
            ),

            Rule::Cohere { radius, strength: _ } => format!(
                r#"            // Cohesion (accumulate for averaging)
            if neighbor_dist < {radius} {{
                cohesion_sum += neighbor_pos;
                cohesion_count += 1.0;
            }}"#
            ),

            Rule::Align { radius, strength } => format!(
                r#"            // Alignment (accumulate for averaging)
            if neighbor_dist < {radius} {{
                alignment_sum += neighbor_vel;
                alignment_count += 1.0;
            }}"#
            ),

            _ => String::new(),
        }
    }

    /// Generate post-neighbor-loop code (for averaging rules like cohere/align)
    pub(crate) fn to_post_neighbor_wgsl(&self) -> String {
        match self {
            Rule::Cohere { strength, .. } => format!(
                r#"    // Apply cohesion
    if cohesion_count > 0.0 {{
        let center = cohesion_sum / cohesion_count;
        let to_center = center - p.position;
        p.velocity += normalize(to_center) * {strength} * uniforms.delta_time;
    }}"#
            ),

            Rule::Align { strength, .. } => format!(
                r#"    // Apply alignment
    if alignment_count > 0.0 {{
        let avg_vel = alignment_sum / alignment_count;
        p.velocity += (avg_vel - p.velocity) * {strength} * uniforms.delta_time;
    }}"#
            ),

            _ => String::new(),
        }
    }
}
