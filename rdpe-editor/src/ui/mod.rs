//! UI modules for the editor

mod effects_panel;
mod rules_panel;
mod spawn_panel;
mod visuals_panel;
mod presets;

pub use effects_panel::render_effects_panel;
pub use rules_panel::render_rules_panel;
pub use spawn_panel::render_spawn_panel;
pub use visuals_panel::render_visuals_panel;
pub use presets::{PRESETS, Preset};
