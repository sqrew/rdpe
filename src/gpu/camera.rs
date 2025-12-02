//! Camera for 3D orbit view.

use glam::{Mat4, Vec3};

/// Orbit camera for viewing particle simulations.
pub struct Camera {
    /// Horizontal rotation angle in radians.
    pub yaw: f32,
    /// Vertical rotation angle in radians.
    pub pitch: f32,
    /// Distance from the target point.
    pub distance: f32,
    /// Point the camera orbits around.
    pub target: Vec3,
}

impl Camera {
    /// Create a new camera with default positioning.
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.3,
            distance: 3.0,
            target: Vec3::ZERO,
        }
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
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}
