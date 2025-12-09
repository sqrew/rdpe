//! UI modules for the editor

mod custom_panel;
mod effects_panel;
mod export_panel;
mod fields_panel;
mod particle_fields_panel;
mod rules_panel;
mod spawn_panel;
mod visuals_panel;
mod volume_panel;
mod presets;

pub use custom_panel::{render_custom_panel, AddUniformState};
pub use effects_panel::render_effects_panel;
pub use export_panel::{render_export_window, render_export_button, ExportPanelState};
pub use fields_panel::render_fields_panel;
pub use particle_fields_panel::render_particle_fields_panel;
pub use rules_panel::render_rules_panel;
pub use spawn_panel::render_spawn_panel;
pub use visuals_panel::render_visuals_panel;
pub use volume_panel::render_volume_panel;
pub use presets::{PRESETS, Preset};
