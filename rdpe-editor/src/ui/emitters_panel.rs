//! UI panel for particle emitters

use crate::config::EmitterConfig;
use eframe::egui;

/// Render the emitters panel.
///
/// Returns true if any changes require a shader rebuild.
pub fn render_emitters_panel(ui: &mut egui::Ui, emitters: &mut Vec<EmitterConfig>) -> bool {
    let mut changed = false;

    ui.heading("Emitters");

    ui.label(
        egui::RichText::new("Emitters continuously spawn particles into dead slots")
            .small()
            .weak(),
    );

    ui.add_space(8.0);

    // Add emitter button
    ui.horizontal(|ui| {
        ui.label("Add Emitter:");
        let variants = EmitterConfig::type_variants();
        for variant in variants {
            if ui.button(*variant).clicked() {
                emitters.push(EmitterConfig::default_of_type(variant));
                changed = true;
            }
        }
    });

    if emitters.is_empty() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("No emitters configured. Add one above to enable runtime spawning.")
                .weak()
                .italics(),
        );
        return changed;
    }

    ui.separator();

    // List of emitters
    let mut to_remove = None;

    for (i, emitter) in emitters.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            egui::CollapsingHeader::new(format!("{} - {}", i + 1, emitter.type_name()))
                .default_open(true)
                .show(ui, |ui| {
                    // Remove button
                    ui.horizontal(|ui| {
                        if ui.small_button("Remove").clicked() {
                            to_remove = Some(i);
                        }
                        ui.label(
                            egui::RichText::new(format!("Rate: {:.0}/s", emitter.rate()))
                                .small()
                                .weak(),
                        );
                    });

                    ui.separator();

                    // Emitter-specific controls
                    changed |= render_emitter_controls(ui, emitter);
                });
        });

        ui.add_space(4.0);
    }

    // Remove emitter if requested
    if let Some(idx) = to_remove {
        emitters.remove(idx);
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

/// Render controls for a specific emitter type.
fn render_emitter_controls(ui: &mut egui::Ui, emitter: &mut EmitterConfig) -> bool {
    let mut changed = false;

    match emitter {
        EmitterConfig::Point { position, rate, speed } => {
            ui.label("Position:");
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut position[0]).speed(0.01)).changed();
                ui.label("Y:");
                changed |= ui.add(egui::DragValue::new(&mut position[1]).speed(0.01)).changed();
                ui.label("Z:");
                changed |= ui.add(egui::DragValue::new(&mut position[2]).speed(0.01)).changed();
            });

            changed |= ui
                .add(egui::Slider::new(rate, 1.0..=5000.0).text("Rate (particles/s)").logarithmic(true))
                .changed();

            changed |= ui
                .add(egui::Slider::new(speed, 0.0..=5.0).text("Speed"))
                .on_hover_text("Initial particle speed (0 = random)")
                .changed();
        }

        EmitterConfig::Burst { position, count, speed } => {
            ui.label(
                egui::RichText::new("One-time burst at simulation start")
                    .small()
                    .weak(),
            );

            ui.label("Position:");
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut position[0]).speed(0.01)).changed();
                ui.label("Y:");
                changed |= ui.add(egui::DragValue::new(&mut position[1]).speed(0.01)).changed();
                ui.label("Z:");
                changed |= ui.add(egui::DragValue::new(&mut position[2]).speed(0.01)).changed();
            });

            let mut count_i32 = *count as i32;
            if ui
                .add(egui::Slider::new(&mut count_i32, 1..=10000).text("Count").logarithmic(true))
                .changed()
            {
                *count = count_i32 as u32;
                changed = true;
            }

            changed |= ui
                .add(egui::Slider::new(speed, 0.1..=10.0).text("Speed"))
                .changed();
        }

        EmitterConfig::Cone { position, direction, speed, spread, rate } => {
            ui.label("Position:");
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut position[0]).speed(0.01)).changed();
                ui.label("Y:");
                changed |= ui.add(egui::DragValue::new(&mut position[1]).speed(0.01)).changed();
                ui.label("Z:");
                changed |= ui.add(egui::DragValue::new(&mut position[2]).speed(0.01)).changed();
            });

            ui.label("Direction:");
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut direction[0]).speed(0.01)).changed();
                ui.label("Y:");
                changed |= ui.add(egui::DragValue::new(&mut direction[1]).speed(0.01)).changed();
                ui.label("Z:");
                changed |= ui.add(egui::DragValue::new(&mut direction[2]).speed(0.01)).changed();
            });

            // Quick direction presets
            ui.horizontal(|ui| {
                ui.label("Preset:");
                if ui.small_button("Up").clicked() {
                    *direction = [0.0, 1.0, 0.0];
                    changed = true;
                }
                if ui.small_button("Down").clicked() {
                    *direction = [0.0, -1.0, 0.0];
                    changed = true;
                }
                if ui.small_button("Forward").clicked() {
                    *direction = [0.0, 0.0, 1.0];
                    changed = true;
                }
            });

            changed |= ui
                .add(egui::Slider::new(rate, 1.0..=5000.0).text("Rate (particles/s)").logarithmic(true))
                .changed();

            changed |= ui
                .add(egui::Slider::new(speed, 0.1..=10.0).text("Speed"))
                .changed();

            // Spread in degrees for easier understanding
            let mut spread_deg = spread.to_degrees();
            if ui
                .add(egui::Slider::new(&mut spread_deg, 0.0..=180.0).text("Spread (degrees)"))
                .on_hover_text("Cone half-angle: 0 = laser, 90 = hemisphere, 180 = full sphere")
                .changed()
            {
                *spread = spread_deg.to_radians();
                changed = true;
            }
        }

        EmitterConfig::Sphere { center, radius, speed, rate } => {
            ui.label("Center:");
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut center[0]).speed(0.01)).changed();
                ui.label("Y:");
                changed |= ui.add(egui::DragValue::new(&mut center[1]).speed(0.01)).changed();
                ui.label("Z:");
                changed |= ui.add(egui::DragValue::new(&mut center[2]).speed(0.01)).changed();
            });

            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=2.0).text("Radius"))
                .changed();

            changed |= ui
                .add(egui::Slider::new(rate, 1.0..=5000.0).text("Rate (particles/s)").logarithmic(true))
                .changed();

            changed |= ui
                .add(egui::Slider::new(speed, -5.0..=5.0).text("Speed"))
                .on_hover_text("Outward speed (negative = inward)")
                .changed();
        }

        EmitterConfig::Box { min, max, velocity, rate } => {
            ui.label("Min Corner:");
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut min[0]).speed(0.01)).changed();
                ui.label("Y:");
                changed |= ui.add(egui::DragValue::new(&mut min[1]).speed(0.01)).changed();
                ui.label("Z:");
                changed |= ui.add(egui::DragValue::new(&mut min[2]).speed(0.01)).changed();
            });

            ui.label("Max Corner:");
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut max[0]).speed(0.01)).changed();
                ui.label("Y:");
                changed |= ui.add(egui::DragValue::new(&mut max[1]).speed(0.01)).changed();
                ui.label("Z:");
                changed |= ui.add(egui::DragValue::new(&mut max[2]).speed(0.01)).changed();
            });

            ui.label("Initial Velocity:");
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut velocity[0]).speed(0.01)).changed();
                ui.label("Y:");
                changed |= ui.add(egui::DragValue::new(&mut velocity[1]).speed(0.01)).changed();
                ui.label("Z:");
                changed |= ui.add(egui::DragValue::new(&mut velocity[2]).speed(0.01)).changed();
            });

            changed |= ui
                .add(egui::Slider::new(rate, 1.0..=5000.0).text("Rate (particles/s)").logarithmic(true))
                .changed();
        }
    }

    changed
}
