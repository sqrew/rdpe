//! UI panel for post-processing effects

use crate::config::{PostProcessConfig, PostProcessPreset};
use eframe::egui;

pub fn render_post_process_panel(ui: &mut egui::Ui, config: &mut PostProcessConfig) -> bool {
    let mut changed = false;

    ui.heading("Post-Processing");

    ui.label(
        egui::RichText::new("Apply fullscreen effects after rendering")
            .small()
            .weak(),
    );

    // Enable toggle
    if ui
        .checkbox(&mut config.enabled, "Enable Post-Processing")
        .changed()
    {
        changed = true;
    }

    if !config.enabled {
        ui.label(egui::RichText::new("Post-processing is disabled").weak());
        return changed;
    }

    ui.separator();

    // Preset selection
    let variants = PostProcessPreset::variants();
    let mut preset_idx = match config.preset {
        PostProcessPreset::None => 0,
        PostProcessPreset::Bloom => 1,
        PostProcessPreset::ChromaticAberration => 2,
        PostProcessPreset::Vignette => 3,
        PostProcessPreset::CRT => 4,
        PostProcessPreset::Custom => 5,
    };

    ui.horizontal(|ui| {
        ui.label("Preset:");
        if egui::ComboBox::from_id_salt("pp_preset")
            .selected_text(variants[preset_idx])
            .show_index(ui, &mut preset_idx, variants.len(), |i| variants[i])
            .changed()
        {
            let new_preset = match preset_idx {
                0 => PostProcessPreset::None,
                1 => PostProcessPreset::Bloom,
                2 => PostProcessPreset::ChromaticAberration,
                3 => PostProcessPreset::Vignette,
                4 => PostProcessPreset::CRT,
                5 => PostProcessPreset::Custom,
                _ => PostProcessPreset::None,
            };
            // When changing preset, clear custom shader (use preset default)
            if config.preset != new_preset {
                config.preset = new_preset;
                config.custom_shader.clear();
                changed = true;
            }
        }
    });

    // Show preset description
    let description = match config.preset {
        PostProcessPreset::None => "No effect - passthrough",
        PostProcessPreset::Bloom => "Bright areas glow and bleed light",
        PostProcessPreset::ChromaticAberration => "RGB color fringing at edges",
        PostProcessPreset::Vignette => "Darkened edges around screen",
        PostProcessPreset::CRT => "Retro CRT monitor with scanlines",
        PostProcessPreset::Custom => "Write your own shader code",
    };
    ui.label(egui::RichText::new(description).small().weak());

    ui.add_space(8.0);

    // Intensity slider (only for presets that use it)
    if config.preset != PostProcessPreset::None {
        ui.separator();

        changed |= ui
            .add(egui::Slider::new(&mut config.intensity, 0.0..=2.0).text("Intensity"))
            .on_hover_text("Effect strength (0 = off, 1 = normal, 2 = strong)")
            .changed();
    }

    // Custom shader editor
    if config.preset == PostProcessPreset::Custom {
        ui.separator();
        ui.heading("Custom Shader");

        ui.label(
            egui::RichText::new(
                "Write WGSL code to process pixels. Available variables:\n\
                 - uv: vec2<f32> (0-1 screen coords)\n\
                 - input_texture, input_sampler (scene texture)\n\
                 - pp_time, pp_intensity, pp_resolution",
            )
            .small()
            .weak(),
        );

        ui.add_space(4.0);

        // If custom shader is empty, show the default template
        if config.custom_shader.is_empty() {
            config.custom_shader = config.preset.default_shader().to_string();
        }

        // Code editor
        let editor = egui::TextEdit::multiline(&mut config.custom_shader)
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(15);

        if ui.add(editor).changed() {
            changed = true;
        }

        ui.add_space(4.0);

        // Reset to default button
        if ui.button("Reset to Default").clicked() {
            config.custom_shader = PostProcessPreset::Custom.default_shader().to_string();
            changed = true;
        }
    }

    // Info about rebuild requirement
    if changed {
        ui.separator();
        ui.label(
            egui::RichText::new("Changes require rebuild (Ctrl+R)")
                .small()
                .color(egui::Color32::YELLOW),
        );
    }

    changed
}
