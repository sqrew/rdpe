//! Spawn configuration panel

use crate::config::*;
use egui::Ui;

pub fn render_spawn_panel(ui: &mut Ui, config: &mut SimConfig) -> bool {
    let mut changed = false;

    ui.heading("Simulation");

    changed |= ui
        .add(
            egui::Slider::new(&mut config.particle_count, 100..=100_000)
                .text("Particles")
                .logarithmic(true),
        )
        .changed();

    changed |= ui
        .add(egui::Slider::new(&mut config.bounds, 0.1..=10.0).text("Bounds"))
        .changed();

    changed |= ui
        .add(
            egui::Slider::new(&mut config.particle_size, 0.0..=0.1)
                .text("Particle Size")
                .logarithmic(true),
        )
        .changed();

    // Speed slider - doesn't trigger rebuild, just changes simulation rate
    ui.add(egui::Slider::new(&mut config.speed, 0.01..=100.0).text("Speed"));

    ui.separator();
    ui.heading("Spawn Shape");

    let shapes = SpawnShape::variants();
    let mut shape_idx = match &config.spawn.shape {
        SpawnShape::Cube { .. } => 0,
        SpawnShape::Sphere { .. } => 1,
        SpawnShape::Shell { .. } => 2,
        SpawnShape::Ring { .. } => 3,
        SpawnShape::Point => 4,
        SpawnShape::Line { .. } => 5,
        SpawnShape::Plane { .. } => 6,
    };

    let shape_changed = ui
        .horizontal(|ui| {
            ui.label("Shape:");
            egui::ComboBox::from_id_salt("spawn_shape")
                .selected_text(shapes[shape_idx])
                .show_index(ui, &mut shape_idx, shapes.len(), |i| shapes[i])
                .changed()
        })
        .inner;

    if shape_changed {
        config.spawn.shape = match shape_idx {
            0 => SpawnShape::Cube { size: 0.5 },
            1 => SpawnShape::Sphere { radius: 0.5 },
            2 => SpawnShape::Shell {
                inner: 0.3,
                outer: 0.5,
            },
            3 => SpawnShape::Ring {
                radius: 0.5,
                thickness: 0.1,
            },
            4 => SpawnShape::Point,
            5 => SpawnShape::Line { length: 1.0 },
            6 => SpawnShape::Plane {
                width: 1.0,
                depth: 1.0,
            },
            _ => SpawnShape::Sphere { radius: 0.5 },
        };
        changed = true;
    }

    match &mut config.spawn.shape {
        SpawnShape::Cube { size } => {
            changed |= ui
                .add(egui::Slider::new(size, 0.1..=2.0).text("Size"))
                .changed();
        }
        SpawnShape::Sphere { radius } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"))
                .changed();
        }
        SpawnShape::Shell { inner, outer } => {
            changed |= ui
                .add(egui::Slider::new(inner, 0.0..=1.9).text("Inner"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(outer, 0.1..=2.0).text("Outer"))
                .changed();
        }
        SpawnShape::Ring { radius, thickness } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(thickness, 0.01..=0.5).text("Thickness"))
                .changed();
        }
        SpawnShape::Point => {}
        SpawnShape::Line { length } => {
            changed |= ui
                .add(egui::Slider::new(length, 0.1..=3.0).text("Length"))
                .changed();
        }
        SpawnShape::Plane { width, depth } => {
            changed |= ui
                .add(egui::Slider::new(width, 0.1..=3.0).text("Width"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(depth, 0.1..=3.0).text("Depth"))
                .changed();
        }
    }

    ui.separator();
    ui.heading("Initial Velocity");

    let vel_variants = InitialVelocity::variants();
    let mut vel_idx = match &config.spawn.velocity {
        InitialVelocity::Zero => 0,
        InitialVelocity::RandomDirection { .. } => 1,
        InitialVelocity::Outward { .. } => 2,
        InitialVelocity::Inward { .. } => 3,
        InitialVelocity::Swirl { .. } => 4,
        InitialVelocity::Directional { .. } => 5,
    };

    if egui::ComboBox::from_label("Mode")
        .show_index(ui, &mut vel_idx, vel_variants.len(), |i| vel_variants[i])
        .changed()
    {
        config.spawn.velocity = match vel_idx {
            0 => InitialVelocity::Zero,
            1 => InitialVelocity::RandomDirection { speed: 0.1 },
            2 => InitialVelocity::Outward { speed: 0.1 },
            3 => InitialVelocity::Inward { speed: 0.1 },
            4 => InitialVelocity::Swirl { speed: 0.1 },
            5 => InitialVelocity::Directional {
                direction: [1.0, 0.0, 0.0],
                speed: 0.1,
            },
            _ => InitialVelocity::Zero,
        };
        changed = true;
    }

    match &mut config.spawn.velocity {
        InitialVelocity::Zero => {}
        InitialVelocity::RandomDirection { speed }
        | InitialVelocity::Outward { speed }
        | InitialVelocity::Inward { speed }
        | InitialVelocity::Swirl { speed } => {
            changed |= ui
                .add(egui::Slider::new(speed, 0.0..=2.0).text("Speed"))
                .changed();
        }
        InitialVelocity::Directional { direction, speed } => {
            changed |= ui
                .add(egui::Slider::new(speed, 0.0..=2.0).text("Speed"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Dir:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut direction[0])
                            .speed(0.01)
                            .prefix("x:"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut direction[1])
                            .speed(0.01)
                            .prefix("y:"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut direction[2])
                            .speed(0.01)
                            .prefix("z:"),
                    )
                    .changed();
            });
        }
    }

    ui.separator();
    ui.heading("Particle Types");

    // Ensure at least one type exists
    if config.spawn.type_weights.is_empty() {
        config.spawn.type_weights.push(1.0);
    }

    let total_weight: f32 = config.spawn.type_weights.iter().sum();
    let num_types = config.spawn.type_weights.len();

    // Show each type with weight slider
    let mut remove_idx: Option<usize> = None;
    for (i, weight) in config.spawn.type_weights.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            // Calculate percentage
            let pct = if total_weight > 0.0 {
                *weight / total_weight * 100.0
            } else {
                0.0
            };

            ui.label(format!("Type {}:", i));
            if ui
                .add(egui::DragValue::new(weight).speed(0.1).range(0.0..=100.0))
                .changed()
            {
                changed = true;
            }
            ui.label(format!("({:.0}%)", pct));

            // Remove button (only if more than 1 type)
            if num_types > 1 && ui.small_button("×").clicked() {
                remove_idx = Some(i);
            }
        });
    }

    // Remove type if requested
    if let Some(idx) = remove_idx {
        config.spawn.type_weights.remove(idx);
        changed = true;
    }

    // Add type button
    ui.horizontal(|ui| {
        if ui.button("+ Add Type").clicked() {
            // New type gets same weight as average
            let avg = total_weight / num_types as f32;
            config.spawn.type_weights.push(avg.max(1.0));
            changed = true;
        }

        // Quick presets
        ui.menu_button("Presets", |ui| {
            if ui.button("50/50 (2 types)").clicked() {
                config.spawn.type_weights = vec![1.0, 1.0];
                changed = true;
                ui.close_menu();
            }
            if ui.button("Predator/Prey (20/80)").clicked() {
                config.spawn.type_weights = vec![4.0, 1.0]; // 80% type 0, 20% type 1
                changed = true;
                ui.close_menu();
            }
            if ui.button("DLA (1/99 seed/mobile)").clicked() {
                config.spawn.type_weights = vec![1.0, 99.0]; // 1% seeds, 99% mobile
                changed = true;
                ui.close_menu();
            }
            if ui.button("3 Types Equal").clicked() {
                config.spawn.type_weights = vec![1.0, 1.0, 1.0];
                changed = true;
                ui.close_menu();
            }
            if ui.button("Reset (All Type 0)").clicked() {
                config.spawn.type_weights = vec![1.0];
                changed = true;
                ui.close_menu();
            }
        });
    });

    changed
}
