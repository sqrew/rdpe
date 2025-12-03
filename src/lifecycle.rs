//! Particle lifecycle management.
//!
//! This module provides ergonomic tools for configuring particle lifecycles,
//! including aging, death, visual effects, and respawning via emitters.
//!
//! # Hidden Lifecycle Fields
//!
//! Every particle automatically has these fields (injected by the derive macro):
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `age` | `f32` | Time since spawn/respawn (seconds) |
//! | `alive` | `u32` | 0 = dead (skip simulation), 1 = alive |
//! | `scale` | `f32` | Visual size multiplier (1.0 = normal) |
//!
//! These are accessible in custom WGSL rules via `p.age`, `p.alive`, `p.scale`.
//!
//! # Quick Start
//!
//! ```ignore
//! Simulation::<Spark>::new()
//!     .with_lifecycle(|l| {
//!         l.lifetime(2.0)
//!          .fade_out()
//!          .emitter(Emitter::Point {
//!              position: Vec3::ZERO,
//!              rate: 500.0,
//!              speed: 1.0,
//!          });
//!     })
//!     .with_rule(Rule::Gravity(9.8))
//!     .run();
//! ```
//!
//! # Lifecycle Presets
//!
//! Common particle system patterns available as one-liners:
//!
//! ```ignore
//! .with_lifecycle(Lifecycle::fire(Vec3::ZERO, 800.0))
//! .with_lifecycle(Lifecycle::fountain(Vec3::new(0.0, -0.5, 0.0), 1000.0))
//! .with_lifecycle(Lifecycle::explosion(Vec3::ZERO, 500))
//! ```

use crate::emitter::Emitter;
use crate::rules::Rule;
use glam::Vec3;
use std::ops::Range;

/// Lifecycle configuration builder.
///
/// Collects lifecycle settings and generates the appropriate rules and emitters.
///
/// # Example
///
/// ```ignore
/// .with_lifecycle(|l| {
///     l.lifetime(2.0..4.0)     // Random lifetime per particle
///      .fade_out()             // Dim as particle ages
///      .shrink_out()           // Shrink as particle ages
///      .color_over_life(       // Color gradient over lifetime
///          Vec3::new(1.0, 1.0, 0.0),  // Yellow at birth
///          Vec3::new(1.0, 0.0, 0.0),  // Red at death
///      )
///      .emitter(Emitter::Cone { ... });
/// })
/// ```
#[derive(Default, Clone)]
pub struct Lifecycle {
    /// Fixed lifetime in seconds (if set).
    lifetime_fixed: Option<f32>,
    /// Random lifetime range (if set, overrides fixed).
    lifetime_range: Option<Range<f32>>,
    /// Whether to fade out color over lifetime.
    fade_out: bool,
    /// Whether to shrink scale over lifetime.
    shrink_out: bool,
    /// Color transition over lifetime (start, end).
    color_over_life: Option<(Vec3, Vec3)>,
    /// Emitters to add.
    emitters: Vec<Emitter>,
    /// Start particles dead (for emitter-only spawning).
    start_dead: bool,
}

impl Lifecycle {
    /// Create a new empty lifecycle configuration.
    pub fn new() -> Self {
        Self::default()
    }

    // =========================================================================
    // PRESETS
    // =========================================================================

    /// Fire preset: rising embers that fade and shrink.
    ///
    /// Creates warm-colored particles that rise, fade out, and shrink.
    /// Great for campfires, torches, and flame effects.
    ///
    /// # Arguments
    ///
    /// * `position` - Emission point (base of fire)
    /// * `rate` - Particles per second
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_lifecycle(Lifecycle::fire(Vec3::new(0.0, -0.5, 0.0), 1000.0))
    /// ```
    pub fn fire(position: Vec3, rate: f32) -> Self {
        Self {
            lifetime_fixed: Some(1.5),
            fade_out: true,
            shrink_out: true,
            color_over_life: Some((
                Vec3::new(1.0, 0.9, 0.3),  // Bright yellow-white
                Vec3::new(0.8, 0.2, 0.0),  // Deep red-orange
            )),
            emitters: vec![Emitter::Cone {
                position,
                direction: Vec3::Y,
                speed: 0.8,
                spread: 0.4,
                rate,
            }],
            start_dead: true,
            ..Default::default()
        }
    }

    /// Fountain preset: particles arc up and fall down.
    ///
    /// Classic water fountain effect with particles shooting up,
    /// arcing, and fading as they fall.
    ///
    /// # Arguments
    ///
    /// * `position` - Fountain nozzle position
    /// * `rate` - Particles per second
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_lifecycle(Lifecycle::fountain(Vec3::new(0.0, -0.8, 0.0), 800.0))
    /// ```
    pub fn fountain(position: Vec3, rate: f32) -> Self {
        Self {
            lifetime_fixed: Some(3.0),
            fade_out: true,
            shrink_out: false,
            color_over_life: Some((
                Vec3::new(0.7, 0.85, 1.0),  // Light blue
                Vec3::new(0.2, 0.4, 0.8),   // Deeper blue
            )),
            emitters: vec![Emitter::Cone {
                position,
                direction: Vec3::Y,
                speed: 2.5,
                spread: 0.2,
                rate,
            }],
            start_dead: true,
            ..Default::default()
        }
    }

    /// Explosion preset: one-time burst of particles.
    ///
    /// Radial explosion that fades quickly. Particles spawn once,
    /// fly outward, slow down, and fade.
    ///
    /// # Arguments
    ///
    /// * `position` - Explosion center
    /// * `count` - Number of particles in burst
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_lifecycle(Lifecycle::explosion(Vec3::ZERO, 500))
    /// ```
    pub fn explosion(position: Vec3, count: u32) -> Self {
        Self {
            lifetime_fixed: Some(1.2),
            fade_out: true,
            shrink_out: true,
            color_over_life: Some((
                Vec3::new(1.0, 1.0, 0.8),  // Bright flash
                Vec3::new(1.0, 0.3, 0.0),  // Orange ember
            )),
            emitters: vec![Emitter::Burst {
                position,
                count,
                speed: 3.0,
            }],
            start_dead: true,
            ..Default::default()
        }
    }

    /// Smoke preset: slow-rising particles that expand and fade.
    ///
    /// Billowing smoke effect with particles that grow as they rise.
    ///
    /// # Arguments
    ///
    /// * `position` - Smoke source position
    /// * `rate` - Particles per second
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_lifecycle(Lifecycle::smoke(Vec3::ZERO, 300.0))
    /// ```
    pub fn smoke(position: Vec3, rate: f32) -> Self {
        Self {
            lifetime_fixed: Some(4.0),
            fade_out: true,
            shrink_out: false, // Smoke expands, doesn't shrink
            color_over_life: Some((
                Vec3::new(0.4, 0.4, 0.4),  // Medium gray
                Vec3::new(0.15, 0.15, 0.15), // Dark gray
            )),
            emitters: vec![Emitter::Cone {
                position,
                direction: Vec3::Y,
                speed: 0.3,
                spread: 0.6,
                rate,
            }],
            start_dead: true,
            ..Default::default()
        }
    }

    /// Sparkler preset: erratic sparks flying in all directions.
    ///
    /// Fast, short-lived particles that spray outward like a firework sparkler.
    ///
    /// # Arguments
    ///
    /// * `position` - Sparkler tip position
    /// * `rate` - Particles per second
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_lifecycle(Lifecycle::sparkler(Vec3::ZERO, 2000.0))
    /// ```
    pub fn sparkler(position: Vec3, rate: f32) -> Self {
        Self {
            lifetime_fixed: Some(0.5),
            fade_out: true,
            shrink_out: true,
            color_over_life: Some((
                Vec3::new(1.0, 1.0, 1.0),  // White hot
                Vec3::new(1.0, 0.6, 0.1),  // Orange
            )),
            emitters: vec![Emitter::Sphere {
                center: position,
                radius: 0.02,
                speed: 2.0,
                rate,
            }],
            start_dead: true,
            ..Default::default()
        }
    }

    /// Rain preset: particles falling downward.
    ///
    /// # Arguments
    ///
    /// * `rate` - Particles per second
    ///
    /// # Example
    ///
    /// ```ignore
    /// .with_lifecycle(Lifecycle::rain(500.0))
    /// ```
    pub fn rain(rate: f32) -> Self {
        Self {
            lifetime_fixed: Some(2.0),
            fade_out: false,
            shrink_out: false,
            color_over_life: Some((
                Vec3::new(0.6, 0.7, 0.9),  // Light blue-gray
                Vec3::new(0.4, 0.5, 0.7),  // Slightly darker
            )),
            emitters: vec![Emitter::Box {
                min: Vec3::new(-1.0, 0.9, -1.0),
                max: Vec3::new(1.0, 1.0, 1.0),
                velocity: Vec3::new(0.0, -2.0, 0.0),
                rate,
            }],
            start_dead: true,
            ..Default::default()
        }
    }

    // =========================================================================
    // BUILDER METHODS
    // =========================================================================

    /// Set a fixed lifetime for all particles.
    ///
    /// Particles die after this many seconds.
    ///
    /// # Example
    ///
    /// ```ignore
    /// l.lifetime(2.0)  // All particles live 2 seconds
    /// ```
    pub fn lifetime(mut self, seconds: f32) -> Self {
        self.lifetime_fixed = Some(seconds);
        self.lifetime_range = None;
        self
    }

    /// Set a random lifetime range.
    ///
    /// Each particle gets a random lifetime within this range.
    /// (Note: Currently uses fixed value at midpoint; true randomness
    /// requires runtime uniform or per-particle field.)
    ///
    /// # Example
    ///
    /// ```ignore
    /// l.lifetime_range(1.0..3.0)  // Live between 1-3 seconds
    /// ```
    pub fn lifetime_range(mut self, range: Range<f32>) -> Self {
        self.lifetime_range = Some(range.clone());
        // For now, use midpoint. True randomness would need per-particle storage.
        self.lifetime_fixed = Some((range.start + range.end) / 2.0);
        self
    }

    /// Enable fade out effect.
    ///
    /// Particle color dims to black as it approaches death.
    ///
    /// # Example
    ///
    /// ```ignore
    /// l.lifetime(2.0).fade_out()
    /// ```
    pub fn fade_out(mut self) -> Self {
        self.fade_out = true;
        self
    }

    /// Enable shrink out effect.
    ///
    /// Particle scale shrinks to zero as it approaches death.
    ///
    /// # Example
    ///
    /// ```ignore
    /// l.lifetime(2.0).shrink_out()
    /// ```
    pub fn shrink_out(mut self) -> Self {
        self.shrink_out = true;
        self
    }

    /// Set color gradient over lifetime.
    ///
    /// Particle color transitions from `start` to `end` over its lifetime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// l.color_over_life(
    ///     Vec3::new(1.0, 1.0, 0.0),  // Yellow at birth
    ///     Vec3::new(1.0, 0.0, 0.0),  // Red at death
    /// )
    /// ```
    pub fn color_over_life(mut self, start: Vec3, end: Vec3) -> Self {
        self.color_over_life = Some((start, end));
        self
    }

    /// Add an emitter for respawning dead particles.
    ///
    /// # Example
    ///
    /// ```ignore
    /// l.emitter(Emitter::Point {
    ///     position: Vec3::ZERO,
    ///     rate: 500.0,
    ///     speed: 1.0,
    /// })
    /// ```
    pub fn emitter(mut self, emitter: Emitter) -> Self {
        self.emitters.push(emitter);
        self
    }

    /// Start all particles dead (emitter-only spawning).
    ///
    /// When true, the spawner creates dead particles (`alive = 0`)
    /// and emitters are responsible for all spawning.
    ///
    /// # Example
    ///
    /// ```ignore
    /// l.start_dead()
    ///  .emitter(Emitter::Point { ... })
    /// ```
    pub fn start_dead(mut self) -> Self {
        self.start_dead = true;
        self
    }

    // =========================================================================
    // INTERNAL: GENERATE RULES AND EMITTERS
    // =========================================================================

    /// Get the lifetime duration (for reference).
    pub fn get_lifetime(&self) -> Option<f32> {
        self.lifetime_fixed
    }

    /// Build the lifecycle configuration into rules and emitters.
    ///
    /// Returns a tuple of (rules, emitters, start_dead).
    pub(crate) fn build(self) -> (Vec<Rule>, Vec<Emitter>, bool) {
        let mut rules = Vec::new();

        // Always need Age rule if we have any lifecycle features
        let has_lifecycle = self.lifetime_fixed.is_some()
            || self.fade_out
            || self.shrink_out
            || self.color_over_life.is_some();

        if has_lifecycle {
            rules.push(Rule::Age);
        }

        // Add lifetime rule
        if let Some(duration) = self.lifetime_fixed {
            rules.push(Rule::Lifetime(duration));

            // Fade and shrink use the same duration
            if self.fade_out {
                rules.push(Rule::FadeOut(duration));
            }
            if self.shrink_out {
                rules.push(Rule::ShrinkOut(duration));
            }
            if let Some((start, end)) = self.color_over_life {
                rules.push(Rule::ColorOverLife {
                    start,
                    end,
                    duration,
                });
            }
        }

        (rules, self.emitters, self.start_dead)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire_preset() {
        let lifecycle = Lifecycle::fire(Vec3::ZERO, 500.0);
        let (rules, emitters, start_dead) = lifecycle.build();

        assert!(start_dead);
        assert_eq!(emitters.len(), 1);
        assert!(rules.iter().any(|r| matches!(r, Rule::Age)));
        assert!(rules.iter().any(|r| matches!(r, Rule::Lifetime(_))));
        assert!(rules.iter().any(|r| matches!(r, Rule::FadeOut(_))));
        assert!(rules.iter().any(|r| matches!(r, Rule::ShrinkOut(_))));
    }

    #[test]
    fn test_builder_chain() {
        let lifecycle = Lifecycle::new()
            .lifetime(2.0)
            .fade_out()
            .emitter(Emitter::Point {
                position: Vec3::ZERO,
                rate: 100.0,
                speed: 1.0,
            });

        let (rules, emitters, _) = lifecycle.build();

        assert_eq!(emitters.len(), 1);
        assert!(rules.iter().any(|r| matches!(r, Rule::Age)));
        assert!(rules.iter().any(|r| matches!(r, Rule::Lifetime(2.0))));
        assert!(rules.iter().any(|r| matches!(r, Rule::FadeOut(2.0))));
    }
}
