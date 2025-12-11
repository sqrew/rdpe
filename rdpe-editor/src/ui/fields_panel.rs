//! Fields configuration panel

use crate::config::{FieldConfigEntry, FieldTypeConfig};
use egui::Ui;

pub fn render_fields_panel(ui: &mut Ui, fields: &mut Vec<FieldConfigEntry>) -> bool {
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
