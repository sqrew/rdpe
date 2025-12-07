//! Camera for 3D orbit view with smooth controls.

use glam::{Mat4, Vec3};

/// Orbit camera with smooth interpolation and full movement controls.
///
/// # Controls (when wired up)
///
/// - **WASD** - Move target along camera's local XZ plane
/// - **Q/E** - Move target down/up
/// - **Right drag** - Orbit (yaw/pitch rotation)
/// - **Scroll** - Zoom in/out
/// - **Shift** - Speed multiplier
/// - **R** - Reset to default position
/// - **Left click** - Reserved
/// - **Middle click** - Reserved
pub struct Camera {
    // Current interpolated state
    /// Horizontal rotation angle in radians.
    pub yaw: f32,
    /// Vertical rotation angle in radians.
    pub pitch: f32,
    /// Distance from the target point.
    pub distance: f32,
    /// Point the camera orbits around.
    pub target: Vec3,

    // Target state (what we're interpolating toward)
    yaw_target: f32,
    pitch_target: f32,
    distance_target: f32,
    target_target: Vec3,

    // Settings
    /// Base movement speed (units per second).
    pub move_speed: f32,
    /// Rotation sensitivity (radians per pixel).
    pub rotate_speed: f32,
    /// Zoom sensitivity.
    pub zoom_speed: f32,
    /// Pan sensitivity (units per pixel).
    pub pan_speed: f32,
    /// Smoothing factor (0 = instant, higher = smoother). ~10-20 is good.
    pub smoothing: f32,
    /// Speed multiplier when shift is held.
    pub sprint_multiplier: f32,

    // Limits
    /// Minimum pitch angle (radians). Prevents looking straight down.
    pub pitch_min: f32,
    /// Maximum pitch angle (radians). Prevents looking straight up.
    pub pitch_max: f32,
    /// Minimum zoom distance.
    pub distance_min: f32,
    /// Maximum zoom distance.
    pub distance_max: f32,

    // Default values for reset
    default_yaw: f32,
    default_pitch: f32,
    default_distance: f32,
    default_target: Vec3,
}

impl Camera {
    /// Create a new camera with default positioning.
    pub fn new() -> Self {
        let default_yaw = 0.0;
        let default_pitch = 0.3;
        let default_distance = 3.0;
        let default_target = Vec3::ZERO;

        Self {
            yaw: default_yaw,
            pitch: default_pitch,
            distance: default_distance,
            target: default_target,

            yaw_target: default_yaw,
            pitch_target: default_pitch,
            distance_target: default_distance,
            target_target: default_target,

            move_speed: 2.0,
            rotate_speed: 0.005,
            zoom_speed: 0.15,
            pan_speed: 0.003,
            smoothing: 15.0,
            sprint_multiplier: 3.0,

            pitch_min: -1.5,
            pitch_max: 1.5,
            distance_min: 0.5,
            distance_max: 50.0,

            default_yaw,
            default_pitch,
            default_distance,
            default_target,
        }
    }

    /// Update camera state with smooth interpolation.
    ///
    /// Call this every frame with the delta time.
    pub fn update(&mut self, dt: f32) {
        let t = 1.0 - (-self.smoothing * dt).exp();

        self.yaw = lerp(self.yaw, self.yaw_target, t);
        self.pitch = lerp(self.pitch, self.pitch_target, t);
        self.distance = lerp(self.distance, self.distance_target, t);
        self.target = self.target.lerp(self.target_target, t);
    }

    /// Orbit the camera around the target point.
    ///
    /// `dx` and `dy` are typically mouse delta in pixels.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw_target -= dx * self.rotate_speed;
        self.pitch_target += dy * self.rotate_speed;
        self.pitch_target = self.pitch_target.clamp(self.pitch_min, self.pitch_max);
    }

    /// Pan the camera (move target perpendicular to view direction).
    ///
    /// `dx` and `dy` are typically mouse delta in pixels.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let right = self.right();
        let up = self.up();

        // Scale pan by distance so it feels consistent at different zoom levels
        let scale = self.distance_target * self.pan_speed;

        self.target_target -= right * dx * scale;
        self.target_target += up * dy * scale;
    }

    /// Zoom in or out.
    ///
    /// Positive values zoom in, negative zoom out.
    pub fn zoom(&mut self, amount: f32) {
        self.distance_target -= amount * self.zoom_speed * self.distance_target;
        self.distance_target = self.distance_target.clamp(self.distance_min, self.distance_max);
    }

    /// Move the target forward/backward relative to camera view.
    ///
    /// `amount` is in units (will be scaled by move_speed and dt externally).
    pub fn move_forward(&mut self, amount: f32) {
        // Forward is the direction from camera to target, projected onto XZ plane
        let forward = self.forward_xz();
        self.target_target += forward * amount;
    }

    /// Move the target left/right relative to camera view.
    ///
    /// `amount` is in units (positive = right).
    pub fn move_right(&mut self, amount: f32) {
        let right = self.right();
        self.target_target += right * amount;
    }

    /// Move the target up/down in world space.
    ///
    /// `amount` is in units (positive = up).
    pub fn move_up(&mut self, amount: f32) {
        self.target_target.y += amount;
    }

    /// Reset camera to default position.
    pub fn reset(&mut self) {
        self.yaw_target = self.default_yaw;
        self.pitch_target = self.default_pitch;
        self.distance_target = self.default_distance;
        self.target_target = self.default_target;
    }

    /// Reset camera instantly (no interpolation).
    pub fn reset_instant(&mut self) {
        self.reset();
        self.yaw = self.yaw_target;
        self.pitch = self.pitch_target;
        self.distance = self.distance_target;
        self.target = self.target_target;
    }

    /// Calculate the camera's world position.
    pub fn position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + Vec3::new(x, y, z)
    }

    /// Calculate the view matrix for rendering.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    /// Get the camera's forward direction (toward target).
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position()).normalize_or_zero()
    }

    /// Get the camera's forward direction projected onto the XZ plane.
    ///
    /// Useful for WASD movement that ignores pitch.
    pub fn forward_xz(&self) -> Vec3 {
        let forward = self.forward();
        Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero()
    }

    /// Get the camera's right direction.
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or_zero()
    }

    /// Get the camera's up direction (perpendicular to view).
    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize_or_zero()
    }

    /// Set the default position (used by reset).
    pub fn set_default(&mut self, yaw: f32, pitch: f32, distance: f32, target: Vec3) {
        self.default_yaw = yaw;
        self.default_pitch = pitch;
        self.default_distance = distance;
        self.default_target = target;
    }

    /// Jump to a position instantly (no interpolation).
    pub fn set_instant(&mut self, yaw: f32, pitch: f32, distance: f32, target: Vec3) {
        self.yaw = yaw;
        self.pitch = pitch;
        self.distance = distance;
        self.target = target;
        self.yaw_target = yaw;
        self.pitch_target = pitch;
        self.distance_target = distance;
        self.target_target = target;
    }

    /// Look at a specific point from the current distance.
    pub fn look_at(&mut self, point: Vec3) {
        self.target_target = point;
    }

    /// Look at a point instantly.
    pub fn look_at_instant(&mut self, point: Vec3) {
        self.target = point;
        self.target_target = point;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

/// Linear interpolation.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
