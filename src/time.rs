//! Time facilities for simulation timing.
//!
//! Provides a universal source of truth for time-related values across the simulation.
//! Uses `std::time` for high-precision timing with no external dependencies.
//!
//! # Example
//!
//! ```ignore
//! use rdpe::time::Time;
//!
//! let mut time = Time::new();
//!
//! // In your game loop:
//! time.update();
//!
//! println!("Elapsed: {:.2}s", time.elapsed());
//! println!("Delta: {:.4}s", time.delta());
//! println!("Frame: {}", time.frame());
//! println!("FPS: {:.1}", time.fps());
//! ```

use std::time::{Duration, Instant};

/// Time tracking for simulations and rendering.
///
/// Provides consistent timing information including elapsed time, delta time,
/// frame counting, and FPS calculation.
#[derive(Debug)]
pub struct Time {
    /// When the timer was created.
    start: Instant,
    /// When the last frame occurred.
    last_frame: Instant,
    /// Total elapsed time in seconds (cached for fast access).
    elapsed_secs: f32,
    /// Time since last frame in seconds.
    delta_secs: f32,
    /// Total frames since start.
    frame_count: u64,
    /// Calculated FPS (updated periodically).
    fps: f32,
    /// Frame count at last FPS update.
    fps_frame_count: u64,
    /// Time of last FPS calculation.
    fps_update_time: Instant,
    /// How often to update FPS calculation.
    fps_update_interval: Duration,
    /// Whether time is paused.
    paused: bool,
    /// Elapsed time when paused.
    pause_elapsed: Duration,
    /// Fixed delta time for deterministic updates (optional).
    fixed_delta: Option<f32>,
    /// Time scale multiplier (1.0 = normal speed).
    time_scale: f32,
}

impl Time {
    /// Create a new time tracker starting from now.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_frame: now,
            elapsed_secs: 0.0,
            delta_secs: 0.0,
            frame_count: 0,
            fps: 0.0,
            fps_frame_count: 0,
            fps_update_time: now,
            fps_update_interval: Duration::from_millis(500),
            paused: false,
            pause_elapsed: Duration::ZERO,
            fixed_delta: None,
            time_scale: 1.0,
        }
    }

    /// Update timing values. Call once per frame.
    ///
    /// Returns `(elapsed_time, delta_time)` for convenience.
    pub fn update(&mut self) -> (f32, f32) {
        let now = Instant::now();

        if self.paused {
            self.delta_secs = 0.0;
            return (self.elapsed_secs, self.delta_secs);
        }

        // Calculate delta time
        let raw_delta = now.duration_since(self.last_frame).as_secs_f32();
        self.delta_secs = self.fixed_delta.unwrap_or(raw_delta) * self.time_scale;
        self.last_frame = now;

        // Calculate elapsed time
        let raw_elapsed = now.duration_since(self.start) - self.pause_elapsed;
        self.elapsed_secs = raw_elapsed.as_secs_f32() * self.time_scale;

        // Update frame count
        self.frame_count += 1;

        // Update FPS periodically
        let fps_elapsed = now.duration_since(self.fps_update_time);
        if fps_elapsed >= self.fps_update_interval {
            let frames_since = self.frame_count - self.fps_frame_count;
            self.fps = frames_since as f32 / fps_elapsed.as_secs_f32();
            self.fps_frame_count = self.frame_count;
            self.fps_update_time = now;
        }

        (self.elapsed_secs, self.delta_secs)
    }

    /// Total elapsed time in seconds since start.
    #[inline]
    pub fn elapsed(&self) -> f32 {
        self.elapsed_secs
    }

    /// Time since last frame in seconds (delta time).
    #[inline]
    pub fn delta(&self) -> f32 {
        self.delta_secs
    }

    /// Total frames since start.
    #[inline]
    pub fn frame(&self) -> u64 {
        self.frame_count
    }

    /// Calculated frames per second.
    #[inline]
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Whether time is currently paused.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Current time scale multiplier.
    #[inline]
    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    /// Pause time progression.
    ///
    /// While paused, `delta()` returns 0 and `elapsed()` stops increasing.
    pub fn pause(&mut self) {
        if !self.paused {
            self.paused = true;
        }
    }

    /// Resume time progression after pausing.
    pub fn resume(&mut self) {
        if self.paused {
            let now = Instant::now();
            self.pause_elapsed += now.duration_since(self.last_frame);
            self.last_frame = now;
            self.paused = false;
        }
    }

    /// Toggle pause state.
    pub fn toggle_pause(&mut self) {
        if self.paused {
            self.resume();
        } else {
            self.pause();
        }
    }

    /// Set a fixed delta time for deterministic updates.
    ///
    /// Useful for physics simulations that need consistent timesteps.
    /// Pass `None` to use real frame timing.
    pub fn set_fixed_delta(&mut self, delta: Option<f32>) {
        self.fixed_delta = delta;
    }

    /// Set time scale multiplier.
    ///
    /// - `1.0` = normal speed
    /// - `0.5` = half speed (slow motion)
    /// - `2.0` = double speed
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0);
    }

    /// Reset the timer to its initial state.
    pub fn reset(&mut self) {
        let now = Instant::now();
        self.start = now;
        self.last_frame = now;
        self.elapsed_secs = 0.0;
        self.delta_secs = 0.0;
        self.frame_count = 0;
        self.fps = 0.0;
        self.fps_frame_count = 0;
        self.fps_update_time = now;
        self.paused = false;
        self.pause_elapsed = Duration::ZERO;
    }

    /// Get the raw start instant.
    #[inline]
    pub fn start_instant(&self) -> Instant {
        self.start
    }

    /// Get elapsed time as a Duration.
    #[inline]
    pub fn elapsed_duration(&self) -> Duration {
        self.start.elapsed() - self.pause_elapsed
    }

    /// Get delta time as a Duration.
    #[inline]
    pub fn delta_duration(&self) -> Duration {
        Duration::from_secs_f32(self.delta_secs)
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_time_new() {
        let time = Time::new();
        assert_eq!(time.frame(), 0);
        assert!(!time.is_paused());
        assert_eq!(time.time_scale(), 1.0);
    }

    #[test]
    fn test_time_update() {
        let mut time = Time::new();
        thread::sleep(Duration::from_millis(10));
        let (elapsed, delta) = time.update();

        assert!(elapsed > 0.0);
        assert!(delta > 0.0);
        assert_eq!(time.frame(), 1);
    }

    #[test]
    fn test_time_pause() {
        let mut time = Time::new();
        time.update();

        time.pause();
        assert!(time.is_paused());

        let elapsed_before = time.elapsed();
        thread::sleep(Duration::from_millis(10));
        time.update();

        // Elapsed should not increase while paused
        assert_eq!(time.elapsed(), elapsed_before);
        assert_eq!(time.delta(), 0.0);
    }

    #[test]
    fn test_time_scale() {
        let mut time = Time::new();
        time.set_time_scale(2.0);
        assert_eq!(time.time_scale(), 2.0);

        // Negative scale should clamp to 0
        time.set_time_scale(-1.0);
        assert_eq!(time.time_scale(), 0.0);
    }

    #[test]
    fn test_fixed_delta() {
        let mut time = Time::new();
        time.set_fixed_delta(Some(1.0 / 60.0));

        thread::sleep(Duration::from_millis(100));
        time.update();

        // Should use fixed delta regardless of actual time
        let expected = 1.0 / 60.0;
        assert!((time.delta() - expected).abs() < 0.0001);
    }
}
