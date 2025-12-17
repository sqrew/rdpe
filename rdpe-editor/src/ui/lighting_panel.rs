//! UI panel for lighting configuration

use crate::config::{LightConfig, LightType, LightingConfig, MAX_LIGHTS};
use eframe::egui;

/// Render the lighting panel.
///
/// Returns true if any changes require a shader rebuild.
pub fn render_lighting_panel(ui: &mut egui::Ui, lighting: &mut LightingConfig) -> bool {
    let mut changed = false;

    ui.heading("Lighting");

    ui.label(
        egui::RichText::new("Configure lights for sphere impostor shading")
            .small()
            .weak(),
    );

    ui.add_space(8.0);

    // Enable toggle
    if ui
        .checkbox(&mut lighting.enabled, "Enable Lighting")
        .changed()
    {
        changed = true;
    }

    if !lighting.enabled {
        ui.label(egui::RichText::new("Lighting is disabled").weak());
        return changed;
    }

    ui.separator();

    // Ambient light section
    ui.label(egui::RichText::new("Ambient Light").strong());

    ui.horizontal(|ui| {
        ui.label("Color:");
        let mut color = egui::Color32::from_rgb(
            (lighting.ambient_color[0] * 255.0) as u8,
            (lighting.ambient_color[1] * 255.0) as u8,
            (lighting.ambient_color[2] * 255.0) as u8,
        );
        if ui.color_edit_button_srgba(&mut color).changed() {
            lighting.ambient_color = [
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
            ];
            changed = true;
        }
    });

    changed |= ui
        .add(
            egui::Slider::new(&mut lighting.ambient_intensity, 0.0..=1.0)
                .text("Intensity")
                .fixed_decimals(2),
        )
        .changed();

    ui.separator();

    // Add light buttons
    ui.horizontal(|ui| {
        ui.label("Add Light:");
        let can_add = lighting.lights.len() < MAX_LIGHTS;
        ui.add_enabled_ui(can_add, |ui| {
            if ui.button("Point").clicked() {
                lighting.add_point_light();
                changed = true;
            }
            if ui.button("Spot").clicked() {
                lighting.add_spot_light();
                changed = true;
            }
        });
        if !can_add {
            ui.label(
                egui::RichText::new(format!("(max {})", MAX_LIGHTS))
                    .small()
                    .weak(),
            );
        }
    });

    if lighting.lights.is_empty() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("No lights configured. Add one above.")
                .weak()
                .italics(),
        );
        return changed;
    }

    ui.separator();

    // List of lights
    let mut to_remove = None;

    for (i, light) in lighting.lights.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            let header_text = format!("{} - {} ({})", i + 1, light.name, light.light_type.name());
            egui::CollapsingHeader::new(header_text)
                .default_open(true)
                .show(ui, |ui| {
                    changed |= render_light_controls(ui, light, &mut to_remove, i);
                });
        });

        ui.add_space(4.0);
    }

    // Remove light if requested
    if let Some(idx) = to_remove {
        lighting.remove_light(idx);
        changed = true;
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

/// Render controls for a single light.
fn render_light_controls(
    ui: &mut egui::Ui,
    light: &mut LightConfig,
    to_remove: &mut Option<usize>,
    index: usize,
) -> bool {
    let mut changed = false;

    // Header row with enabled toggle and remove button
    ui.horizontal(|ui| {
        if ui.checkbox(&mut light.enabled, "Enabled").changed() {
            changed = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Remove").clicked() {
                *to_remove = Some(index);
            }
        });
    });

    ui.separator();

    // Name
    ui.horizontal(|ui| {
        ui.label("Name:");
        if ui.text_edit_singleline(&mut light.name).changed() {
            changed = true;
        }
    });

    // Light type selector
    ui.horizontal(|ui| {
        ui.label("Type:");
        let is_spot = light.light_type.is_spot();
        let mut type_idx = if is_spot { 1 } else { 0 };
        if egui::ComboBox::from_id_salt(format!("light_type_{}", index))
            .selected_text(light.light_type.name())
            .show_index(ui, &mut type_idx, 2, |i| {
                match i {
                    0 => "Point",
                    1 => "Spot",
                    _ => "Unknown",
                }
            })
            .changed()
        {
            light.light_type = match type_idx {
                0 => LightType::Point,
                1 => LightType::Spot {
                    direction: [0.0, -1.0, 0.0],
                    angle: std::f32::consts::FRAC_PI_4,
                },
                _ => LightType::Point,
            };
            changed = true;
        }
    });

    ui.add_space(4.0);

    // Position
    ui.label("Position:");
    ui.horizontal(|ui| {
        ui.label("X:");
        changed |= ui
            .add(egui::DragValue::new(&mut light.position[0]).speed(0.05))
            .changed();
        ui.label("Y:");
        changed |= ui
            .add(egui::DragValue::new(&mut light.position[1]).speed(0.05))
            .changed();
        ui.label("Z:");
        changed |= ui
            .add(egui::DragValue::new(&mut light.position[2]).speed(0.05))
            .changed();
    });

    // Position presets
    ui.horizontal(|ui| {
        ui.label("Preset:");
        if ui.small_button("Top").clicked() {
            light.position = [0.0, 3.0, 0.0];
            changed = true;
        }
        if ui.small_button("Front").clicked() {
            light.position = [0.0, 0.0, 3.0];
            changed = true;
        }
        if ui.small_button("Side").clicked() {
            light.position = [3.0, 1.0, 0.0];
            changed = true;
        }
        if ui.small_button("Corner").clicked() {
            light.position = [2.0, 2.0, 2.0];
            changed = true;
        }
    });

    ui.add_space(4.0);

    // Spot light specific controls
    if let LightType::Spot { direction, angle } = &mut light.light_type {
        ui.label("Direction:");
        ui.horizontal(|ui| {
            ui.label("X:");
            changed |= ui
                .add(egui::DragValue::new(&mut direction[0]).speed(0.05))
                .changed();
            ui.label("Y:");
            changed |= ui
                .add(egui::DragValue::new(&mut direction[1]).speed(0.05))
                .changed();
            ui.label("Z:");
            changed |= ui
                .add(egui::DragValue::new(&mut direction[2]).speed(0.05))
                .changed();
        });

        // Direction presets
        ui.horizontal(|ui| {
            ui.label("Preset:");
            if ui.small_button("Down").clicked() {
                *direction = [0.0, -1.0, 0.0];
                changed = true;
            }
            if ui.small_button("Forward").clicked() {
                *direction = [0.0, 0.0, -1.0];
                changed = true;
            }
            if ui.small_button("To Origin").clicked() {
                // Point toward origin from current position
                let len = (light.position[0].powi(2)
                    + light.position[1].powi(2)
                    + light.position[2].powi(2))
                .sqrt();
                if len > 0.001 {
                    *direction = [
                        -light.position[0] / len,
                        -light.position[1] / len,
                        -light.position[2] / len,
                    ];
                }
                changed = true;
            }
        });

        // Cone angle in degrees
        let mut angle_deg = angle.to_degrees();
        if ui
            .add(
                egui::Slider::new(&mut angle_deg, 5.0..=90.0)
                    .text("Cone Angle")
                    .suffix("°"),
            )
            .on_hover_text("Half-angle of the spotlight cone")
            .changed()
        {
            *angle = angle_deg.to_radians();
            changed = true;
        }

        ui.add_space(4.0);
    }

    // Color
    ui.horizontal(|ui| {
        ui.label("Color:");
        let mut color = egui::Color32::from_rgb(
            (light.color[0] * 255.0) as u8,
            (light.color[1] * 255.0) as u8,
            (light.color[2] * 255.0) as u8,
        );
        if ui.color_edit_button_srgba(&mut color).changed() {
            light.color = [
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
            ];
            changed = true;
        }

        // Quick color presets
        if ui.small_button("White").clicked() {
            light.color = [1.0, 1.0, 1.0];
            changed = true;
        }
        if ui.small_button("Warm").clicked() {
            light.color = [1.0, 0.9, 0.7];
            changed = true;
        }
        if ui.small_button("Cool").clicked() {
            light.color = [0.7, 0.85, 1.0];
            changed = true;
        }
    });

    // Intensity
    changed |= ui
        .add(
            egui::Slider::new(&mut light.intensity, 0.0..=5.0)
                .text("Intensity")
                .logarithmic(true),
        )
        .changed();

    // Falloff
    changed |= ui
        .add(
            egui::Slider::new(&mut light.falloff, 0.0..=2.0)
                .text("Falloff")
                .fixed_decimals(2),
        )
        .on_hover_text("Distance attenuation factor (higher = faster falloff)")
        .changed();

    changed
}
