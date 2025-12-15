//! UI modules for the editor

mod custom_panel;
mod effects_panel;
mod emitters_panel;
mod export_panel;
mod fields_panel;
mod interactions_panel;
mod mouse_panel;
mod particle_fields_panel;
mod post_process_panel;
mod rules_panel;
mod spawn_panel;
mod visuals_panel;
mod volume_panel;
mod presets;

pub use custom_panel::{render_custom_panel, AddUniformState};
pub use effects_panel::render_effects_panel;
pub use emitters_panel::render_emitters_panel;
pub use export_panel::{render_export_window, render_export_button, ExportPanelState};
pub use fields_panel::render_fields_panel;
pub use interactions_panel::{render_interactions_panel, InteractionsPanelState};
pub use mouse_panel::render_mouse_panel;
pub use particle_fields_panel::render_particle_fields_panel;
pub use post_process_panel::render_post_process_panel;
pub use rules_panel::{render_rules_panel, ShaderContext};
pub use spawn_panel::render_spawn_panel;
pub use visuals_panel::render_visuals_panel;
pub use volume_panel::render_volume_panel;
pub use presets::{PRESETS, Preset};
