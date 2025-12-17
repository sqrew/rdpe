//! Fields configuration panel

use crate::config::{FieldConfigEntry, FieldTypeConfig, GlyphsConfig, GlyphModeConfig, GlyphColorModeConfig};
use egui::Ui;

pub fn render_fields_panel(ui: &mut Ui, fields: &mut Vec<FieldConfigEntry>, glyphs: &mut GlyphsConfig) -> bool {
    let mut changed = false;

    ui.heading("3D Fields");

    ui.label(
        egui::RichText::new("Fields let particles read/write to 3D volumetric data")
            .small()
            .weak(),
    );

    // Add field button
    if ui.button("+ Add Field").clicked() {
        let name = format!("field_{}", fields.len());
        fields.push(FieldConfigEntry {
            name,
            ..Default::default()
        });
        changed = true;
    }

    ui.separator();

    // List existing fields
    let mut remove_idx = None;
    for (idx, field) in fields.iter_mut().enumerate() {
        let id = ui.make_persistent_id(format!("field_{}", idx));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("#{}", idx));
                    ui.add(egui::TextEdit::singleline(&mut field.name).desired_width(120.0));
                    if ui.small_button("X").clicked() {
                        remove_idx = Some(idx);
                    }
                });
            })
            .body(|ui| {
                changed |= render_field_editor(ui, field);
            });
    }

    if let Some(idx) = remove_idx {
        fields.remove(idx);
        changed = true;
    }

    ui.add_space(8.0);
    ui.separator();

    // Vector Glyphs section
    ui.heading("Vector Glyphs");
    ui.label(
        egui::RichText::new("Visualize vector fields with arrow glyphs")
            .small()
            .weak(),
    );

    egui::ComboBox::from_label("Glyph Mode")
        .selected_text(glyphs.mode.name())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut glyphs.mode, GlyphModeConfig::None, "None");
            ui.selectable_value(&mut glyphs.mode, GlyphModeConfig::ParticleVelocity, "Particle Velocity");
            if !fields.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new("Vector Fields").small().weak());
                for (i, field) in fields.iter().enumerate() {
                    if ui.selectable_label(
                        matches!(glyphs.mode, GlyphModeConfig::VectorField { field_index } if field_index == i),
                        format!("{} ({})", field.name, i)
                    ).clicked() {
                        glyphs.mode = GlyphModeConfig::VectorField { field_index: i };
                    }
                }
            }
        });

    if glyphs.mode != GlyphModeConfig::None {
        ui.add(egui::Slider::new(&mut glyphs.grid_resolution, 2..=16).text("Grid Resolution"));
        ui.add(egui::Slider::new(&mut glyphs.scale, 0.01..=0.5).text("Scale"));

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("glyph_color_mode")
                .selected_text(glyphs.color_mode.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut glyphs.color_mode, GlyphColorModeConfig::Uniform, "Uniform");
                    ui.selectable_value(&mut glyphs.color_mode, GlyphColorModeConfig::ByMagnitude, "By Magnitude");
                    ui.selectable_value(&mut glyphs.color_mode, GlyphColorModeConfig::ByDirection, "By Direction");
                });
            ui.label("Color Mode");
        });

        if glyphs.color_mode == GlyphColorModeConfig::Uniform {
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_rgb(&mut glyphs.color);
            });
        }
    }

    changed
}

fn render_field_editor(ui: &mut Ui, field: &mut FieldConfigEntry) -> bool {
    let mut changed = false;

    // Field type
    let type_variants = FieldTypeConfig::variants();
    let mut type_idx = match field.field_type {
        FieldTypeConfig::Scalar => 0,
        FieldTypeConfig::Vector => 1,
    };

    ui.horizontal(|ui| {
        ui.label("Type:");
        if egui::ComboBox::from_id_salt("field_type")
            .selected_text(type_variants[type_idx])
            .show_index(ui, &mut type_idx, type_variants.len(), |i| type_variants[i])
            .changed()
        {
            field.field_type = match type_idx {
                0 => FieldTypeConfig::Scalar,
                1 => FieldTypeConfig::Vector,
                _ => FieldTypeConfig::Scalar,
            };
            changed = true;
        }
    });

    // Resolution (power of 2)
    const VALID_RESOLUTIONS: &[u32] = &[8, 16, 32, 64, 128, 256];
    let mut res_idx = VALID_RESOLUTIONS
        .iter()
        .position(|&r| r == field.resolution)
        .unwrap_or(2); // Default to 64

    ui.horizontal(|ui| {
        ui.label("Resolution:");
        if egui::ComboBox::from_id_salt("field_resolution")
            .selected_text(format!("{}^3", VALID_RESOLUTIONS[res_idx]))
            .show_index(ui, &mut res_idx, VALID_RESOLUTIONS.len(), |i| {
                format!("{}^3", VALID_RESOLUTIONS[i])
            })
            .changed()
        {
            field.resolution = VALID_RESOLUTIONS[res_idx];
            changed = true;
        }

        let total_cells = field.resolution.pow(3);
        let components = if matches!(field.field_type, FieldTypeConfig::Vector) { 4 } else { 1 };
        let memory_kb = (total_cells as u64 * components * 4) / 1024;
        ui.label(egui::RichText::new(format!("(~{} KB)", memory_kb)).small().weak());
    });

    // Extent
    changed |= ui
        .add(
            egui::Slider::new(&mut field.extent, 0.1..=5.0)
                .text("World Extent")
                .logarithmic(true),
        )
        .on_hover_text("The field covers [-extent, extent] in world space")
        .changed();

    // Decay
    changed |= ui
        .add(
            egui::Slider::new(&mut field.decay, 0.0..=1.0)
                .text("Decay")
                .fixed_decimals(2),
        )
        .on_hover_text("Per-frame decay multiplier (1.0 = no decay)")
        .changed();

    // Blur
    changed |= ui
        .add(
            egui::Slider::new(&mut field.blur, 0.0..=1.0)
                .text("Blur")
                .fixed_decimals(2),
        )
        .on_hover_text("Per-frame diffusion strength")
        .changed();

    // Blur iterations
    changed |= ui
        .add(
            egui::Slider::new(&mut field.blur_iterations, 0..=10)
                .text("Blur Iterations"),
        )
        .on_hover_text("Number of blur passes per frame")
        .changed();

    ui.separator();

    // Custom update shader section
    let has_custom = field.custom_update.is_some();
    let mut use_custom = has_custom;
    if ui.checkbox(&mut use_custom, "Custom Update Shader")
        .on_hover_text("Replace blur/decay with custom WGSL code")
        .changed()
    {
        if use_custom && !has_custom {
            // Initialize with example code
            let value_type = if matches!(field.field_type, FieldTypeConfig::Vector) { "vec3<f32>" } else { "f32" };
            let example = format!(
                r#"// Custom field update shader
// Available variables:
//   value: {} - current cell value
//   pos: vec3<u32> - cell position (0 to resolution-1)
//   world_pos: vec3<f32> - world position
//   uniforms.time: f32 - total time
//   uniforms.delta_time: f32 - frame delta
//   params.resolution: u32 - grid resolution
//   params.decay: f32 - configured decay
//   params.blur: f32 - configured blur
//
// Helper functions:
//   read_neighbor(dx, dy, dz) - read neighboring cell
//
// Set new_value to the result:
new_value = value * params.decay;"#,
                value_type
            );
            field.custom_update = Some(example);
            changed = true;
        } else if !use_custom && has_custom {
            field.custom_update = None;
            changed = true;
        }
    }

    if let Some(ref mut code) = field.custom_update {
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                if ui.add(
                    egui::TextEdit::multiline(code)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(10)
                ).changed() {
                    changed = true;
                }
            });
        ui.add_space(4.0);
    }

    // Show usage hint
    ui.separator();
    ui.label(egui::RichText::new("Usage in custom shader:").small().weak());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!(
            "field_write({}, pos, value); // write to field",
            idx_from_name(&field.name)
        )).monospace().small());
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!(
            "let val = field_read({}, pos); // read from field",
            idx_from_name(&field.name)
        )).monospace().small());
    });

    changed
}

fn idx_from_name(_name: &str) -> &str {
    // Just use 0 as placeholder since actual index depends on registry order
    "0u"
}
