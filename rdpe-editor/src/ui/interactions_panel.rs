//! Rule matrix panel for the editor.
//!
//! Provides a visual grid for configuring type-based particle interactions.

use crate::config::{InteractionConfig, RuleMatrixCell, RuleMatrixPreset, RuleConfig};
use egui::{Color32, Ui, Vec2};

/// State for the selected cell editor popup.
#[derive(Default)]
pub struct InteractionsPanelState {
    /// Currently selected cell (from_type, to_type).
    pub selected_cell: Option<(usize, usize)>,
}

/// Render the interactions panel.
///
/// Returns true if any value changed (requires simulation rebuild).
pub fn render_interactions_panel(
    ui: &mut Ui,
    config: &mut InteractionConfig,
    state: &mut InteractionsPanelState,
) -> bool {
    let mut changed = false;

    // Enable toggle
    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut config.enabled, "Enable Rule Matrix")
            .on_hover_text("Type-based particle interaction rules")
            .changed()
        {
            changed = true;
        }
    });

    if !config.enabled {
        ui.label("Enable the rule matrix to configure per-type interactions.");
        return changed;
    }

    ui.separator();

    // Number of types (synced from Spawn tab)
    ui.horizontal(|ui| {
        ui.label(format!("Particle Types: {}", config.num_types));
        ui.weak("(set in Spawn tab)");
    });

    ui.separator();

    // Presets
    ui.horizontal_wrapped(|ui| {
        ui.label("Presets:");
        for preset in RuleMatrixPreset::all() {
            if ui.button(preset.name()).clicked() {
                preset.apply(config);
                changed = true;
            }
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Clear All").clicked() {
            config.clear();
            changed = true;
        }
    });

    ui.separator();

    // Rule matrix grid
    ui.heading("Rule Matrix");
    ui.label("Click a cell to edit rules. Row = responding type, Column = target type.");

    ui.add_space(8.0);

    // Calculate cell size based on available width
    let num_types = config.num_types;
    let available_width = ui.available_width() - 80.0;
    let cell_size = (available_width / (num_types as f32 + 1.0)).min(50.0).max(35.0);

    egui::ScrollArea::horizontal().show(ui, |ui| {
        egui::Grid::new("rule_matrix")
            .spacing(Vec2::new(2.0, 2.0))
            .show(ui, |ui| {
                // Header row with column labels
                ui.label(""); // Empty corner cell
                for j in 0..num_types {
                    let type_info = &config.type_info[j];
                    let color = Color32::from_rgb(
                        (type_info.color[0] * 255.0) as u8,
                        (type_info.color[1] * 255.0) as u8,
                        (type_info.color[2] * 255.0) as u8,
                    );
                    ui.colored_label(color, &type_info.name);
                }
                ui.end_row();

                // Data rows
                for i in 0..num_types {
                    // Row label
                    let type_info = &config.type_info[i];
                    let color = Color32::from_rgb(
                        (type_info.color[0] * 255.0) as u8,
                        (type_info.color[1] * 255.0) as u8,
                        (type_info.color[2] * 255.0) as u8,
                    );
                    ui.colored_label(color, &type_info.name);

                    // Cells
                    for j in 0..num_types {
                        let cell = config.get(i, j);
                        if render_matrix_cell(ui, i, j, cell, cell_size, state) {
                            // Cell was clicked - selection handled in render_matrix_cell
                        }
                    }
                    ui.end_row();
                }
            });
    });

    ui.separator();

    // Selected cell editor
    if let Some((from, to)) = state.selected_cell {
        let from_name = config.type_info.get(from).map(|t| t.name.as_str()).unwrap_or("?");
        let to_name = config.type_info.get(to).map(|t| t.name.as_str()).unwrap_or("?");

        ui.heading(format!("{} -> {}", from_name, to_name));
        ui.label("Rules for when this type encounters the target type:");

        ui.add_space(4.0);

        let cell = config.get_mut(from, to);

        // Show existing rules
        let mut to_remove = None;
        for (idx, rule) in cell.rules.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}.", idx + 1));

                // Rule name and parameters
                changed |= render_rule_inline(ui, rule);

                if ui.small_button("X").clicked() {
                    to_remove = Some(idx);
                    changed = true;
                }
            });
        }

        if let Some(idx) = to_remove {
            cell.rules.remove(idx);
        }

        ui.add_space(4.0);

        // Add rule buttons
        ui.horizontal_wrapped(|ui| {
            ui.label("Add:");
            if ui.button("Attract").clicked() {
                cell.rules.push(RuleConfig::Attract { radius: 0.2, strength: 0.5 });
                changed = true;
            }
            if ui.button("Separate").clicked() {
                cell.rules.push(RuleConfig::Separate { radius: 0.1, strength: 2.0 });
                changed = true;
            }
            if ui.button("Cohere").clicked() {
                cell.rules.push(RuleConfig::Cohere { radius: 0.2, strength: 0.5 });
                changed = true;
            }
            if ui.button("Align").clicked() {
                cell.rules.push(RuleConfig::Align { radius: 0.15, strength: 1.0 });
                changed = true;
            }
        });

        ui.add_space(8.0);

        // Custom WGSL code editor
        ui.collapsing("Custom WGSL Code", |ui| {
            ui.label("Available variables:");
            ui.horizontal_wrapped(|ui| {
                ui.code("p");
                ui.weak("(current particle),");
                ui.code("other");
                ui.weak("(neighbor),");
                ui.code("neighbor_dist");
                ui.weak("(distance),");
            });
            ui.horizontal_wrapped(|ui| {
                ui.code("neighbor_dir");
                ui.weak("(direction),");
                ui.code("neighbor_pos");
                ui.weak(",");
                ui.code("neighbor_vel");
                ui.weak(",");
                ui.code("uniforms.delta_time");
            });
            ui.add_space(4.0);

            let response = ui.add(
                egui::TextEdit::multiline(&mut cell.custom_code)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(4)
                    .hint_text("// Example: attract with custom falloff\nlet force = 1.0 / (neighbor_dist * neighbor_dist + 0.01);\np.velocity -= neighbor_dir * force * 0.1 * uniforms.delta_time;")
            );
            if response.changed() {
                changed = true;
            }
        });

        ui.add_space(4.0);

        if ui.button("Close").clicked() {
            state.selected_cell = None;
        }
    } else {
        ui.label("Click a cell in the matrix above to edit its rules.");
    }

    ui.separator();

    // Type editor
    ui.collapsing("Type Names & Colors", |ui| {
        changed |= render_type_editor(ui, config);
    });

    changed
}

/// Render a single cell in the rule matrix.
fn render_matrix_cell(
    ui: &mut Ui,
    from: usize,
    to: usize,
    cell: &RuleMatrixCell,
    size: f32,
    state: &mut InteractionsPanelState,
) -> bool {
    let is_selected = state.selected_cell == Some((from, to));

    // Color based on rules
    let bg_color = if cell.is_empty() {
        Color32::from_gray(40)
    } else if cell.has_custom_code() && cell.rules.is_empty() {
        // Custom code only - purple
        Color32::from_rgb(80, 40, 120)
    } else {
        // Color based on primary rule type
        let has_attract = cell.rules.iter().any(|r| matches!(r, RuleConfig::Attract { .. } | RuleConfig::Cohere { .. }));
        let has_repel = cell.rules.iter().any(|r| matches!(r, RuleConfig::Separate { .. }));
        let has_custom = cell.has_custom_code();

        if has_custom {
            // Has custom code + rules - purple tint
            Color32::from_rgb(90, 50, 110)
        } else if has_attract && has_repel {
            // Mixed - yellow/orange
            Color32::from_rgb(120, 100, 40)
        } else if has_attract {
            // Attraction - green
            let intensity = (cell.len() as f32 * 60.0).min(150.0) as u8;
            Color32::from_rgb(30, 50 + intensity, 30)
        } else if has_repel {
            // Repulsion - red
            let intensity = (cell.len() as f32 * 60.0).min(150.0) as u8;
            Color32::from_rgb(50 + intensity, 30, 30)
        } else {
            // Other rules - blue
            let intensity = (cell.len() as f32 * 60.0).min(150.0) as u8;
            Color32::from_rgb(30, 30, 50 + intensity)
        }
    };

    let stroke_color = if is_selected {
        Color32::WHITE
    } else {
        Color32::TRANSPARENT
    };

    let (response, painter) = ui.allocate_painter(Vec2::new(size, size), egui::Sense::click());
    let rect = response.rect;

    painter.rect_filled(rect, 4.0, bg_color);
    if is_selected {
        painter.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, stroke_color), egui::StrokeKind::Outside);
    }

    // Show rule count (with * if custom code present)
    let text = if cell.is_empty() {
        "-".to_string()
    } else if cell.has_custom_code() {
        if cell.rules.is_empty() {
            "*".to_string()  // Custom code only
        } else {
            format!("{}*", cell.len())  // Rules + custom
        }
    } else {
        cell.len().to_string()
    };

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::default(),
        Color32::WHITE,
    );

    if response.clicked() {
        if is_selected {
            state.selected_cell = None;
        } else {
            state.selected_cell = Some((from, to));
        }
        return true;
    }

    false
}

/// Render inline rule editor with parameters.
fn render_rule_inline(ui: &mut Ui, rule: &mut RuleConfig) -> bool {
    let mut changed = false;

    match rule {
        RuleConfig::Attract { radius, strength } => {
            ui.label("Attract");
            ui.label("r:");
            changed |= ui.add(egui::DragValue::new(radius).speed(0.01).range(0.01..=1.0)).changed();
            ui.label("s:");
            changed |= ui.add(egui::DragValue::new(strength).speed(0.05).range(-5.0..=5.0)).changed();
        }
        RuleConfig::Separate { radius, strength } => {
            ui.label("Separate");
            ui.label("r:");
            changed |= ui.add(egui::DragValue::new(radius).speed(0.01).range(0.01..=1.0)).changed();
            ui.label("s:");
            changed |= ui.add(egui::DragValue::new(strength).speed(0.05).range(0.0..=10.0)).changed();
        }
        RuleConfig::Cohere { radius, strength } => {
            ui.label("Cohere");
            ui.label("r:");
            changed |= ui.add(egui::DragValue::new(radius).speed(0.01).range(0.01..=1.0)).changed();
            ui.label("s:");
            changed |= ui.add(egui::DragValue::new(strength).speed(0.05).range(0.0..=5.0)).changed();
        }
        RuleConfig::Align { radius, strength } => {
            ui.label("Align");
            ui.label("r:");
            changed |= ui.add(egui::DragValue::new(radius).speed(0.01).range(0.01..=1.0)).changed();
            ui.label("s:");
            changed |= ui.add(egui::DragValue::new(strength).speed(0.05).range(0.0..=5.0)).changed();
        }
        _ => {
            ui.label(rule.name());
        }
    }

    changed
}

/// Render the type editor for renaming types and changing colors.
fn render_type_editor(ui: &mut Ui, config: &mut InteractionConfig) -> bool {
    let mut changed = false;

    for i in 0..config.num_types {
        ui.horizontal(|ui| {
            ui.label(format!("{}:", i));

            let type_info = &mut config.type_info[i];
            if ui.text_edit_singleline(&mut type_info.name).changed() {
                changed = true;
            }

            let mut color = type_info.color;
            if ui.color_edit_button_rgb(&mut color).changed() {
                type_info.color = color;
                changed = true;
            }
        });
    }

    changed
}
