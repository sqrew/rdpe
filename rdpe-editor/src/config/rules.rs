//! Rule configuration types for particle behaviors

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::UniformValueConfig;

/// Falloff function for distance-based effects
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum Falloff {
    Constant,
    Linear,
    Inverse,
    #[default]
    InverseSquare,
    Smooth,
}

impl Falloff {
    pub fn variants() -> &'static [&'static str] {
        &["Constant", "Linear", "Inverse", "InverseSquare", "Smooth"]
    }
}

/// A transition in an agent state machine.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransitionConfig {
    /// Target state ID.
    pub to: u32,
    /// WGSL condition that triggers this transition.
    pub condition: String,
    /// Priority (higher = checked first).
    pub priority: i32,
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            to: 0,
            condition: "false".into(),
            priority: 0,
        }
    }
}

/// A state in an agent state machine.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AgentStateConfig {
    /// Unique state identifier.
    pub id: u32,
    /// Optional name for documentation.
    pub name: Option<String>,
    /// WGSL code to run when entering this state.
    pub on_enter: Option<String>,
    /// WGSL code to run every frame in this state.
    pub on_update: Option<String>,
    /// WGSL code to run when exiting this state.
    pub on_exit: Option<String>,
    /// Transitions to other states.
    pub transitions: Vec<TransitionConfig>,
}

impl AgentStateConfig {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: None,
            on_enter: None,
            on_update: None,
            on_exit: None,
            transitions: Vec::new(),
        }
    }
}

/// Serializable rule configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RuleConfig {
    // === Basic Forces ===
    Gravity(f32),
    Drag(f32),
    Acceleration { direction: [f32; 3] },

    // === Boundaries ===
    BounceWalls,
    WrapWalls,

    // === Point Forces ===
    AttractTo { point: [f32; 3], strength: f32 },
    RepelFrom { point: [f32; 3], strength: f32, radius: f32 },
    PointGravity { point: [f32; 3], strength: f32, softening: f32 },
    Orbit { center: [f32; 3], strength: f32 },
    Spring { anchor: [f32; 3], stiffness: f32, damping: f32 },
    Radial { point: [f32; 3], strength: f32, radius: f32, falloff: Falloff },
    Vortex { center: [f32; 3], axis: [f32; 3], strength: f32 },
    Pulse { point: [f32; 3], strength: f32, frequency: f32, radius: f32 },

    // === Noise & Flow ===
    Turbulence { scale: f32, strength: f32 },
    Curl { scale: f32, strength: f32 },
    Wind { direction: [f32; 3], strength: f32, turbulence: f32 },
    PositionNoise { scale: f32, strength: f32, speed: f32 },

    // === Steering ===
    Seek { target: [f32; 3], max_speed: f32, max_force: f32 },
    Flee { target: [f32; 3], max_speed: f32, max_force: f32, panic_radius: f32 },
    Arrive { target: [f32; 3], max_speed: f32, max_force: f32, slowing_radius: f32 },
    Wander { strength: f32, frequency: f32 },

    // === Boids / Flocking ===
    Separate { radius: f32, strength: f32 },
    Cohere { radius: f32, strength: f32 },
    Align { radius: f32, strength: f32 },
    Flock { radius: f32, separation: f32, cohesion: f32, alignment: f32 },
    Avoid { radius: f32, strength: f32 },

    // === Physics ===
    Collide { radius: f32, restitution: f32 },
    NBodyGravity { strength: f32, softening: f32, radius: f32 },
    LennardJones { epsilon: f32, sigma: f32, cutoff: f32 },
    Viscosity { radius: f32, strength: f32 },
    Pressure { radius: f32, strength: f32, target_density: f32 },
    SurfaceTension { radius: f32, strength: f32, threshold: f32 },
    Magnetism { radius: f32, strength: f32, same_repel: bool },

    // === Constraints ===
    SpeedLimit { min: f32, max: f32 },
    Buoyancy { surface_y: f32, density: f32 },
    Friction { ground_y: f32, strength: f32, threshold: f32 },

    // === Lifecycle ===
    Age,
    Lifetime(f32),
    FadeOut(f32),
    ShrinkOut(f32),
    ColorOverLife { start: [f32; 3], end: [f32; 3], duration: f32 },
    ColorBySpeed { slow_color: [f32; 3], fast_color: [f32; 3], max_speed: f32 },
    ColorByAge { young_color: [f32; 3], old_color: [f32; 3], max_age: f32 },
    ScaleBySpeed { min_scale: f32, max_scale: f32, max_speed: f32 },

    // === Typed Interactions ===
    Chase { self_type: u32, target_type: u32, radius: f32, strength: f32 },
    Evade { self_type: u32, threat_type: u32, radius: f32, strength: f32 },
    Convert { from_type: u32, trigger_type: u32, to_type: u32, radius: f32, probability: f32 },

    // === Events ===
    Shockwave { origin: [f32; 3], speed: f32, width: f32, strength: f32, repeat: f32 },
    Oscillate { axis: [f32; 3], amplitude: f32, frequency: f32, spatial_scale: f32 },
    RespawnBelow { threshold_y: f32, spawn_y: f32, reset_velocity: bool },

    // === Conditional ===
    Maybe { probability: f32, action: String },
    Trigger { condition: String, action: String },

    // === Custom WGSL ===
    Custom { code: String },
    NeighborCustom { code: String },
    OnCollision { radius: f32, response: String },
    CustomDynamic { code: String, params: Vec<(String, f32)> },
    NeighborCustomDynamic { code: String, params: Vec<(String, f32)> },

    // === Event Hooks ===
    OnCondition { condition: String, action: String },
    OnDeath { action: String },
    OnInterval { interval: f32, action: String },
    OnSpawn { action: String },

    // === Growth & Decay ===
    Grow { rate: f32, min: f32, max: f32 },
    Decay { field: String, rate: f32 },
    Die { condition: String },
    DLA { seed_type: u32, mobile_type: u32, stick_radius: f32, diffusion_strength: f32 },
    Refractory { trigger: String, charge: String, active_threshold: f32, depletion_rate: f32, regen_rate: f32 },

    // === Springs ===
    ChainSprings { stiffness: f32, damping: f32, rest_length: f32, max_stretch: Option<f32> },
    RadialSprings { hub_stiffness: f32, ring_stiffness: f32, damping: f32, hub_length: f32, ring_length: f32 },
    BondSprings { bonds: Vec<String>, stiffness: f32, damping: f32, rest_length: f32, max_stretch: Option<f32> },

    // === State Machine ===
    State { field: String, transitions: Vec<(u32, u32, String)> },
    Agent {
        state_field: String,
        prev_state_field: String,
        state_timer_field: Option<String>,
        states: Vec<AgentStateConfig>,
    },

    // === Conditional ===
    Switch { condition: String, then_code: String, else_code: Option<String> },
    TypedNeighbor { self_type: Option<u32>, other_type: Option<u32>, radius: f32, code: String },

    // === Advanced Physics ===
    DensityBuoyancy { density_field: String, medium_density: f32, strength: f32 },
    Diffuse { field: String, rate: f32, radius: f32 },
    Mass { field: String },

    // === Field Operations ===
    CopyField { from: String, to: String },
    Current { field: String, strength: f32 },

    // === Math / Signal ===
    Lerp { field: String, target: f32, rate: f32 },
    Clamp { field: String, min: f32, max: f32 },
    Remap { field: String, in_min: f32, in_max: f32, out_min: f32, out_max: f32 },
    Quantize { field: String, step: f32 },
    Noise { field: String, amplitude: f32, frequency: f32 },
    Smooth { field: String, target: f32, rate: f32 },
    Modulo { field: String, min: f32, max: f32 },
    Copy { from: String, to: String, scale: f32, offset: f32 },
    Threshold { input_field: String, output_field: String, threshold: f32, above: f32, below: f32 },
    Gate { condition: String, action: String },
    Tween { field: String, from: f32, to: f32, duration: f32, timer_field: String },
    Periodic { interval: f32, phase_field: Option<String>, action: String },

    // === Field Interactions ===
    Deposit { field_index: u32, source: String, amount: f32 },
    Sense { field_index: u32, target: String },
    Consume { field_index: u32, target: String, rate: f32 },
    Gradient { field: u32, strength: f32, ascending: bool },

    // === Neighbor Field Operations ===
    Accumulate { source: String, target: String, radius: f32, operation: String, falloff: Option<Falloff> },
    Signal { source: String, target: String, radius: f32, strength: f32, falloff: Option<Falloff> },
    Absorb { target_type: Option<u32>, radius: f32, source_field: String, target_field: String },

    // === Logic Gates ===
    And { a: String, b: String, output: String },
    Or { a: String, b: String, output: String },
    Not { input: String, output: String, max: f32 },
    Xor { a: String, b: String, output: String },
    Hysteresis { input: String, output: String, low_threshold: f32, high_threshold: f32, on_value: f32, off_value: f32 },
    Latch { output: String, set_condition: String, reset_condition: String, set_value: f32, reset_value: f32 },
    Edge { input: String, prev_field: String, output: String, threshold: f32, rising: bool, falling: bool },
    Select { condition: String, then_field: String, else_field: String, output: String },
    Blend { a: String, b: String, weight: String, output: String },

    // === Synchronization & Reproduction ===
    Sync {
        phase_field: String,
        frequency: f32,
        field: u32,
        emit_amount: f32,
        coupling: f32,
        detection_threshold: f32,
        on_fire: Option<String>,
    },
    Split {
        condition: String,
        offspring_count: u32,
        offspring_type: Option<u32>,
        resource_field: Option<String>,
        resource_cost: f32,
        spread: f32,
        speed_min: f32,
        speed_max: f32,
    },

    // === Dynamic Collision ===
    OnCollisionDynamic {
        radius: f32,
        response: String,
        params: Vec<(String, UniformValueConfig)>,
    },
}

impl RuleConfig {
    pub fn name(&self) -> &'static str {
        match self {
            // Basic Forces
            RuleConfig::Gravity(_) => "Gravity",
            RuleConfig::Drag(_) => "Drag",
            RuleConfig::Acceleration { .. } => "Acceleration",
            // Boundaries
            RuleConfig::BounceWalls => "Bounce Walls",
            RuleConfig::WrapWalls => "Wrap Walls",
            // Point Forces
            RuleConfig::AttractTo { .. } => "Attract To",
            RuleConfig::RepelFrom { .. } => "Repel From",
            RuleConfig::PointGravity { .. } => "Point Gravity",
            RuleConfig::Orbit { .. } => "Orbit",
            RuleConfig::Spring { .. } => "Spring",
            RuleConfig::Radial { .. } => "Radial",
            RuleConfig::Vortex { .. } => "Vortex",
            RuleConfig::Pulse { .. } => "Pulse",
            // Noise & Flow
            RuleConfig::Turbulence { .. } => "Turbulence",
            RuleConfig::Curl { .. } => "Curl",
            RuleConfig::Wind { .. } => "Wind",
            RuleConfig::PositionNoise { .. } => "Position Noise",
            // Steering
            RuleConfig::Seek { .. } => "Seek",
            RuleConfig::Flee { .. } => "Flee",
            RuleConfig::Arrive { .. } => "Arrive",
            RuleConfig::Wander { .. } => "Wander",
            // Boids
            RuleConfig::Separate { .. } => "Separate",
            RuleConfig::Cohere { .. } => "Cohere",
            RuleConfig::Align { .. } => "Align",
            RuleConfig::Flock { .. } => "Flock",
            RuleConfig::Avoid { .. } => "Avoid",
            // Physics
            RuleConfig::Collide { .. } => "Collide",
            RuleConfig::NBodyGravity { .. } => "N-Body Gravity",
            RuleConfig::LennardJones { .. } => "Lennard-Jones",
            RuleConfig::Viscosity { .. } => "Viscosity",
            RuleConfig::Pressure { .. } => "Pressure",
            RuleConfig::SurfaceTension { .. } => "Surface Tension",
            RuleConfig::Magnetism { .. } => "Magnetism",
            // Constraints
            RuleConfig::SpeedLimit { .. } => "Speed Limit",
            RuleConfig::Buoyancy { .. } => "Buoyancy",
            RuleConfig::Friction { .. } => "Friction",
            // Lifecycle
            RuleConfig::Age => "Age",
            RuleConfig::Lifetime(_) => "Lifetime",
            RuleConfig::FadeOut(_) => "Fade Out",
            RuleConfig::ShrinkOut(_) => "Shrink Out",
            RuleConfig::ColorOverLife { .. } => "Color Over Life",
            RuleConfig::ColorBySpeed { .. } => "Color By Speed",
            RuleConfig::ColorByAge { .. } => "Color By Age",
            RuleConfig::ScaleBySpeed { .. } => "Scale By Speed",
            // Typed
            RuleConfig::Chase { .. } => "Chase",
            RuleConfig::Evade { .. } => "Evade",
            RuleConfig::Convert { .. } => "Convert",
            // Events
            RuleConfig::Shockwave { .. } => "Shockwave",
            RuleConfig::Oscillate { .. } => "Oscillate",
            RuleConfig::RespawnBelow { .. } => "Respawn Below",
            // Conditional
            RuleConfig::Maybe { .. } => "Maybe",
            RuleConfig::Trigger { .. } => "Trigger",
            // Custom
            RuleConfig::Custom { .. } => "Custom WGSL",
            RuleConfig::NeighborCustom { .. } => "Neighbor Custom",
            RuleConfig::OnCollision { .. } => "On Collision",
            RuleConfig::CustomDynamic { .. } => "Custom Dynamic",
            RuleConfig::NeighborCustomDynamic { .. } => "Neighbor Custom Dynamic",
            // Event Hooks
            RuleConfig::OnCondition { .. } => "On Condition",
            RuleConfig::OnDeath { .. } => "On Death",
            RuleConfig::OnInterval { .. } => "On Interval",
            RuleConfig::OnSpawn { .. } => "On Spawn",
            // Growth & Decay
            RuleConfig::Grow { .. } => "Grow",
            RuleConfig::Decay { .. } => "Decay",
            RuleConfig::Die { .. } => "Die",
            RuleConfig::DLA { .. } => "DLA",
            RuleConfig::Refractory { .. } => "Refractory",
            // Springs
            RuleConfig::ChainSprings { .. } => "Chain Springs",
            RuleConfig::RadialSprings { .. } => "Radial Springs",
            RuleConfig::BondSprings { .. } => "Bond Springs",
            // State Machine
            RuleConfig::State { .. } => "State",
            RuleConfig::Agent { .. } => "Agent",
            // Conditional
            RuleConfig::Switch { .. } => "Switch",
            RuleConfig::TypedNeighbor { .. } => "Typed Neighbor",
            // Advanced Physics
            RuleConfig::DensityBuoyancy { .. } => "Density Buoyancy",
            RuleConfig::Diffuse { .. } => "Diffuse",
            RuleConfig::Mass { .. } => "Mass",
            // Field Operations
            RuleConfig::CopyField { .. } => "Copy Field",
            RuleConfig::Current { .. } => "Current",
            // Math / Signal
            RuleConfig::Lerp { .. } => "Lerp",
            RuleConfig::Clamp { .. } => "Clamp",
            RuleConfig::Remap { .. } => "Remap",
            RuleConfig::Quantize { .. } => "Quantize",
            RuleConfig::Noise { .. } => "Noise",
            RuleConfig::Smooth { .. } => "Smooth",
            RuleConfig::Modulo { .. } => "Modulo",
            RuleConfig::Copy { .. } => "Copy",
            RuleConfig::Threshold { .. } => "Threshold",
            RuleConfig::Gate { .. } => "Gate",
            RuleConfig::Tween { .. } => "Tween",
            RuleConfig::Periodic { .. } => "Periodic",
            // Field Interactions
            RuleConfig::Deposit { .. } => "Deposit",
            RuleConfig::Sense { .. } => "Sense",
            RuleConfig::Consume { .. } => "Consume",
            RuleConfig::Gradient { .. } => "Gradient",
            // Neighbor Field Operations
            RuleConfig::Accumulate { .. } => "Accumulate",
            RuleConfig::Signal { .. } => "Signal",
            RuleConfig::Absorb { .. } => "Absorb",
            // Logic Gates
            RuleConfig::And { .. } => "And",
            RuleConfig::Or { .. } => "Or",
            RuleConfig::Not { .. } => "Not",
            RuleConfig::Xor { .. } => "Xor",
            RuleConfig::Hysteresis { .. } => "Hysteresis",
            RuleConfig::Latch { .. } => "Latch",
            RuleConfig::Edge { .. } => "Edge",
            RuleConfig::Select { .. } => "Select",
            RuleConfig::Blend { .. } => "Blend",
            // Synchronization & Reproduction
            RuleConfig::Sync { .. } => "Sync",
            RuleConfig::Split { .. } => "Split",
            // Dynamic Collision
            RuleConfig::OnCollisionDynamic { .. } => "On Collision Dynamic",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            RuleConfig::Gravity(_) | RuleConfig::Drag(_) | RuleConfig::Acceleration { .. } => "Forces",
            RuleConfig::BounceWalls | RuleConfig::WrapWalls => "Boundaries",
            RuleConfig::AttractTo { .. } | RuleConfig::RepelFrom { .. } | RuleConfig::PointGravity { .. } |
            RuleConfig::Orbit { .. } | RuleConfig::Spring { .. } | RuleConfig::Radial { .. } |
            RuleConfig::Vortex { .. } | RuleConfig::Pulse { .. } => "Point Forces",
            RuleConfig::Turbulence { .. } | RuleConfig::Curl { .. } | RuleConfig::Wind { .. } |
            RuleConfig::PositionNoise { .. } => "Noise & Flow",
            RuleConfig::Seek { .. } | RuleConfig::Flee { .. } | RuleConfig::Arrive { .. } |
            RuleConfig::Wander { .. } => "Steering",
            RuleConfig::Separate { .. } | RuleConfig::Cohere { .. } | RuleConfig::Align { .. } |
            RuleConfig::Flock { .. } | RuleConfig::Avoid { .. } => "Flocking",
            RuleConfig::Collide { .. } | RuleConfig::NBodyGravity { .. } | RuleConfig::LennardJones { .. } |
            RuleConfig::Viscosity { .. } | RuleConfig::Pressure { .. } | RuleConfig::SurfaceTension { .. } |
            RuleConfig::Magnetism { .. } => "Physics",
            RuleConfig::SpeedLimit { .. } | RuleConfig::Buoyancy { .. } | RuleConfig::Friction { .. } => "Constraints",
            RuleConfig::Age | RuleConfig::Lifetime(_) | RuleConfig::FadeOut(_) | RuleConfig::ShrinkOut(_) |
            RuleConfig::ColorOverLife { .. } | RuleConfig::ColorBySpeed { .. } | RuleConfig::ColorByAge { .. } |
            RuleConfig::ScaleBySpeed { .. } => "Lifecycle",
            RuleConfig::Chase { .. } | RuleConfig::Evade { .. } | RuleConfig::Convert { .. } => "Typed",
            RuleConfig::Shockwave { .. } | RuleConfig::Oscillate { .. } | RuleConfig::RespawnBelow { .. } => "Events",
            RuleConfig::Maybe { .. } | RuleConfig::Trigger { .. } => "Conditional",
            RuleConfig::Custom { .. } | RuleConfig::NeighborCustom { .. } | RuleConfig::OnCollision { .. } |
            RuleConfig::CustomDynamic { .. } => "Custom",
            RuleConfig::NeighborCustomDynamic { .. } => "Custom",
            // New categories
            RuleConfig::OnCondition { .. } | RuleConfig::OnDeath { .. } | RuleConfig::OnInterval { .. } |
            RuleConfig::OnSpawn { .. } => "Event Hooks",
            RuleConfig::Grow { .. } | RuleConfig::Decay { .. } | RuleConfig::Die { .. } |
            RuleConfig::DLA { .. } | RuleConfig::Refractory { .. } => "Growth & Decay",
            RuleConfig::ChainSprings { .. } | RuleConfig::RadialSprings { .. } | RuleConfig::BondSprings { .. } => "Springs",
            RuleConfig::State { .. } | RuleConfig::Agent { .. } => "State Machine",
            RuleConfig::Switch { .. } => "Conditional",
            RuleConfig::TypedNeighbor { .. } => "Typed",
            RuleConfig::DensityBuoyancy { .. } | RuleConfig::Diffuse { .. } | RuleConfig::Mass { .. } => "Physics",
            RuleConfig::CopyField { .. } | RuleConfig::Current { .. } |
            RuleConfig::Deposit { .. } | RuleConfig::Sense { .. } | RuleConfig::Consume { .. } |
            RuleConfig::Gradient { .. } => "Fields",
            RuleConfig::Lerp { .. } | RuleConfig::Clamp { .. } | RuleConfig::Remap { .. } |
            RuleConfig::Quantize { .. } | RuleConfig::Noise { .. } | RuleConfig::Smooth { .. } |
            RuleConfig::Modulo { .. } | RuleConfig::Copy { .. } | RuleConfig::Threshold { .. } |
            RuleConfig::Gate { .. } | RuleConfig::Tween { .. } | RuleConfig::Periodic { .. } => "Math",
            RuleConfig::Accumulate { .. } | RuleConfig::Signal { .. } | RuleConfig::Absorb { .. } => "Neighbor Fields",
            RuleConfig::And { .. } | RuleConfig::Or { .. } | RuleConfig::Not { .. } |
            RuleConfig::Xor { .. } | RuleConfig::Hysteresis { .. } | RuleConfig::Latch { .. } |
            RuleConfig::Edge { .. } | RuleConfig::Select { .. } | RuleConfig::Blend { .. } => "Logic",
            RuleConfig::Sync { .. } | RuleConfig::Split { .. } => "Lifecycle",
            RuleConfig::OnCollisionDynamic { .. } => "Custom",
        }
    }

    /// Convert to rdpe::Rule
    pub fn to_rule(&self) -> rdpe::Rule {
        use rdpe::Rule;
        match self {
            RuleConfig::Gravity(g) => Rule::Gravity(*g),
            RuleConfig::Drag(d) => Rule::Drag(*d),
            RuleConfig::Acceleration { direction } => Rule::Acceleration(Vec3::from_array(*direction)),
            RuleConfig::BounceWalls => Rule::BounceWalls,
            RuleConfig::WrapWalls => Rule::WrapWalls,
            RuleConfig::AttractTo { point, strength } => Rule::AttractTo {
                point: Vec3::from_array(*point),
                strength: *strength,
            },
            RuleConfig::RepelFrom { point, strength, radius } => Rule::RepelFrom {
                point: Vec3::from_array(*point),
                strength: *strength,
                radius: *radius,
            },
            RuleConfig::PointGravity { point, strength, softening } => Rule::PointGravity {
                point: Vec3::from_array(*point),
                strength: *strength,
                softening: *softening,
            },
            RuleConfig::Orbit { center, strength } => Rule::Orbit {
                center: Vec3::from_array(*center),
                strength: *strength,
            },
            RuleConfig::Spring { anchor, stiffness, damping } => Rule::Spring {
                anchor: Vec3::from_array(*anchor),
                stiffness: *stiffness,
                damping: *damping,
            },
            RuleConfig::Radial { point, strength, radius, falloff } => Rule::Radial {
                point: Vec3::from_array(*point),
                strength: *strength,
                radius: *radius,
                falloff: match falloff {
                    Falloff::Constant => rdpe::Falloff::Constant,
                    Falloff::Linear => rdpe::Falloff::Linear,
                    Falloff::Inverse => rdpe::Falloff::Inverse,
                    Falloff::InverseSquare => rdpe::Falloff::InverseSquare,
                    Falloff::Smooth => rdpe::Falloff::Smooth,
                },
            },
            RuleConfig::Vortex { center, axis, strength } => Rule::Vortex {
                center: Vec3::from_array(*center),
                axis: Vec3::from_array(*axis),
                strength: *strength,
            },
            RuleConfig::Pulse { point, strength, frequency, radius } => Rule::Pulse {
                point: Vec3::from_array(*point),
                strength: *strength,
                frequency: *frequency,
                radius: *radius,
            },
            RuleConfig::Turbulence { scale, strength } => Rule::Turbulence {
                scale: *scale,
                strength: *strength,
            },
            RuleConfig::Curl { scale, strength } => Rule::Curl {
                scale: *scale,
                strength: *strength,
            },
            RuleConfig::Wind { direction, strength, turbulence } => Rule::Wind {
                direction: Vec3::from_array(*direction),
                strength: *strength,
                turbulence: *turbulence,
            },
            RuleConfig::PositionNoise { scale, strength, speed } => Rule::PositionNoise {
                scale: *scale,
                strength: *strength,
                speed: *speed,
            },
            RuleConfig::Seek { target, max_speed, max_force } => Rule::Seek {
                target: Vec3::from_array(*target),
                max_speed: *max_speed,
                max_force: *max_force,
            },
            RuleConfig::Flee { target, max_speed, max_force, panic_radius } => Rule::Flee {
                target: Vec3::from_array(*target),
                max_speed: *max_speed,
                max_force: *max_force,
                panic_radius: *panic_radius,
            },
            RuleConfig::Arrive { target, max_speed, max_force, slowing_radius } => Rule::Arrive {
                target: Vec3::from_array(*target),
                max_speed: *max_speed,
                max_force: *max_force,
                slowing_radius: *slowing_radius,
            },
            RuleConfig::Wander { strength, frequency } => Rule::Wander {
                strength: *strength,
                frequency: *frequency,
            },
            RuleConfig::Separate { radius, strength } => Rule::Separate {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Cohere { radius, strength } => Rule::Cohere {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Align { radius, strength } => Rule::Align {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Flock { radius, separation, cohesion, alignment } => Rule::Flock {
                radius: *radius,
                separation: *separation,
                cohesion: *cohesion,
                alignment: *alignment,
            },
            RuleConfig::Avoid { radius, strength } => Rule::Avoid {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Collide { radius, restitution } => Rule::Collide {
                radius: *radius,
                restitution: *restitution,
            },
            RuleConfig::NBodyGravity { strength, softening, radius } => Rule::NBodyGravity {
                strength: *strength,
                softening: *softening,
                radius: *radius,
            },
            RuleConfig::LennardJones { epsilon, sigma, cutoff } => Rule::LennardJones {
                epsilon: *epsilon,
                sigma: *sigma,
                cutoff: *cutoff,
            },
            RuleConfig::Viscosity { radius, strength } => Rule::Viscosity {
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Pressure { radius, strength, target_density } => Rule::Pressure {
                radius: *radius,
                strength: *strength,
                target_density: *target_density,
            },
            RuleConfig::SurfaceTension { radius, strength, threshold } => Rule::SurfaceTension {
                radius: *radius,
                strength: *strength,
                threshold: *threshold,
            },
            RuleConfig::Magnetism { radius, strength, same_repel } => Rule::Magnetism {
                radius: *radius,
                strength: *strength,
                same_repel: *same_repel,
            },
            RuleConfig::SpeedLimit { min, max } => Rule::SpeedLimit {
                min: *min,
                max: *max,
            },
            RuleConfig::Buoyancy { surface_y, density } => Rule::Buoyancy {
                surface_y: *surface_y,
                density: *density,
            },
            RuleConfig::Friction { ground_y, strength, threshold } => Rule::Friction {
                ground_y: *ground_y,
                strength: *strength,
                threshold: *threshold,
            },
            RuleConfig::Age => Rule::Age,
            RuleConfig::Lifetime(t) => Rule::Lifetime(*t),
            RuleConfig::FadeOut(t) => Rule::FadeOut(*t),
            RuleConfig::ShrinkOut(t) => Rule::ShrinkOut(*t),
            RuleConfig::ColorOverLife { start, end, duration } => Rule::ColorOverLife {
                start: Vec3::from_array(*start),
                end: Vec3::from_array(*end),
                duration: *duration,
            },
            RuleConfig::ColorBySpeed { slow_color, fast_color, max_speed } => Rule::ColorBySpeed {
                slow_color: Vec3::from_array(*slow_color),
                fast_color: Vec3::from_array(*fast_color),
                max_speed: *max_speed,
            },
            RuleConfig::ColorByAge { young_color, old_color, max_age } => Rule::ColorByAge {
                young_color: Vec3::from_array(*young_color),
                old_color: Vec3::from_array(*old_color),
                max_age: *max_age,
            },
            RuleConfig::ScaleBySpeed { min_scale, max_scale, max_speed } => Rule::ScaleBySpeed {
                min_scale: *min_scale,
                max_scale: *max_scale,
                max_speed: *max_speed,
            },
            RuleConfig::Chase { self_type, target_type, radius, strength } => Rule::Chase {
                self_type: *self_type,
                target_type: *target_type,
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Evade { self_type, threat_type, radius, strength } => Rule::Evade {
                self_type: *self_type,
                threat_type: *threat_type,
                radius: *radius,
                strength: *strength,
            },
            RuleConfig::Convert { from_type, trigger_type, to_type, radius, probability } => Rule::Convert {
                from_type: *from_type,
                trigger_type: *trigger_type,
                to_type: *to_type,
                radius: *radius,
                probability: *probability,
            },
            RuleConfig::Shockwave { origin, speed, width, strength, repeat } => Rule::Shockwave {
                origin: Vec3::from_array(*origin),
                speed: *speed,
                width: *width,
                strength: *strength,
                repeat: *repeat,
            },
            RuleConfig::Oscillate { axis, amplitude, frequency, spatial_scale } => Rule::Oscillate {
                axis: Vec3::from_array(*axis),
                amplitude: *amplitude,
                frequency: *frequency,
                spatial_scale: *spatial_scale,
            },
            RuleConfig::RespawnBelow { threshold_y, spawn_y, reset_velocity } => Rule::RespawnBelow {
                threshold_y: *threshold_y,
                spawn_y: *spawn_y,
                reset_velocity: *reset_velocity,
            },
            RuleConfig::Maybe { probability, action } => Rule::Maybe {
                probability: *probability,
                action: action.clone(),
            },
            RuleConfig::Trigger { condition, action } => Rule::Trigger {
                condition: condition.clone(),
                action: action.clone(),
            },
            RuleConfig::Custom { code } => Rule::Custom(code.clone()),
            RuleConfig::NeighborCustom { code } => Rule::NeighborCustom(code.clone()),
            RuleConfig::OnCollision { radius, response } => Rule::OnCollision {
                radius: *radius,
                response: response.clone(),
            },
            RuleConfig::CustomDynamic { code, params } => {
                let mut builder = Rule::custom_dynamic(code.clone());
                for (name, value) in params {
                    builder = builder.with_param(name, *value);
                }
                builder.into()
            }
            RuleConfig::NeighborCustomDynamic { code, params } => {
                let mut builder = Rule::neighbor_custom_dynamic(code.clone());
                for (name, value) in params {
                    builder = builder.with_param(name, *value);
                }
                builder.into()
            }
            // Event Hooks
            RuleConfig::OnCondition { condition, action } => Rule::OnCondition {
                condition: condition.clone(),
                action: action.clone(),
            },
            RuleConfig::OnDeath { action } => Rule::OnDeath {
                action: action.clone(),
            },
            RuleConfig::OnInterval { interval, action } => Rule::OnInterval {
                interval: *interval,
                action: action.clone(),
            },
            RuleConfig::OnSpawn { action } => Rule::OnSpawn {
                action: action.clone(),
            },
            // Growth & Decay
            RuleConfig::Grow { rate, min, max } => Rule::Grow {
                rate: *rate,
                min: *min,
                max: *max,
            },
            RuleConfig::Decay { field, rate } => Rule::Decay {
                field: field.clone(),
                rate: *rate,
            },
            RuleConfig::Die { condition } => Rule::Die {
                condition: condition.clone(),
                field: "alive".into(),
            },
            RuleConfig::DLA { seed_type, mobile_type, stick_radius, diffusion_strength } => Rule::DLA {
                seed_type: *seed_type,
                mobile_type: *mobile_type,
                stick_radius: *stick_radius,
                diffusion_strength: *diffusion_strength,
            },
            // Field Operations
            RuleConfig::CopyField { from, to } => Rule::CopyField {
                from: from.clone(),
                to: to.clone(),
            },
            RuleConfig::Current { field, strength } => Rule::Current {
                field: Box::leak(field.clone().into_boxed_str()),
                strength: *strength,
            },
            // Math / Signal
            RuleConfig::Lerp { field, target, rate } => Rule::Lerp {
                field: field.clone(),
                target: *target,
                rate: *rate,
            },
            RuleConfig::Clamp { field, min, max } => Rule::Clamp {
                field: field.clone(),
                min: *min,
                max: *max,
            },
            RuleConfig::Remap { field, in_min, in_max, out_min, out_max } => Rule::Remap {
                field: field.clone(),
                in_min: *in_min,
                in_max: *in_max,
                out_min: *out_min,
                out_max: *out_max,
            },
            RuleConfig::Quantize { field, step } => Rule::Quantize {
                field: field.clone(),
                step: *step,
            },
            RuleConfig::Noise { field, amplitude, frequency } => Rule::Noise {
                field: field.clone(),
                amplitude: *amplitude,
                frequency: *frequency,
                time_scale: 1.0,
            },
            // Springs
            RuleConfig::ChainSprings { stiffness, damping, rest_length, max_stretch } => Rule::ChainSprings {
                stiffness: *stiffness,
                damping: *damping,
                rest_length: *rest_length,
                max_stretch: *max_stretch,
            },
            RuleConfig::RadialSprings { hub_stiffness, ring_stiffness, damping, hub_length, ring_length } => Rule::RadialSprings {
                hub_stiffness: *hub_stiffness,
                ring_stiffness: *ring_stiffness,
                damping: *damping,
                hub_length: *hub_length,
                ring_length: *ring_length,
            },
            RuleConfig::BondSprings { bonds, stiffness, damping, rest_length, max_stretch } => Rule::BondSprings {
                bonds: bonds.iter().map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str).collect(),
                stiffness: *stiffness,
                damping: *damping,
                rest_length: *rest_length,
                max_stretch: *max_stretch,
            },
            // State Machine
            RuleConfig::State { field, transitions } => Rule::State {
                field: field.clone(),
                transitions: transitions.clone(),
            },
            RuleConfig::Agent { state_field, prev_state_field, state_timer_field, states } => Rule::Agent {
                state_field: state_field.clone(),
                prev_state_field: prev_state_field.clone(),
                state_timer_field: state_timer_field.clone(),
                states: states.iter().map(|s| {
                    let mut agent_state = rdpe::AgentState::new(s.id);
                    if let Some(name) = &s.name {
                        agent_state = agent_state.named(name.clone());
                    }
                    if let Some(code) = &s.on_enter {
                        agent_state = agent_state.on_enter(code.clone());
                    }
                    if let Some(code) = &s.on_update {
                        agent_state = agent_state.on_update(code.clone());
                    }
                    if let Some(code) = &s.on_exit {
                        agent_state = agent_state.on_exit(code.clone());
                    }
                    for t in &s.transitions {
                        agent_state = agent_state.transition_priority(t.to, t.condition.clone(), t.priority);
                    }
                    agent_state
                }).collect(),
            },
            // Conditional (simplified)
            RuleConfig::Switch { condition, then_code, else_code } => {
                let code = if let Some(else_c) = else_code {
                    format!("if ({}) {{\n    {}\n}} else {{\n    {}\n}}", condition, then_code, else_c)
                } else {
                    format!("if ({}) {{\n    {}\n}}", condition, then_code)
                };
                Rule::Custom(code)
            },
            RuleConfig::TypedNeighbor { self_type, other_type, radius, code } => {
                let type_check = match (self_type, other_type) {
                    (Some(st), Some(ot)) => format!("if p.particle_type != {}u || other.particle_type != {}u {{ continue; }}\n", st, ot),
                    (Some(st), None) => format!("if p.particle_type != {}u {{ continue; }}\n", st),
                    (None, Some(ot)) => format!("if other.particle_type != {}u {{ continue; }}\n", ot),
                    (None, None) => String::new(),
                };
                let full_code = format!(
                    "{}if neighbor_dist < {} && neighbor_dist > 0.001 {{\n    {}\n}}",
                    type_check, radius, code
                );
                Rule::NeighborCustom(full_code)
            },
            // Advanced Physics
            RuleConfig::DensityBuoyancy { density_field, medium_density, strength } => Rule::DensityBuoyancy {
                density_field: density_field.clone(),
                medium_density: *medium_density,
                strength: *strength,
            },
            RuleConfig::Diffuse { field, rate, radius } => Rule::Diffuse {
                field: field.clone(),
                rate: *rate,
                radius: *radius,
            },
            RuleConfig::Mass { field } => Rule::Mass {
                field: field.clone(),
            },
            RuleConfig::Refractory { trigger, charge, active_threshold, depletion_rate, regen_rate } => Rule::Refractory {
                trigger: trigger.clone(),
                charge: charge.clone(),
                active_threshold: *active_threshold,
                depletion_rate: *depletion_rate,
                regen_rate: *regen_rate,
            },
            // Math / Signal
            RuleConfig::Smooth { field, target, rate } => Rule::Smooth {
                field: field.clone(),
                target: *target,
                rate: *rate,
            },
            RuleConfig::Modulo { field, min, max } => Rule::Modulo {
                field: field.clone(),
                min: *min,
                max: *max,
            },
            RuleConfig::Copy { from, to, scale, offset } => Rule::Copy {
                from: from.clone(),
                to: to.clone(),
                scale: *scale,
                offset: *offset,
            },
            RuleConfig::Threshold { input_field, output_field, threshold, above, below } => Rule::Threshold {
                input_field: input_field.clone(),
                output_field: output_field.clone(),
                threshold: *threshold,
                above: *above,
                below: *below,
            },
            RuleConfig::Gate { condition, action } => Rule::Gate {
                condition: condition.clone(),
                action: action.clone(),
            },
            RuleConfig::Tween { field, from, to, duration, timer_field } => Rule::Tween {
                field: field.clone(),
                from: *from,
                to: *to,
                duration: *duration,
                timer_field: timer_field.clone(),
            },
            RuleConfig::Periodic { interval, phase_field, action } => Rule::Periodic {
                interval: *interval,
                phase_field: phase_field.clone(),
                action: action.clone(),
            },
            // Field Interactions
            RuleConfig::Deposit { field_index, source, amount } => Rule::Deposit {
                field_index: *field_index,
                source: source.clone(),
                amount: *amount,
            },
            RuleConfig::Sense { field_index, target } => Rule::Sense {
                field_index: *field_index,
                target: target.clone(),
            },
            RuleConfig::Consume { field_index, target, rate } => Rule::Consume {
                field_index: *field_index,
                target: target.clone(),
                rate: *rate,
            },
            RuleConfig::Gradient { field, strength, ascending } => Rule::Gradient {
                field: *field,
                strength: *strength,
                ascending: *ascending,
            },
            // Neighbor Field Operations
            RuleConfig::Accumulate { source, target, radius, operation, falloff } => Rule::Accumulate {
                source: source.clone(),
                target: target.clone(),
                radius: *radius,
                operation: operation.clone(),
                falloff: falloff.map(|f| match f {
                    Falloff::Constant => rdpe::Falloff::Constant,
                    Falloff::Linear => rdpe::Falloff::Linear,
                    Falloff::Inverse => rdpe::Falloff::Inverse,
                    Falloff::InverseSquare => rdpe::Falloff::InverseSquare,
                    Falloff::Smooth => rdpe::Falloff::Smooth,
                }),
            },
            RuleConfig::Signal { source, target, radius, strength, falloff } => Rule::Signal {
                source: source.clone(),
                target: target.clone(),
                radius: *radius,
                strength: *strength,
                falloff: falloff.map(|f| match f {
                    Falloff::Constant => rdpe::Falloff::Constant,
                    Falloff::Linear => rdpe::Falloff::Linear,
                    Falloff::Inverse => rdpe::Falloff::Inverse,
                    Falloff::InverseSquare => rdpe::Falloff::InverseSquare,
                    Falloff::Smooth => rdpe::Falloff::Smooth,
                }),
            },
            RuleConfig::Absorb { target_type, radius, source_field, target_field } => Rule::Absorb {
                target_type: *target_type,
                radius: *radius,
                source_field: source_field.clone(),
                target_field: target_field.clone(),
            },
            // Logic Gates
            RuleConfig::And { a, b, output } => Rule::And {
                a: a.clone(),
                b: b.clone(),
                output: output.clone(),
            },
            RuleConfig::Or { a, b, output } => Rule::Or {
                a: a.clone(),
                b: b.clone(),
                output: output.clone(),
            },
            RuleConfig::Not { input, output, max } => Rule::Not {
                input: input.clone(),
                output: output.clone(),
                max: *max,
            },
            RuleConfig::Xor { a, b, output } => Rule::Xor {
                a: a.clone(),
                b: b.clone(),
                output: output.clone(),
            },
            RuleConfig::Hysteresis { input, output, low_threshold, high_threshold, on_value, off_value } => Rule::Hysteresis {
                input: input.clone(),
                output: output.clone(),
                low_threshold: *low_threshold,
                high_threshold: *high_threshold,
                on_value: *on_value,
                off_value: *off_value,
            },
            RuleConfig::Latch { output, set_condition, reset_condition, set_value, reset_value } => Rule::Latch {
                output: output.clone(),
                set_condition: set_condition.clone(),
                reset_condition: reset_condition.clone(),
                set_value: *set_value,
                reset_value: *reset_value,
            },
            RuleConfig::Edge { input, prev_field, output, threshold, rising, falling } => Rule::Edge {
                input: input.clone(),
                prev_field: prev_field.clone(),
                output: output.clone(),
                threshold: *threshold,
                rising: *rising,
                falling: *falling,
            },
            RuleConfig::Select { condition, then_field, else_field, output } => Rule::Select {
                condition: condition.clone(),
                then_field: then_field.clone(),
                else_field: else_field.clone(),
                output: output.clone(),
            },
            RuleConfig::Blend { a, b, weight, output } => Rule::Blend {
                a: a.clone(),
                b: b.clone(),
                weight: weight.clone(),
                output: output.clone(),
            },
            RuleConfig::Sync { phase_field, frequency, field, emit_amount, coupling, detection_threshold, on_fire } => Rule::Sync {
                phase_field: phase_field.clone(),
                frequency: *frequency,
                field: *field,
                emit_amount: *emit_amount,
                coupling: *coupling,
                detection_threshold: *detection_threshold,
                on_fire: on_fire.clone(),
            },
            RuleConfig::Split { condition, offspring_count, offspring_type, resource_field, resource_cost, spread, speed_min, speed_max } => Rule::Split {
                condition: condition.clone(),
                offspring_count: *offspring_count,
                offspring_type: *offspring_type,
                resource_field: resource_field.clone(),
                resource_cost: *resource_cost,
                spread: *spread,
                speed_min: *speed_min,
                speed_max: *speed_max,
            },
            RuleConfig::OnCollisionDynamic { radius, response, params } => Rule::OnCollisionDynamic {
                radius: *radius,
                response: response.clone(),
                params: params.iter().map(|(k, v)| {
                    let uv = match v {
                        UniformValueConfig::F32(f) => rdpe::UniformValue::F32(*f),
                        UniformValueConfig::Vec2(arr) => rdpe::UniformValue::Vec2(glam::Vec2::from_array(*arr)),
                        UniformValueConfig::Vec3(arr) => rdpe::UniformValue::Vec3(glam::Vec3::from_array(*arr)),
                        UniformValueConfig::Vec4(arr) => rdpe::UniformValue::Vec4(glam::Vec4::from_array(*arr)),
                    };
                    (k.clone(), uv)
                }).collect(),
            },
        }
    }

    /// Check if this rule requires spatial hashing
    pub fn requires_neighbors(&self) -> bool {
        matches!(self,
            RuleConfig::Separate { .. } | RuleConfig::Cohere { .. } | RuleConfig::Align { .. } |
            RuleConfig::Flock { .. } | RuleConfig::Avoid { .. } | RuleConfig::Collide { .. } |
            RuleConfig::NBodyGravity { .. } | RuleConfig::LennardJones { .. } |
            RuleConfig::Viscosity { .. } | RuleConfig::Pressure { .. } |
            RuleConfig::SurfaceTension { .. } | RuleConfig::Magnetism { .. } |
            RuleConfig::Chase { .. } | RuleConfig::Evade { .. } | RuleConfig::Convert { .. } |
            RuleConfig::NeighborCustom { .. } | RuleConfig::OnCollision { .. } |
            RuleConfig::DLA { .. } | RuleConfig::Diffuse { .. } |
            RuleConfig::NeighborCustomDynamic { .. } |
            RuleConfig::Accumulate { .. } | RuleConfig::Signal { .. } | RuleConfig::Absorb { .. } |
            RuleConfig::OnCollisionDynamic { .. } | RuleConfig::TypedNeighbor { .. }
        )
    }

    /// Generate custom neighbor WGSL for rules that need editor-specific handling.
    ///
    /// Returns `Some(wgsl)` if this rule needs custom code generation in the editor,
    /// `None` to use the core library's default implementation.
    ///
    /// Note: `particle_type` is now a base field, always present.
    pub fn to_neighbor_wgsl(&self) -> Option<String> {
        match self {
            RuleConfig::Magnetism { radius, strength, same_repel } => {
                let same_sign = if *same_repel { "1.0" } else { "-1.0" };
                Some(format!(
                    r#"            // Magnetism
            if neighbor_dist < {radius} && neighbor_dist > 0.001 {{
                let same_type = select(-1.0, 1.0, p.particle_type == other.particle_type);
                let force_dir = same_type * {same_sign}; // +1 = repel, -1 = attract
                let falloff = 1.0 - neighbor_dist / {radius};
                p.velocity += neighbor_dir * force_dir * falloff * {strength} * uniforms.delta_time;
            }}"#
                ))
            }

            RuleConfig::Chase { self_type, target_type, radius, .. } => {
                Some(format!(
                    r#"            // Chase: track nearest target
            if p.particle_type == {self_type}u && other.particle_type == {target_type}u && neighbor_dist < {radius} {{
                if neighbor_dist < chase_nearest_dist {{
                    chase_nearest_dist = neighbor_dist;
                    chase_nearest_pos = neighbor_pos;
                }}
            }}"#
                ))
            }

            RuleConfig::Evade { self_type, threat_type, radius, .. } => {
                Some(format!(
                    r#"            // Evade: track nearest threat
            if p.particle_type == {self_type}u && other.particle_type == {threat_type}u && neighbor_dist < {radius} {{
                if neighbor_dist < evade_nearest_dist {{
                    evade_nearest_dist = neighbor_dist;
                    evade_nearest_pos = neighbor_pos;
                }}
            }}"#
                ))
            }

            RuleConfig::Convert { from_type, trigger_type, to_type, radius, probability } => {
                Some(format!(
                    r#"            // Convert type {from_type} -> {to_type} (triggered by {trigger_type})
            if p.particle_type == {from_type}u && other.particle_type == {trigger_type}u && neighbor_dist < {radius} {{
                let hash_input = index ^ (other_idx * 1103515245u) ^ u32(uniforms.time * 1000.0);
                let hash = (hash_input ^ (hash_input >> 16u)) * 0x45d9f3bu;
                let rand = f32(hash & 0xFFFFu) / 65535.0;
                if rand < {probability} {{
                    p.particle_type = {to_type}u;
                }}
            }}"#
                ))
            }

            RuleConfig::DLA { seed_type, mobile_type, stick_radius, diffusion_strength } => {
                Some(format!(
                    r#"            // Diffusion-Limited Aggregation
            if p.particle_type == {mobile_type}u && other.particle_type == {seed_type}u {{
                if neighbor_dist < {stick_radius} {{
                    p.particle_type = {seed_type}u;
                    p.velocity = vec3<f32>(0.0, 0.0, 0.0);
                }}
            }}
            if p.particle_type == {mobile_type}u {{
                let diff_seed = index * 1103515245u + u32(uniforms.time * 1000.0);
                let hx = (diff_seed ^ (diff_seed >> 15u)) * 0x45d9f3bu;
                let hy = ((diff_seed + 1u) ^ ((diff_seed + 1u) >> 15u)) * 0x45d9f3bu;
                let hz = ((diff_seed + 2u) ^ ((diff_seed + 2u) >> 15u)) * 0x45d9f3bu;
                let diff_force = vec3<f32>(
                    f32(hx & 0xFFFFu) / 32768.0 - 1.0,
                    f32(hy & 0xFFFFu) / 32768.0 - 1.0,
                    f32(hz & 0xFFFFu) / 32768.0 - 1.0
                );
                p.velocity += diff_force * {diffusion_strength} * uniforms.delta_time;
            }}"#
                ))
            }

            _ => None,
        }
    }

    /// Generate custom post-neighbor WGSL for rules that need editor-specific handling.
    ///
    /// Returns `Some(wgsl)` if this rule needs custom post-neighbor code,
    /// `None` to use the core library's default implementation.
    pub fn to_post_neighbor_wgsl(&self) -> Option<String> {
        match self {
            RuleConfig::Chase { self_type, strength, .. } => {
                Some(format!(
                    r#"    // Apply chase steering
    if p.particle_type == {self_type}u && chase_nearest_dist < 1000.0 {{
        let to_target = chase_nearest_pos - p.position;
        let dist = length(to_target);
        if dist > 0.001 {{
            p.velocity += normalize(to_target) * {strength} * uniforms.delta_time;
        }}
    }}"#
                ))
            }

            RuleConfig::Evade { self_type, strength, .. } => {
                Some(format!(
                    r#"    // Apply evade steering
    if p.particle_type == {self_type}u && evade_nearest_dist < 1000.0 {{
        let away_from_threat = p.position - evade_nearest_pos;
        let dist = length(away_from_threat);
        if dist > 0.001 {{
            p.velocity += normalize(away_from_threat) * {strength} * uniforms.delta_time;
        }}
    }}"#
                ))
            }

            _ => None,
        }
    }
}
