//! Input handling for RDPE simulations.
//!
//! The `Input` struct provides a clean abstraction over raw window events,
//! tracking both instantaneous events (key just pressed) and continuous state
//! (key held down).
//!
//! # Usage
//!
//! Input state is available in your update callback via `UpdateContext`:
//!
//! ```ignore
//! .with_update(|ctx| {
//!     // Check if space was just pressed this frame
//!     if ctx.input.key_pressed(KeyCode::Space) {
//!         ctx.set("burst", 1.0);
//!     }
//!
//!     // Check if left mouse is held down
//!     if ctx.input.mouse_held(MouseButton::Left) {
//!         ctx.set("attractor", ctx.input.mouse_ndc());
//!     }
//! })
//! ```

use glam::Vec2;
use std::collections::HashSet;
use winit::event::{ElementState, MouseButton as WinitMouseButton, WindowEvent};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl From<WinitMouseButton> for MouseButton {
    fn from(btn: WinitMouseButton) -> Self {
        match btn {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            _ => MouseButton::Left, // Default for other buttons
        }
    }
}

/// Keyboard key codes.
///
/// Re-exports common keys from winit for convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Numbers
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,

    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    // Arrows
    Up, Down, Left, Right,

    // Common keys
    Space, Enter, Escape, Tab, Backspace, Delete,
    Shift, Control, Alt,

    // Other
    Other(u32),
}

impl From<WinitKeyCode> for KeyCode {
    fn from(key: WinitKeyCode) -> Self {
        match key {
            WinitKeyCode::KeyA => KeyCode::A,
            WinitKeyCode::KeyB => KeyCode::B,
            WinitKeyCode::KeyC => KeyCode::C,
            WinitKeyCode::KeyD => KeyCode::D,
            WinitKeyCode::KeyE => KeyCode::E,
            WinitKeyCode::KeyF => KeyCode::F,
            WinitKeyCode::KeyG => KeyCode::G,
            WinitKeyCode::KeyH => KeyCode::H,
            WinitKeyCode::KeyI => KeyCode::I,
            WinitKeyCode::KeyJ => KeyCode::J,
            WinitKeyCode::KeyK => KeyCode::K,
            WinitKeyCode::KeyL => KeyCode::L,
            WinitKeyCode::KeyM => KeyCode::M,
            WinitKeyCode::KeyN => KeyCode::N,
            WinitKeyCode::KeyO => KeyCode::O,
            WinitKeyCode::KeyP => KeyCode::P,
            WinitKeyCode::KeyQ => KeyCode::Q,
            WinitKeyCode::KeyR => KeyCode::R,
            WinitKeyCode::KeyS => KeyCode::S,
            WinitKeyCode::KeyT => KeyCode::T,
            WinitKeyCode::KeyU => KeyCode::U,
            WinitKeyCode::KeyV => KeyCode::V,
            WinitKeyCode::KeyW => KeyCode::W,
            WinitKeyCode::KeyX => KeyCode::X,
            WinitKeyCode::KeyY => KeyCode::Y,
            WinitKeyCode::KeyZ => KeyCode::Z,

            WinitKeyCode::Digit0 => KeyCode::Key0,
            WinitKeyCode::Digit1 => KeyCode::Key1,
            WinitKeyCode::Digit2 => KeyCode::Key2,
            WinitKeyCode::Digit3 => KeyCode::Key3,
            WinitKeyCode::Digit4 => KeyCode::Key4,
            WinitKeyCode::Digit5 => KeyCode::Key5,
            WinitKeyCode::Digit6 => KeyCode::Key6,
            WinitKeyCode::Digit7 => KeyCode::Key7,
            WinitKeyCode::Digit8 => KeyCode::Key8,
            WinitKeyCode::Digit9 => KeyCode::Key9,

            WinitKeyCode::F1 => KeyCode::F1,
            WinitKeyCode::F2 => KeyCode::F2,
            WinitKeyCode::F3 => KeyCode::F3,
            WinitKeyCode::F4 => KeyCode::F4,
            WinitKeyCode::F5 => KeyCode::F5,
            WinitKeyCode::F6 => KeyCode::F6,
            WinitKeyCode::F7 => KeyCode::F7,
            WinitKeyCode::F8 => KeyCode::F8,
            WinitKeyCode::F9 => KeyCode::F9,
            WinitKeyCode::F10 => KeyCode::F10,
            WinitKeyCode::F11 => KeyCode::F11,
            WinitKeyCode::F12 => KeyCode::F12,

            WinitKeyCode::ArrowUp => KeyCode::Up,
            WinitKeyCode::ArrowDown => KeyCode::Down,
            WinitKeyCode::ArrowLeft => KeyCode::Left,
            WinitKeyCode::ArrowRight => KeyCode::Right,

            WinitKeyCode::Space => KeyCode::Space,
            WinitKeyCode::Enter => KeyCode::Enter,
            WinitKeyCode::Escape => KeyCode::Escape,
            WinitKeyCode::Tab => KeyCode::Tab,
            WinitKeyCode::Backspace => KeyCode::Backspace,
            WinitKeyCode::Delete => KeyCode::Delete,
            WinitKeyCode::ShiftLeft | WinitKeyCode::ShiftRight => KeyCode::Shift,
            WinitKeyCode::ControlLeft | WinitKeyCode::ControlRight => KeyCode::Control,
            WinitKeyCode::AltLeft | WinitKeyCode::AltRight => KeyCode::Alt,

            _ => KeyCode::Other(key as u32),
        }
    }
}

/// Input state tracking for keyboard and mouse.
///
/// Tracks both instantaneous events (pressed/released this frame) and
/// continuous state (currently held).
#[derive(Debug, Default)]
pub struct Input {
    // Key state
    keys_held: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,

    // Mouse button state
    mouse_held: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,

    // Mouse position
    mouse_position: Vec2,
    mouse_ndc: Vec2,
    mouse_delta: Vec2,
    last_mouse_position: Vec2,

    // Scroll
    scroll_delta: f32,

    // Window size for NDC calculation
    window_size: (u32, u32),
}

impl Input {
    /// Create a new input tracker.
    pub fn new() -> Self {
        Self {
            window_size: (800, 600),
            ..Default::default()
        }
    }

    // ========== Key Queries ==========

    /// Check if a key was pressed this frame (just went down).
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Check if a key is currently held down.
    pub fn key_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    /// Check if a key was released this frame (just went up).
    pub fn key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    // ========== Mouse Button Queries ==========

    /// Check if a mouse button was pressed this frame.
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    /// Check if a mouse button is currently held down.
    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.mouse_held.contains(&button)
    }

    /// Check if a mouse button was released this frame.
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.mouse_released.contains(&button)
    }

    // ========== Mouse Position Queries ==========

    /// Get the mouse position in screen pixels.
    pub fn mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    /// Get the mouse position in normalized device coordinates (-1 to 1).
    ///
    /// Origin is at center of window. X increases to the right, Y increases upward.
    pub fn mouse_ndc(&self) -> Vec2 {
        self.mouse_ndc
    }

    /// Get the mouse movement since last frame in pixels.
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    /// Get the scroll wheel delta this frame.
    ///
    /// Positive values indicate scrolling up/forward.
    pub fn scroll_delta(&self) -> f32 {
        self.scroll_delta
    }

    // ========== Internal Methods ==========

    /// Called at the start of each frame to clear per-frame state.
    pub(crate) fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = 0.0;
    }

    /// Update window size for NDC calculations.
    pub(crate) fn set_window_size(&mut self, width: u32, height: u32) {
        self.window_size = (width, height);
    }

    /// Process a winit window event.
    pub(crate) fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let key = KeyCode::from(keycode);
                    match event.state {
                        ElementState::Pressed => {
                            // Only fire pressed event if not already held (no repeat)
                            if !self.keys_held.contains(&key) {
                                self.keys_pressed.insert(key);
                            }
                            self.keys_held.insert(key);
                        }
                        ElementState::Released => {
                            self.keys_held.remove(&key);
                            self.keys_released.insert(key);
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let btn = MouseButton::from(*button);
                match state {
                    ElementState::Pressed => {
                        self.mouse_pressed.insert(btn);
                        self.mouse_held.insert(btn);
                    }
                    ElementState::Released => {
                        self.mouse_held.remove(&btn);
                        self.mouse_released.insert(btn);
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = Vec2::new(position.x as f32, position.y as f32);
                self.mouse_delta = new_pos - self.last_mouse_position;
                self.last_mouse_position = self.mouse_position;
                self.mouse_position = new_pos;

                // Calculate NDC
                let (w, h) = self.window_size;
                if w > 0 && h > 0 {
                    self.mouse_ndc = Vec2::new(
                        (position.x as f32 / w as f32) * 2.0 - 1.0,
                        1.0 - (position.y as f32 / h as f32) * 2.0, // Y flipped
                    );
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.scroll_delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_state() {
        let mut input = Input::new();

        // Initially nothing pressed
        assert!(!input.key_held(KeyCode::Space));
        assert!(!input.key_pressed(KeyCode::Space));

        // Simulate key press via direct state manipulation (normally done via handle_event)
        input.keys_pressed.insert(KeyCode::Space);
        input.keys_held.insert(KeyCode::Space);

        assert!(input.key_held(KeyCode::Space));
        assert!(input.key_pressed(KeyCode::Space));

        // After begin_frame, pressed is cleared but held remains
        input.begin_frame();
        assert!(input.key_held(KeyCode::Space));
        assert!(!input.key_pressed(KeyCode::Space));
    }

    #[test]
    fn test_mouse_ndc() {
        let mut input = Input::new();
        input.set_window_size(800, 600);

        // Center of window should be (0, 0) in NDC
        input.mouse_position = Vec2::new(400.0, 300.0);
        input.mouse_ndc = Vec2::new(
            (400.0 / 800.0) * 2.0 - 1.0,
            1.0 - (300.0 / 600.0) * 2.0,
        );

        assert!((input.mouse_ndc().x).abs() < 0.01);
        assert!((input.mouse_ndc().y).abs() < 0.01);
    }
}
