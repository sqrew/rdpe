//! UI panel for vertex effects

use eframe::egui;
use crate::config::VertexEffectConfig;

/// Effect template for creating new effects
struct EffectTemplate {
    name: &'static str,
    create: fn() -> VertexEffectConfig,
}

const EFFECT_TEMPLATES: &[EffectTemplate] = &[
    EffectTemplate { name: "Rotate", create: || VertexEffectConfig::Rotate { speed: 3.0 } },
    EffectTemplate { name: "Wobble", create: || VertexEffectConfig::Wobble { frequency: 3.0, amplitude: 0.3 } },
    EffectTemplate { name: "Pulse", create: || VertexEffectConfig::Pulse { frequency: 2.0, amplitude: 0.5 } },
    EffectTemplate { name: "Wave", create: || VertexEffectConfig::Wave { direction: [0.0, 1.0, 0.0], frequency: 2.0, speed: 2.0, amplitude: 0.2 } },
    EffectTemplate { name: "Jitter", create: || VertexEffectConfig::Jitter { amplitude: 0.1 } },
    EffectTemplate { name: "Stretch To Velocity", create: || VertexEffectConfig::StretchToVelocity { max_stretch: 3.0 } },
    EffectTemplate { name: "Scale By Distance", create: || VertexEffectConfig::ScaleByDistance { center: [0.0, 0.0, 0.0], min_scale: 0.3, max_scale: 2.5, max_distance: 1.0 } },
    EffectTemplate { name: "Fade By Distance", create: || VertexEffectConfig::FadeByDistance { near: 0.3, far: 1.5 } },
    EffectTemplate { name: "Billboard Cylindrical", create: || VertexEffectConfig::BillboardCylindrical { axis: [0.0, 1.0, 0.0] } },
    EffectTemplate { name: "Billboard Fixed", create: || VertexEffectConfig::BillboardFixed { forward: [0.0, 0.0, 1.0], up: [0.0, 1.0, 0.0] } },
    EffectTemplate { name: "Face Point", create: || VertexEffectConfig::FacePoint { target: [0.0, 0.0, 0.0] } },
];

pub fn render_effects_panel(ui: &mut egui::Ui, effects: &mut Vec<VertexEffectConfig>) {
    ui.heading("Vertex Effects");

    // Add effect dropdown
    ui.horizontal(|ui| {
        ui.label("Add Effect:");
        egui::ComboBox::from_id_salt("add_effect")
            .selected_text("Select...")
            .show_ui(ui, |ui| {
                for template in EFFECT_TEMPLATES {
                    if ui.selectable_label(false, template.name).clicked() {
                        effects.push((template.create)());
                    }
                }
            });
    });

    ui.add_space(4.0);

    // List existing effects
    let mut to_remove = None;
    let mut to_move_up = None;
    let mut to_move_down = None;
    let effect_count = effects.len();

    for (idx, effect) in effects.iter_mut().enumerate() {
        ui.push_id(idx, |ui| {
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .inner_margin(6.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Effect name
                        ui.strong(effect.name());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Remove button
                            if ui.small_button("X").clicked() {
                                to_remove = Some(idx);
                            }

                            // Move buttons
                            if idx > 0 && ui.small_button("^").clicked() {
                                to_move_up = Some(idx);
                            }
                            if idx < effect_count - 1 && ui.small_button("v").clicked() {
                                to_move_down = Some(idx);
                            }
                        });
                    });

                    // Effect parameters
                    render_effect_params(ui, effect);
                });
        });
        ui.add_space(2.0);
    }

    // Handle removals and moves
    if let Some(idx) = to_remove {
        effects.remove(idx);
    }
    if let Some(idx) = to_move_up {
        effects.swap(idx, idx - 1);
    }
    if let Some(idx) = to_move_down {
        effects.swap(idx, idx + 1);
    }
}

fn render_effect_params(ui: &mut egui::Ui, effect: &mut VertexEffectConfig) {
    match effect {
        VertexEffectConfig::Rotate { speed } => {
            ui.add(egui::Slider::new(speed, -10.0..=10.0).text("Speed"));
        }
        VertexEffectConfig::Wobble { frequency, amplitude } => {
            ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"));
            ui.add(egui::Slider::new(amplitude, 0.0..=1.0).text("Amplitude"));
        }
        VertexEffectConfig::Pulse { frequency, amplitude } => {
            ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"));
            ui.add(egui::Slider::new(amplitude, 0.0..=1.0).text("Amplitude"));
        }
        VertexEffectConfig::Wave { direction, frequency, speed, amplitude } => {
            ui.horizontal(|ui| {
                ui.label("Direction:");
                ui.add(egui::DragValue::new(&mut direction[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut direction[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut direction[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"));
            ui.add(egui::Slider::new(speed, 0.1..=10.0).text("Speed"));
            ui.add(egui::Slider::new(amplitude, 0.0..=1.0).text("Amplitude"));
        }
        VertexEffectConfig::Jitter { amplitude } => {
            ui.add(egui::Slider::new(amplitude, 0.0..=0.5).text("Amplitude"));
        }
        VertexEffectConfig::StretchToVelocity { max_stretch } => {
            ui.add(egui::Slider::new(max_stretch, 1.0..=5.0).text("Max Stretch"));
        }
        VertexEffectConfig::ScaleByDistance { center, min_scale, max_scale, max_distance } => {
            ui.horizontal(|ui| {
                ui.label("Center:");
                ui.add(egui::DragValue::new(&mut center[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut center[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut center[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(min_scale, 0.0..=2.0).text("Min Scale"));
            ui.add(egui::Slider::new(max_scale, 0.5..=5.0).text("Max Scale"));
            ui.add(egui::Slider::new(max_distance, 0.1..=5.0).text("Max Distance"));
        }
        VertexEffectConfig::FadeByDistance { near, far } => {
            ui.add(egui::Slider::new(near, 0.0..=5.0).text("Near"));
            ui.add(egui::Slider::new(far, 0.1..=10.0).text("Far"));
        }
        VertexEffectConfig::BillboardCylindrical { axis } => {
            ui.horizontal(|ui| {
                ui.label("Axis:");
                ui.add(egui::DragValue::new(&mut axis[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut axis[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut axis[2]).speed(0.1).prefix("Z:"));
            });
        }
        VertexEffectConfig::BillboardFixed { forward, up } => {
            ui.horizontal(|ui| {
                ui.label("Forward:");
                ui.add(egui::DragValue::new(&mut forward[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut forward[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut forward[2]).speed(0.1).prefix("Z:"));
            });
            ui.horizontal(|ui| {
                ui.label("Up:");
                ui.add(egui::DragValue::new(&mut up[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut up[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut up[2]).speed(0.1).prefix("Z:"));
            });
        }
        VertexEffectConfig::FacePoint { target } => {
            ui.horizontal(|ui| {
                ui.label("Target:");
                ui.add(egui::DragValue::new(&mut target[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut target[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut target[2]).speed(0.1).prefix("Z:"));
            });
        }
    }
}
