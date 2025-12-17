//! UI panel for vertex effects

use eframe::egui;
use crate::config::VertexEffectConfig;

/// Effect template for creating new effects
struct EffectTemplate {
    name: &'static str,
    create: fn() -> VertexEffectConfig,
}

/// A category of effects with a label
struct EffectCategory {
    label: &'static str,
    effects: &'static [EffectTemplate],
}

const EFFECT_CATEGORIES: &[EffectCategory] = &[
    EffectCategory {
        label: "Visual Motion",
        effects: &[
            EffectTemplate { name: "Orbit", create: || VertexEffectConfig::Orbit { center: [0.0, 0.0, 0.0], speed: 2.0, radius: 0.3, axis: [0.0, 1.0, 0.0] } },
            EffectTemplate { name: "Spiral", create: || VertexEffectConfig::Spiral { center: [0.0, 0.0, 0.0], speed: 2.0, expansion: 0.1, vertical_speed: 0.2 } },
            EffectTemplate { name: "Vortex", create: || VertexEffectConfig::Vortex { center: [0.0, 0.0, 0.0], speed: 2.0, pull: 0.3, radius: 1.0 } },
            EffectTemplate { name: "Helix", create: || VertexEffectConfig::Helix { axis: [0.0, 1.0, 0.0], radius: 0.2, speed: 2.0, progression: 0.5 } },
            EffectTemplate { name: "Figure 8", create: || VertexEffectConfig::Figure8 { width: 0.3, height: 0.2, speed: 1.5, ratio: 2.0 } },
            EffectTemplate { name: "Bounce", create: || VertexEffectConfig::Bounce { height: 0.3, frequency: 2.0, damping: 0.2 } },
            EffectTemplate { name: "Sway", create: || VertexEffectConfig::Sway { frequency: 2.0, amplitude: 0.2, axis: [1.0, 0.0, 0.0] } },
            EffectTemplate { name: "Wave", create: || VertexEffectConfig::Wave { direction: [0.0, 1.0, 0.0], frequency: 2.0, speed: 2.0, amplitude: 0.2 } },
            EffectTemplate { name: "Wobble", create: || VertexEffectConfig::Wobble { frequency: 3.0, amplitude: 0.3 } },
            EffectTemplate { name: "Flutter", create: || VertexEffectConfig::Flutter { intensity: 0.05, speed: 1.0 } },
            EffectTemplate { name: "Jitter", create: || VertexEffectConfig::Jitter { amplitude: 0.1 } },
            EffectTemplate { name: "Brownian", create: || VertexEffectConfig::Brownian { intensity: 0.15, speed: 1.0 } },
        ],
    },
    EffectCategory {
        label: "Rotation",
        effects: &[
            EffectTemplate { name: "Rotate", create: || VertexEffectConfig::Rotate { speed: 3.0 } },
            EffectTemplate { name: "Tumble", create: || VertexEffectConfig::Tumble { speed: 2.0 } },
        ],
    },
    EffectCategory {
        label: "Scale & Transform",
        effects: &[
            EffectTemplate { name: "Pulse", create: || VertexEffectConfig::Pulse { frequency: 2.0, amplitude: 0.5 } },
            EffectTemplate { name: "Squash", create: || VertexEffectConfig::Squash { axis: [0.0, 1.0, 0.0], amount: 0.3 } },
            EffectTemplate { name: "Stretch To Velocity", create: || VertexEffectConfig::StretchToVelocity { max_stretch: 3.0 } },
            EffectTemplate { name: "Scale By Distance", create: || VertexEffectConfig::ScaleByDistance { center: [0.0, 0.0, 0.0], min_scale: 0.3, max_scale: 2.5, max_distance: 1.0 } },
            EffectTemplate { name: "Scale By Speed", create: || VertexEffectConfig::ScaleBySpeed { min_scale: 0.5, max_scale: 2.0, max_speed: 1.0 } },
            EffectTemplate { name: "Scale By Age", create: || VertexEffectConfig::ScaleByAge { start_scale: 1.0, end_scale: 0.0, lifetime: 3.0 } },
        ],
    },
    EffectCategory {
        label: "Position",
        effects: &[
            EffectTemplate { name: "Attract", create: || VertexEffectConfig::Attract { target: [0.0, 0.0, 0.0], strength: 0.5, max_displacement: 0.5 } },
            EffectTemplate { name: "Repel", create: || VertexEffectConfig::Repel { source: [0.0, 0.0, 0.0], strength: 0.5, radius: 1.0 } },
            EffectTemplate { name: "Turbulence", create: || VertexEffectConfig::Turbulence { frequency: 2.0, amplitude: 0.1, speed: 1.0 } },
        ],
    },
    EffectCategory {
        label: "Orientation",
        effects: &[
            EffectTemplate { name: "Face Point", create: || VertexEffectConfig::FacePoint { target: [0.0, 0.0, 0.0] } },
            EffectTemplate { name: "Billboard Cylindrical", create: || VertexEffectConfig::BillboardCylindrical { axis: [0.0, 1.0, 0.0] } },
            EffectTemplate { name: "Billboard Fixed", create: || VertexEffectConfig::BillboardFixed { forward: [0.0, 0.0, 1.0], up: [0.0, 1.0, 0.0] } },
        ],
    },
    EffectCategory {
        label: "Fade & Visibility",
        effects: &[
            EffectTemplate { name: "Fade By Distance", create: || VertexEffectConfig::FadeByDistance { near: 0.3, far: 1.5 } },
            EffectTemplate { name: "Fade By Age", create: || VertexEffectConfig::FadeByAge { start_alpha: 1.0, end_alpha: 0.0, lifetime: 3.0 } },
        ],
    },
];

pub fn render_effects_panel(ui: &mut egui::Ui, effects: &mut Vec<VertexEffectConfig>) {
    ui.heading("Vertex Effects");

    // Add effect dropdown
    ui.horizontal(|ui| {
        ui.label("Add Effect:");
        ui.menu_button("+ Add...", |ui| {
            ui.set_min_width(180.0);

            // Wrap in scroll area with max height to prevent overflow
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for (cat_idx, category) in EFFECT_CATEGORIES.iter().enumerate() {
                        if cat_idx > 0 {
                            ui.separator();
                        }
                        ui.label(egui::RichText::new(category.label).small().strong());

                        for template in category.effects {
                            if ui.button(template.name).clicked() {
                                effects.push((template.create)());
                                ui.close_menu();
                            }
                        }
                    }
                });
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
        // Motion-based effects
        VertexEffectConfig::Orbit { center, speed, radius, axis } => {
            ui.horizontal(|ui| {
                ui.label("Center:");
                ui.add(egui::DragValue::new(&mut center[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut center[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut center[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(speed, -10.0..=10.0).text("Speed"));
            ui.add(egui::Slider::new(radius, 0.0..=2.0).text("Radius"));
            ui.horizontal(|ui| {
                ui.label("Axis:");
                ui.add(egui::DragValue::new(&mut axis[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut axis[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut axis[2]).speed(0.1).prefix("Z:"));
            });
        }
        VertexEffectConfig::Spiral { center, speed, expansion, vertical_speed } => {
            ui.horizontal(|ui| {
                ui.label("Center:");
                ui.add(egui::DragValue::new(&mut center[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut center[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut center[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(speed, -10.0..=10.0).text("Speed"));
            ui.add(egui::Slider::new(expansion, -1.0..=1.0).text("Expansion"));
            ui.add(egui::Slider::new(vertical_speed, -2.0..=2.0).text("Vertical Speed"));
        }
        VertexEffectConfig::Sway { frequency, amplitude, axis } => {
            ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"));
            ui.add(egui::Slider::new(amplitude, 0.0..=1.0).text("Amplitude"));
            ui.horizontal(|ui| {
                ui.label("Axis:");
                ui.add(egui::DragValue::new(&mut axis[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut axis[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut axis[2]).speed(0.1).prefix("Z:"));
            });
        }
        // Scale/Transform effects
        VertexEffectConfig::ScaleBySpeed { min_scale, max_scale, max_speed } => {
            ui.add(egui::Slider::new(min_scale, 0.0..=2.0).text("Min Scale"));
            ui.add(egui::Slider::new(max_scale, 0.5..=5.0).text("Max Scale"));
            ui.add(egui::Slider::new(max_speed, 0.1..=5.0).text("Max Speed"));
        }
        VertexEffectConfig::Squash { axis, amount } => {
            ui.horizontal(|ui| {
                ui.label("Axis:");
                ui.add(egui::DragValue::new(&mut axis[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut axis[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut axis[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(amount, 0.0..=1.0).text("Amount"));
        }
        VertexEffectConfig::Tumble { speed } => {
            ui.add(egui::Slider::new(speed, 0.0..=10.0).text("Speed"));
        }
        // Position effects
        VertexEffectConfig::Attract { target, strength, max_displacement } => {
            ui.horizontal(|ui| {
                ui.label("Target:");
                ui.add(egui::DragValue::new(&mut target[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut target[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut target[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"));
            ui.add(egui::Slider::new(max_displacement, 0.0..=2.0).text("Max Displacement"));
        }
        VertexEffectConfig::Repel { source, strength, radius } => {
            ui.horizontal(|ui| {
                ui.label("Source:");
                ui.add(egui::DragValue::new(&mut source[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut source[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut source[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"));
            ui.add(egui::Slider::new(radius, 0.1..=5.0).text("Radius"));
        }
        VertexEffectConfig::Turbulence { frequency, amplitude, speed } => {
            ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"));
            ui.add(egui::Slider::new(amplitude, 0.0..=1.0).text("Amplitude"));
            ui.add(egui::Slider::new(speed, 0.0..=5.0).text("Speed"));
        }
        // Time-based effects
        VertexEffectConfig::ScaleByAge { start_scale, end_scale, lifetime } => {
            ui.add(egui::Slider::new(start_scale, 0.0..=3.0).text("Start Scale"));
            ui.add(egui::Slider::new(end_scale, 0.0..=3.0).text("End Scale"));
            ui.add(egui::Slider::new(lifetime, 0.1..=10.0).text("Lifetime"));
        }
        VertexEffectConfig::FadeByAge { start_alpha, end_alpha, lifetime } => {
            ui.add(egui::Slider::new(start_alpha, 0.0..=1.0).text("Start Alpha"));
            ui.add(egui::Slider::new(end_alpha, 0.0..=1.0).text("End Alpha"));
            ui.add(egui::Slider::new(lifetime, 0.1..=10.0).text("Lifetime"));
        }
        // Additional motion effects
        VertexEffectConfig::Vortex { center, speed, pull, radius } => {
            ui.horizontal(|ui| {
                ui.label("Center:");
                ui.add(egui::DragValue::new(&mut center[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut center[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut center[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(speed, -10.0..=10.0).text("Speed"));
            ui.add(egui::Slider::new(pull, -2.0..=2.0).text("Pull"));
            ui.add(egui::Slider::new(radius, 0.1..=5.0).text("Radius"));
        }
        VertexEffectConfig::Bounce { height, frequency, damping } => {
            ui.add(egui::Slider::new(height, 0.0..=2.0).text("Height"));
            ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"));
            ui.add(egui::Slider::new(damping, 0.0..=1.0).text("Damping"));
        }
        VertexEffectConfig::Figure8 { width, height, speed, ratio } => {
            ui.add(egui::Slider::new(width, 0.0..=1.0).text("Width"));
            ui.add(egui::Slider::new(height, 0.0..=1.0).text("Height"));
            ui.add(egui::Slider::new(speed, 0.1..=5.0).text("Speed"));
            ui.add(egui::Slider::new(ratio, 1.0..=5.0).text("Ratio"));
        }
        VertexEffectConfig::Helix { axis, radius, speed, progression } => {
            ui.horizontal(|ui| {
                ui.label("Axis:");
                ui.add(egui::DragValue::new(&mut axis[0]).speed(0.1).prefix("X:"));
                ui.add(egui::DragValue::new(&mut axis[1]).speed(0.1).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut axis[2]).speed(0.1).prefix("Z:"));
            });
            ui.add(egui::Slider::new(radius, 0.0..=1.0).text("Radius"));
            ui.add(egui::Slider::new(speed, -10.0..=10.0).text("Speed"));
            ui.add(egui::Slider::new(progression, 0.0..=2.0).text("Progression"));
        }
        VertexEffectConfig::Flutter { intensity, speed } => {
            ui.add(egui::Slider::new(intensity, 0.0..=0.5).text("Intensity"));
            ui.add(egui::Slider::new(speed, 0.1..=5.0).text("Speed"));
        }
        VertexEffectConfig::Brownian { intensity, speed } => {
            ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity"));
            ui.add(egui::Slider::new(speed, 0.1..=5.0).text("Speed"));
        }
    }
}
