//! Rule matrix configuration for type-based particle interactions.
//!
//! The rule matrix defines what rules apply when particles of different types interact.
//! Each cell [i][j] contains a list of rules that apply when type i encounters type j.

use serde::{Deserialize, Serialize};
use crate::config::RuleConfig;

/// Default number of particle types.
pub const DEFAULT_NUM_TYPES: usize = 4;

/// Maximum number of particle types supported.
pub const MAX_NUM_TYPES: usize = 16;

/// Names/colors for particle types to make the UI more readable.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ParticleTypeInfo {
    /// Display name for this type.
    pub name: String,
    /// Display color (RGB, 0-1 range).
    pub color: [f32; 3],
}

impl ParticleTypeInfo {
    /// Default type colors (matching common particle life palettes).
    pub fn default_for_index(index: usize) -> Self {
        let (name, color) = match index {
            0 => ("Red", [1.0, 0.2, 0.2]),
            1 => ("Green", [0.2, 1.0, 0.2]),
            2 => ("Blue", [0.2, 0.4, 1.0]),
            3 => ("Yellow", [1.0, 1.0, 0.2]),
            4 => ("Cyan", [0.2, 1.0, 1.0]),
            5 => ("Magenta", [1.0, 0.2, 1.0]),
            6 => ("Orange", [1.0, 0.6, 0.2]),
            7 => ("Purple", [0.6, 0.2, 1.0]),
            8 => ("Pink", [1.0, 0.5, 0.7]),
            9 => ("Lime", [0.7, 1.0, 0.3]),
            10 => ("Teal", [0.2, 0.7, 0.7]),
            11 => ("Brown", [0.6, 0.4, 0.2]),
            12 => ("Gray", [0.5, 0.5, 0.5]),
            13 => ("White", [1.0, 1.0, 1.0]),
            14 => ("Navy", [0.1, 0.1, 0.5]),
            15 => ("Maroon", [0.5, 0.1, 0.1]),
            _ => ("Type", [0.5, 0.5, 0.5]),
        };
        Self {
            name: name.to_string(),
            color,
        }
    }
}

/// A cell in the rule matrix containing rules for a specific type pair.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RuleMatrixCell {
    /// Rules that apply when the "from" type encounters the "to" type.
    pub rules: Vec<RuleConfig>,
    /// Custom WGSL code executed for this type pair in the neighbor loop.
    /// Has access to: p, other, neighbor_dist, neighbor_dir, neighbor_pos, neighbor_vel, uniforms
    #[serde(default)]
    pub custom_code: String,
}

impl RuleMatrixCell {
    /// Create a new empty cell.
    pub fn new() -> Self {
        Self { rules: Vec::new(), custom_code: String::new() }
    }

    /// Create a cell with a single rule.
    pub fn with_rule(rule: RuleConfig) -> Self {
        Self { rules: vec![rule], custom_code: String::new() }
    }

    /// Create a cell with multiple rules.
    pub fn with_rules(rules: Vec<RuleConfig>) -> Self {
        Self { rules, custom_code: String::new() }
    }

    /// Create a cell with custom WGSL code.
    pub fn with_custom(code: impl Into<String>) -> Self {
        Self { rules: Vec::new(), custom_code: code.into() }
    }

    /// Check if this cell has any rules or custom code.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.custom_code.is_empty()
    }

    /// Check if this cell has custom code.
    pub fn has_custom_code(&self) -> bool {
        !self.custom_code.trim().is_empty()
    }

    /// Get the number of rules in this cell.
    pub fn len(&self) -> usize {
        self.rules.len()
    }
}

/// Complete rule matrix configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InteractionConfig {
    /// Whether the rule matrix is enabled.
    pub enabled: bool,
    /// Number of particle types (matrix is num_types x num_types).
    pub num_types: usize,
    /// The rule matrix, stored as a flattened row-major array.
    /// Index [i][j] = matrix[i * num_types + j]
    /// Cell [i][j] contains rules for when type i encounters type j.
    pub matrix: Vec<RuleMatrixCell>,
    /// Type information (names and colors for UI).
    #[serde(default)]
    pub type_info: Vec<ParticleTypeInfo>,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        let num_types = DEFAULT_NUM_TYPES;
        let matrix = vec![RuleMatrixCell::new(); num_types * num_types];
        let type_info = (0..num_types)
            .map(ParticleTypeInfo::default_for_index)
            .collect();
        Self {
            enabled: false,
            num_types,
            matrix,
            type_info,
        }
    }
}

impl InteractionConfig {
    /// Create a new config with the specified number of types.
    pub fn with_num_types(num_types: usize) -> Self {
        let num_types = num_types.min(MAX_NUM_TYPES);
        let matrix = vec![RuleMatrixCell::new(); num_types * num_types];
        let type_info = (0..num_types)
            .map(ParticleTypeInfo::default_for_index)
            .collect();
        Self {
            enabled: false,
            num_types,
            matrix,
            type_info,
        }
    }

    /// Resize the matrix to a new number of types, preserving existing cells.
    pub fn resize(&mut self, new_num_types: usize) {
        let new_num_types = new_num_types.min(MAX_NUM_TYPES);
        if new_num_types == self.num_types {
            return;
        }

        let mut new_matrix = vec![RuleMatrixCell::new(); new_num_types * new_num_types];

        // Copy existing cells
        let copy_size = self.num_types.min(new_num_types);
        for i in 0..copy_size {
            for j in 0..copy_size {
                new_matrix[i * new_num_types + j] = self.matrix[i * self.num_types + j].clone();
            }
        }

        self.matrix = new_matrix;

        // Resize type info
        while self.type_info.len() < new_num_types {
            self.type_info.push(ParticleTypeInfo::default_for_index(self.type_info.len()));
        }
        self.type_info.truncate(new_num_types);

        self.num_types = new_num_types;
    }

    /// Get the cell for interaction from type `from` to type `to`.
    pub fn get(&self, from: usize, to: usize) -> &RuleMatrixCell {
        &self.matrix[from * self.num_types + to]
    }

    /// Get mutable reference to the cell for interaction from type `from` to type `to`.
    pub fn get_mut(&mut self, from: usize, to: usize) -> &mut RuleMatrixCell {
        let idx = from * self.num_types + to;
        &mut self.matrix[idx]
    }

    /// Set the rules for interaction from type `from` to type `to`.
    pub fn set(&mut self, from: usize, to: usize, cell: RuleMatrixCell) {
        let idx = from * self.num_types + to;
        self.matrix[idx] = cell;
    }

    /// Set symmetric rules (both directions get the same rules).
    pub fn set_symmetric(&mut self, type_a: usize, type_b: usize, cell: RuleMatrixCell) {
        self.set(type_a, type_b, cell.clone());
        self.set(type_b, type_a, cell);
    }

    /// Check if any cell has rules (to determine if neighbor iteration is needed).
    pub fn has_any_rules(&self) -> bool {
        self.matrix.iter().any(|cell| !cell.is_empty())
    }

    /// Get the maximum radius used by any rule in the matrix.
    pub fn max_radius(&self) -> f32 {
        let mut max = 0.0f32;
        for cell in &self.matrix {
            for rule in &cell.rules {
                if let Some(radius) = rule.radius() {
                    max = max.max(radius);
                }
            }
        }
        max
    }

    /// Clear all rules and custom code from all cells.
    pub fn clear(&mut self) {
        for cell in &mut self.matrix {
            cell.rules.clear();
            cell.custom_code.clear();
        }
    }

    /// Check if this config requires neighbor iteration.
    pub fn needs_neighbors(&self) -> bool {
        self.enabled && self.has_any_rules()
    }
}

/// Preset rule matrix configurations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleMatrixPreset {
    /// Simple attraction between all types.
    AllAttract,
    /// Simple repulsion between all types.
    AllRepel,
    /// Circular chase: 0->1->2->3->0.
    Chase,
    /// Each type clusters with itself, repels others.
    Clusters,
    /// Predator-prey ecosystem (3 types: plants, herbivores, predators).
    Ecosystem,
    /// Full flocking behavior for same types.
    Flocking,
}

impl RuleMatrixPreset {
    /// Apply this preset to a rule matrix config.
    pub fn apply(&self, config: &mut InteractionConfig) {
        config.clear();
        let n = config.num_types;

        match self {
            Self::AllAttract => {
                for i in 0..n {
                    for j in 0..n {
                        config.set(i, j, RuleMatrixCell::with_rule(
                            RuleConfig::Attract { radius: 0.2, strength: 0.5 }
                        ));
                    }
                }
            }
            Self::AllRepel => {
                for i in 0..n {
                    for j in 0..n {
                        config.set(i, j, RuleMatrixCell::with_rule(
                            RuleConfig::Separate { radius: 0.15, strength: 2.0 }
                        ));
                    }
                }
            }
            Self::Chase => {
                for i in 0..n {
                    // Chase the next type
                    let target = (i + 1) % n;
                    config.set(i, target, RuleMatrixCell::with_rule(
                        RuleConfig::Attract { radius: 0.4, strength: 1.0 }
                    ));
                    // Flee from the type chasing us
                    let chaser = if i == 0 { n - 1 } else { i - 1 };
                    config.set(i, chaser, RuleMatrixCell::with_rule(
                        RuleConfig::Separate { radius: 0.3, strength: 3.0 }
                    ));
                }
            }
            Self::Clusters => {
                for i in 0..n {
                    for j in 0..n {
                        if i == j {
                            // Same type: attract and cohere
                            config.set(i, j, RuleMatrixCell::with_rules(vec![
                                RuleConfig::Cohere { radius: 0.25, strength: 0.5 },
                                RuleConfig::Separate { radius: 0.05, strength: 3.0 },
                            ]));
                        } else {
                            // Different types: repel
                            config.set(i, j, RuleMatrixCell::with_rule(
                                RuleConfig::Separate { radius: 0.15, strength: 2.0 }
                            ));
                        }
                    }
                }
            }
            Self::Ecosystem => {
                // Type 0: Plants (passive)
                // Type 1: Herbivores (eat plants, flock, flee predators)
                // Type 2: Predators (hunt herbivores, spread out)
                if n >= 3 {
                    // Herbivores attracted to plants
                    config.set(1, 0, RuleMatrixCell::with_rule(
                        RuleConfig::Attract { radius: 0.3, strength: 0.5 }
                    ));
                    // Herbivores flock together
                    config.set(1, 1, RuleMatrixCell::with_rules(vec![
                        RuleConfig::Cohere { radius: 0.2, strength: 0.3 },
                        RuleConfig::Align { radius: 0.15, strength: 0.5 },
                        RuleConfig::Separate { radius: 0.05, strength: 2.0 },
                    ]));
                    // Herbivores flee predators
                    config.set(1, 2, RuleMatrixCell::with_rule(
                        RuleConfig::Separate { radius: 0.4, strength: 5.0 }
                    ));
                    // Predators hunt herbivores
                    config.set(2, 1, RuleMatrixCell::with_rule(
                        RuleConfig::Attract { radius: 0.5, strength: 0.8 }
                    ));
                    // Predators spread out
                    config.set(2, 2, RuleMatrixCell::with_rule(
                        RuleConfig::Separate { radius: 0.2, strength: 1.5 }
                    ));
                }
            }
            Self::Flocking => {
                // Same types flock together with full boids behavior
                for i in 0..n {
                    config.set(i, i, RuleMatrixCell::with_rules(vec![
                        RuleConfig::Cohere { radius: 0.2, strength: 0.5 },
                        RuleConfig::Align { radius: 0.15, strength: 1.0 },
                        RuleConfig::Separate { radius: 0.05, strength: 3.0 },
                    ]));
                    // Slight repulsion from other types
                    for j in 0..n {
                        if i != j {
                            config.set(i, j, RuleMatrixCell::with_rule(
                                RuleConfig::Separate { radius: 0.1, strength: 1.0 }
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Get the display name for this preset.
    pub fn name(&self) -> &'static str {
        match self {
            Self::AllAttract => "All Attract",
            Self::AllRepel => "All Repel",
            Self::Chase => "Chase",
            Self::Clusters => "Clusters",
            Self::Ecosystem => "Ecosystem",
            Self::Flocking => "Flocking",
        }
    }

    /// Get all available presets.
    pub fn all() -> &'static [Self] {
        &[
            Self::AllAttract,
            Self::AllRepel,
            Self::Chase,
            Self::Clusters,
            Self::Ecosystem,
            Self::Flocking,
        ]
    }
}
