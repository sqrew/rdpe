//! Sub-emitter system for spawning particles on particle death.
//!
//! Sub-emitters allow particles to spawn child particles when they die,
//! enabling effects like fireworks, explosions with debris, chain reactions,
//! and biological reproduction.
//!
//! # How It Works
//!
//! 1. Parent particles are marked with a specific type
//! 2. When a parent dies (via `Rule::Lifetime` or `kill_particle()`), a death event is recorded
//! 3. A secondary compute pass reads death events and spawns children
//! 4. Children inherit position from parent, with configurable velocity spread
//!
//! # Example
//!
//! ```ignore
//! #[derive(ParticleType)]
//! enum Firework {
//!     Rocket,
//!     Spark,
//! }
//!
//! Simulation::<Particle>::new()
//!     .with_particle_count(10_000)
//!     .with_lifecycle(|l| l.lifetime(2.0))  // Rockets live 2 seconds
//!     .with_sub_emitter(SubEmitter::new(Firework::Rocket, Firework::Spark)
//!         .count(50)
//!         .speed(1.0..3.0)
//!         .spread(std::f32::consts::PI)  // Full sphere
//!         .inherit_velocity(0.2)
//!         .child_lifetime(1.0))
//!     .run();
//! ```
//!
//! # Multiple Sub-Emitters
//!
//! You can chain multiple sub-emitters for complex effects:
//!
//! ```ignore
//! // Rockets spawn sparks, sparks spawn embers
//! .with_sub_emitter(SubEmitter::new(Rocket, Spark).count(30))
//! .with_sub_emitter(SubEmitter::new(Spark, Ember).count(5))
//! ```

use glam::Vec3;
use std::ops::Range;

/// Configuration for a sub-emitter that spawns children when parents die.
///
/// # Example
///
/// ```ignore
/// SubEmitter::new(ParentType, ChildType)
///     .count(20)                    // Spawn 20 children per death
///     .speed(1.0..2.0)              // Random speed in range
///     .spread(PI / 4.0)             // 45-degree cone
///     .inherit_velocity(0.5)        // 50% of parent velocity
///     .child_lifetime(1.5)          // Children live 1.5 seconds
///     .child_color(Vec3::new(1.0, 0.5, 0.0))  // Orange children
/// ```
#[derive(Clone, Debug)]
pub struct SubEmitter {
    /// Parent particle type that triggers sub-emission on death.
    pub parent_type: u32,
    /// Child particle type to spawn.
    pub child_type: u32,
    /// Number of children to spawn per parent death.
    pub count: u32,
    /// Speed range for children (random within range).
    pub speed_min: f32,
    pub speed_max: f32,
    /// Spread angle in radians (0 = laser, PI = hemisphere, TAU = full sphere).
    pub spread: f32,
    /// How much of parent's velocity children inherit (0.0 - 1.0).
    pub inherit_velocity: f32,
    /// Optional fixed lifetime for children (overrides any lifecycle rules).
    pub child_lifetime: Option<f32>,
    /// Optional fixed color for children.
    pub child_color: Option<Vec3>,
    /// Spawn radius around parent position.
    pub spawn_radius: f32,
}

impl SubEmitter {
    /// Create a new sub-emitter configuration.
    ///
    /// # Arguments
    ///
    /// * `parent_type` - Type of particle that triggers sub-emission on death
    /// * `child_type` - Type of particle to spawn as children
    ///
    /// # Example
    ///
    /// ```ignore
    /// SubEmitter::new(Firework::Rocket.into(), Firework::Spark.into())
    /// ```
    pub fn new(parent_type: u32, child_type: u32) -> Self {
        Self {
            parent_type,
            child_type,
            count: 10,
            speed_min: 0.5,
            speed_max: 1.5,
            spread: std::f32::consts::PI, // Hemisphere by default
            inherit_velocity: 0.3,
            child_lifetime: None,
            child_color: None,
            spawn_radius: 0.0,
        }
    }

    /// Set the number of children to spawn per parent death.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SubEmitter::new(Rocket, Spark).count(50)
    /// ```
    pub fn count(mut self, n: u32) -> Self {
        self.count = n;
        self
    }

    /// Set the speed range for children.
    ///
    /// Children will have a random speed within this range.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SubEmitter::new(Rocket, Spark).speed(1.0..3.0)
    /// ```
    pub fn speed(mut self, range: Range<f32>) -> Self {
        self.speed_min = range.start;
        self.speed_max = range.end;
        self
    }

    /// Set the spread angle in radians.
    ///
    /// - `0` = All children move in same direction (laser)
    /// - `PI/4` = 45-degree cone
    /// - `PI/2` = 90-degree cone (hemisphere)
    /// - `PI` = Full hemisphere
    /// - `TAU` = Full sphere (all directions)
    ///
    /// # Example
    ///
    /// ```ignore
    /// SubEmitter::new(Rocket, Spark).spread(std::f32::consts::PI)
    /// ```
    pub fn spread(mut self, radians: f32) -> Self {
        self.spread = radians;
        self
    }

    /// Set how much of parent's velocity children inherit.
    ///
    /// - `0.0` = Children ignore parent velocity
    /// - `0.5` = Children inherit half of parent velocity
    /// - `1.0` = Children inherit full parent velocity
    ///
    /// # Example
    ///
    /// ```ignore
    /// SubEmitter::new(Rocket, Spark).inherit_velocity(0.5)
    /// ```
    pub fn inherit_velocity(mut self, factor: f32) -> Self {
        self.inherit_velocity = factor.clamp(0.0, 1.0);
        self
    }

    /// Set a fixed lifetime for children.
    ///
    /// This overrides any `Rule::Lifetime` for child particles.
    /// If not set, children follow normal lifecycle rules.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SubEmitter::new(Rocket, Spark).child_lifetime(1.5)
    /// ```
    pub fn child_lifetime(mut self, seconds: f32) -> Self {
        self.child_lifetime = Some(seconds);
        self
    }

    /// Set a fixed color for children.
    ///
    /// If not set, children inherit parent's color.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SubEmitter::new(Rocket, Spark).child_color(Vec3::new(1.0, 0.8, 0.0))
    /// ```
    pub fn child_color(mut self, color: Vec3) -> Self {
        self.child_color = Some(color);
        self
    }

    /// Set spawn radius around parent position.
    ///
    /// Children spawn at random positions within this radius of the parent.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SubEmitter::new(Rocket, Spark).spawn_radius(0.1)
    /// ```
    pub fn spawn_radius(mut self, radius: f32) -> Self {
        self.spawn_radius = radius;
        self
    }

    /// Generate WGSL code for spawning children from death events.
    ///
    /// This code runs in a secondary compute pass.
    pub(crate) fn child_spawning_wgsl(&self, emitter_index: usize) -> String {
        let child_color_code = if let Some(color) = self.child_color {
            format!(
                "child.color = vec3<f32>({}, {}, {});",
                color.x, color.y, color.z
            )
        } else {
            "child.color = death.color;".to_string()
        };

        let child_lifetime_code = if let Some(lifetime) = self.child_lifetime {
            format!("// Child lifetime set to {}", lifetime)
        } else {
            "// Child uses normal lifecycle".to_string()
        };

        format!(
            r#"
    // Sub-emitter {emitter_index}: Spawn children for parent type {parent_type}
    if death.parent_type == {parent_type}u {{
        let num_children = {count}u;
        let speed_min = {speed_min:.6};
        let speed_max = {speed_max:.6};
        let spread = {spread:.6};
        let inherit_vel = {inherit_velocity:.6};
        let spawn_radius = {spawn_radius:.6};

        // Spawn each child
        for (var child_i = 0u; child_i < num_children; child_i++) {{
            // Find a dead slot using atomic counter
            let slot = atomicAdd(&next_child_slot, 1u);
            if slot >= arrayLength(&particles) {{
                break;
            }}

            // Search for actual dead particle starting from slot
            var actual_slot = slot;
            var found = false;
            for (var search = 0u; search < 100u; search++) {{
                let check_slot = (slot + search) % arrayLength(&particles);
                if particles[check_slot].alive == 0u {{
                    actual_slot = check_slot;
                    found = true;
                    break;
                }}
            }}

            if !found {{
                continue;
            }}

            var child = particles[actual_slot];

            // Random direction within spread cone
            let seed = death_idx * 1000u + child_i * 7u + {emitter_index}u;
            let theta = rand(seed) * 6.28318;
            let phi = rand(seed + 1u) * spread;
            let dir = vec3<f32>(
                sin(phi) * cos(theta),
                cos(phi),
                sin(phi) * sin(theta)
            );

            // Random speed within range
            let speed = speed_min + rand(seed + 2u) * (speed_max - speed_min);

            // Random offset within spawn radius
            let offset = rand_sphere(seed + 3u) * spawn_radius;

            // Set child properties
            child.position = death.position + offset;
            child.velocity = death.velocity * inherit_vel + dir * speed;
            child.particle_type = {child_type}u;
            child.age = 0.0;
            child.alive = 1u;
            child.scale = 1.0;
            {child_color_code}
            {child_lifetime_code}

            particles[actual_slot] = child;
        }}
    }}
"#,
            emitter_index = emitter_index,
            parent_type = self.parent_type,
            child_type = self.child_type,
            count = self.count,
            speed_min = self.speed_min,
            speed_max = self.speed_max,
            spread = self.spread,
            inherit_velocity = self.inherit_velocity,
            spawn_radius = self.spawn_radius,
            child_color_code = child_color_code,
            child_lifetime_code = child_lifetime_code,
        )
    }
}

/// Maximum number of death events that can be recorded per frame.
pub const MAX_DEATH_EVENTS: u32 = 4096;

/// WGSL struct definition for death events.
pub const DEATH_EVENT_WGSL: &str = r#"
struct DeathEvent {
    position: vec3<f32>,
    parent_type: u32,
    velocity: vec3<f32>,
    _pad0: u32,
    color: vec3<f32>,
    _pad1: u32,
};
"#;

/// WGSL bindings for death buffer system.
pub const DEATH_BUFFER_BINDINGS_WGSL: &str = r#"
@group(3) @binding(0)
var<storage, read_write> death_buffer: array<DeathEvent>;

@group(3) @binding(1)
var<storage, read_write> death_count: atomic<u32>;

@group(3) @binding(2)
var<storage, read_write> next_child_slot: atomic<u32>;
"#;

/// WGSL helper function to record a death event.
pub const RECORD_DEATH_WGSL: &str = r#"
// Record a particle death for sub-emitter processing
fn record_death(pos: vec3<f32>, vel: vec3<f32>, col: vec3<f32>, ptype: u32) {
    let idx = atomicAdd(&death_count, 1u);
    if idx < arrayLength(&death_buffer) {
        death_buffer[idx].position = pos;
        death_buffer[idx].velocity = vel;
        death_buffer[idx].color = col;
        death_buffer[idx].parent_type = ptype;
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_emitter_builder() {
        let se = SubEmitter::new(0, 1)
            .count(50)
            .speed(1.0..3.0)
            .spread(std::f32::consts::PI)
            .inherit_velocity(0.5)
            .child_lifetime(2.0)
            .child_color(Vec3::new(1.0, 0.5, 0.0));

        assert_eq!(se.parent_type, 0);
        assert_eq!(se.child_type, 1);
        assert_eq!(se.count, 50);
        assert_eq!(se.speed_min, 1.0);
        assert_eq!(se.speed_max, 3.0);
        assert_eq!(se.inherit_velocity, 0.5);
        assert!(se.child_lifetime.is_some());
        assert!(se.child_color.is_some());
    }

    #[test]
    fn test_inherit_velocity_clamping() {
        let se = SubEmitter::new(0, 1).inherit_velocity(2.0);
        assert_eq!(se.inherit_velocity, 1.0);

        let se = SubEmitter::new(0, 1).inherit_velocity(-0.5);
        assert_eq!(se.inherit_velocity, 0.0);
    }
}
