//! Spawn context for particle initialization.
//!
//! Provides helper methods to reduce boilerplate when spawning particles.

use crate::Vec3;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::f32::consts::{PI, TAU};

/// Context provided to spawner functions with helpers for common spawn patterns.
///
/// Instead of manually setting up RNG and computing random positions, use the
/// helper methods on `SpawnContext`:
///
/// ```ignore
/// // Before: verbose manual setup
/// let mut rng = rand::thread_rng();
/// let particles: Vec<Spark> = (0..count)
///     .map(|_| {
///         let theta = rng.gen_range(0.0..TAU);
///         let phi = rng.gen_range(0.0..PI);
///         let r = rng.gen_range(0.0..0.5);
///         let x = r * phi.sin() * theta.cos();
///         let y = r * phi.sin() * theta.sin();
///         let z = r * phi.cos();
///         Spark { position: Vec3::new(x, y, z), velocity: Vec3::ZERO, color: Vec3::ONE }
///     })
///     .collect();
/// sim.with_spawner(move |i, _| particles[i].clone())
///
/// // After: clean and simple
/// sim.with_spawner(|ctx| Spark {
///     position: ctx.random_in_sphere(0.5),
///     velocity: Vec3::ZERO,
///     color: Vec3::ONE,
/// })
/// ```
pub struct SpawnContext {
    /// Index of the particle being spawned (0 to count-1).
    pub index: u32,
    /// Total number of particles being spawned.
    pub count: u32,
    /// Simulation bounds (half-size of bounding cube).
    pub bounds: f32,
    /// Internal RNG - use helper methods instead of accessing directly.
    rng: SmallRng,
}

impl SpawnContext {
    /// Create a new spawn context for a particle.
    pub(crate) fn new(index: u32, count: u32, bounds: f32) -> Self {
        // Seed RNG based on index for reproducibility within a run,
        // but different each program execution
        let seed = index as u64 ^ (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42));

        Self {
            index,
            count,
            bounds,
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    /// Normalized progress through the spawn (0.0 to 1.0).
    ///
    /// Useful for distributing particles evenly:
    /// ```ignore
    /// let angle = ctx.progress() * TAU;  // Particles around a circle
    /// ```
    #[inline]
    pub fn progress(&self) -> f32 {
        self.index as f32 / self.count as f32
    }

    // ========== Random primitives ==========

    /// Random f32 between 0.0 and 1.0.
    #[inline]
    pub fn random(&mut self) -> f32 {
        self.rng.gen()
    }

    /// Random f32 in the given range.
    #[inline]
    pub fn random_range(&mut self, min: f32, max: f32) -> f32 {
        self.rng.gen_range(min..max)
    }

    /// Random i32 in the given range.
    #[inline]
    pub fn random_int(&mut self, min: i32, max: i32) -> i32 {
        self.rng.gen_range(min..max)
    }

    /// Random u32 in the given range.
    #[inline]
    pub fn random_uint(&mut self, min: u32, max: u32) -> u32 {
        self.rng.gen_range(min..max)
    }

    // ========== Position helpers ==========

    /// Random point inside a sphere of given radius, centered at origin.
    ///
    /// Distribution is uniform throughout the volume.
    pub fn random_in_sphere(&mut self, radius: f32) -> Vec3 {
        let theta = self.rng.gen_range(0.0..TAU);
        let phi = self.rng.gen_range(0.0..PI);
        // Cube root for uniform volume distribution
        let r = radius * self.rng.gen::<f32>().cbrt();

        Vec3::new(
            r * phi.sin() * theta.cos(),
            r * phi.sin() * theta.sin(),
            r * phi.cos(),
        )
    }

    /// Random point on the surface of a sphere of given radius.
    pub fn random_on_sphere(&mut self, radius: f32) -> Vec3 {
        let theta = self.rng.gen_range(0.0..TAU);
        let phi = self.rng.gen_range(0.0..PI);

        Vec3::new(
            radius * phi.sin() * theta.cos(),
            radius * phi.sin() * theta.sin(),
            radius * phi.cos(),
        )
    }

    /// Random point inside a cube of given half-size, centered at origin.
    ///
    /// For a cube from -1 to 1, use `half_size = 1.0`.
    pub fn random_in_cube(&mut self, half_size: f32) -> Vec3 {
        Vec3::new(
            self.rng.gen_range(-half_size..half_size),
            self.rng.gen_range(-half_size..half_size),
            self.rng.gen_range(-half_size..half_size),
        )
    }

    /// Random point within the simulation bounds.
    ///
    /// Equivalent to `random_in_cube(ctx.bounds)`.
    pub fn random_in_bounds(&mut self) -> Vec3 {
        self.random_in_cube(self.bounds)
    }

    /// Random point inside a cylinder along the Y axis.
    ///
    /// * `radius` - Cylinder radius in XZ plane
    /// * `half_height` - Half the cylinder height
    pub fn random_in_cylinder(&mut self, radius: f32, half_height: f32) -> Vec3 {
        let theta = self.rng.gen_range(0.0..TAU);
        let r = radius * self.rng.gen::<f32>().sqrt(); // sqrt for uniform disk

        Vec3::new(
            r * theta.cos(),
            self.rng.gen_range(-half_height..half_height),
            r * theta.sin(),
        )
    }

    /// Random point inside a disk in the XZ plane at y=0.
    pub fn random_in_disk(&mut self, radius: f32) -> Vec3 {
        let theta = self.rng.gen_range(0.0..TAU);
        let r = radius * self.rng.gen::<f32>().sqrt();

        Vec3::new(r * theta.cos(), 0.0, r * theta.sin())
    }

    /// Random point on a ring (circle) in the XZ plane at y=0.
    pub fn random_on_ring(&mut self, radius: f32) -> Vec3 {
        let theta = self.rng.gen_range(0.0..TAU);
        Vec3::new(radius * theta.cos(), 0.0, radius * theta.sin())
    }

    // ========== Direction/velocity helpers ==========

    /// Random unit vector (uniformly distributed on unit sphere).
    pub fn random_direction(&mut self) -> Vec3 {
        self.random_on_sphere(1.0).normalize()
    }

    /// Random velocity tangent to position (for orbital motion).
    ///
    /// Returns a velocity perpendicular to the position vector, in the XZ plane.
    /// Useful for setting up swirling/orbiting particles.
    pub fn tangent_velocity(&self, position: Vec3, speed: f32) -> Vec3 {
        let tangent = Vec3::new(-position.z, 0.0, position.x);
        if tangent.length_squared() > 0.0001 {
            tangent.normalize() * speed
        } else {
            Vec3::new(speed, 0.0, 0.0)
        }
    }

    /// Random velocity pointing outward from origin.
    pub fn outward_velocity(&mut self, position: Vec3, speed: f32) -> Vec3 {
        if position.length_squared() > 0.0001 {
            position.normalize() * speed
        } else {
            self.random_direction() * speed
        }
    }

    // ========== Color helpers ==========

    /// Random RGB color (each channel 0-1).
    pub fn random_color(&mut self) -> Vec3 {
        Vec3::new(self.rng.gen(), self.rng.gen(), self.rng.gen())
    }

    /// Random color with given saturation and value (HSV model).
    ///
    /// Hue is randomized, giving vibrant varied colors.
    pub fn random_hue(&mut self, saturation: f32, value: f32) -> Vec3 {
        let hue = self.rng.gen::<f32>();
        hsv_to_rgb(hue, saturation, value)
    }

    /// Color from HSV values.
    ///
    /// * `hue` - 0.0 to 1.0 (wraps: red → yellow → green → cyan → blue → magenta → red)
    /// * `saturation` - 0.0 (gray) to 1.0 (vivid)
    /// * `value` - 0.0 (black) to 1.0 (bright)
    pub fn hsv(&self, hue: f32, saturation: f32, value: f32) -> Vec3 {
        hsv_to_rgb(hue, saturation, value)
    }

    /// Color based on spawn progress (rainbow gradient).
    ///
    /// First particle is red, middle is cyan, last is back to red.
    pub fn rainbow(&mut self, saturation: f32, value: f32) -> Vec3 {
        hsv_to_rgb(self.progress(), saturation, value)
    }

    // ========== Grid/structured layouts ==========

    /// Position in a 3D grid layout.
    ///
    /// Distributes particles evenly in a grid within bounds.
    ///
    /// * `cols` - Number of columns (X axis)
    /// * `rows` - Number of rows (Y axis)
    /// * `layers` - Number of layers (Z axis)
    pub fn grid_position(&self, cols: u32, rows: u32, layers: u32) -> Vec3 {
        let total = cols * rows * layers;
        let idx = self.index % total;

        let x = idx % cols;
        let y = (idx / cols) % rows;
        let z = idx / (cols * rows);

        let fx = (x as f32 / (cols - 1).max(1) as f32) * 2.0 - 1.0;
        let fy = (y as f32 / (rows - 1).max(1) as f32) * 2.0 - 1.0;
        let fz = (z as f32 / (layers - 1).max(1) as f32) * 2.0 - 1.0;

        Vec3::new(fx * self.bounds, fy * self.bounds, fz * self.bounds)
    }

    /// Position on a 2D grid in the XZ plane (y=0).
    pub fn grid_position_2d(&self, cols: u32, rows: u32) -> Vec3 {
        let idx = self.index % (cols * rows);
        let x = idx % cols;
        let z = idx / cols;

        let fx = (x as f32 / (cols - 1).max(1) as f32) * 2.0 - 1.0;
        let fz = (z as f32 / (rows - 1).max(1) as f32) * 2.0 - 1.0;

        Vec3::new(fx * self.bounds, 0.0, fz * self.bounds)
    }

    /// Position along a line from `start` to `end`.
    ///
    /// Particles are distributed evenly along the line.
    pub fn line_position(&self, start: Vec3, end: Vec3) -> Vec3 {
        start + (end - start) * self.progress()
    }

    /// Position on a circle in the XZ plane.
    ///
    /// Particles are distributed evenly around the circle.
    pub fn circle_position(&self, radius: f32) -> Vec3 {
        let angle = self.progress() * TAU;
        Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin())
    }

    /// Position on a helix/spiral.
    ///
    /// * `radius` - Helix radius
    /// * `height` - Total height of helix
    /// * `turns` - Number of complete rotations
    pub fn helix_position(&self, radius: f32, height: f32, turns: f32) -> Vec3 {
        let t = self.progress();
        let angle = t * TAU * turns;
        Vec3::new(
            radius * angle.cos(),
            (t - 0.5) * height,
            radius * angle.sin(),
        )
    }
}

/// Convert HSV to RGB.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match (h * 6.0) as u32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Vec3::new(r + m, g + m, b + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_context_progress() {
        let ctx = SpawnContext::new(50, 100, 1.0);
        assert!((ctx.progress() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_random_in_sphere_bounds() {
        let mut ctx = SpawnContext::new(0, 1, 1.0);
        for _ in 0..100 {
            let pos = ctx.random_in_sphere(0.5);
            assert!(pos.length() <= 0.5 + 0.001);
        }
    }

    #[test]
    fn test_grid_position() {
        let ctx = SpawnContext::new(0, 27, 1.0);
        let pos = ctx.grid_position(3, 3, 3);
        assert!((pos.x - (-1.0)).abs() < 0.001);
        assert!((pos.y - (-1.0)).abs() < 0.001);
        assert!((pos.z - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_hsv_to_rgb() {
        // Red
        let red = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((red.x - 1.0).abs() < 0.001);
        assert!(red.y < 0.001);
        assert!(red.z < 0.001);
    }
}
