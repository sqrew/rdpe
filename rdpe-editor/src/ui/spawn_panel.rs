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
        .add(egui::Slider::new(&mut config.bounds, 0.5..=5.0).text("Bounds"))
        .changed();

    changed |= ui
        .add(
            egui::Slider::new(&mut config.particle_size, 0.001..=0.1)
                .text("Particle Size")
                .logarithmic(true),
        )
        .changed();

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

    let shape_changed = ui.horizontal(|ui| {
        ui.label("Shape:");
        egui::ComboBox::from_id_salt("spawn_shape")
            .selected_text(shapes[shape_idx])
            .show_index(ui, &mut shape_idx, shapes.len(), |i| shapes[i])
            .changed()
    }).inner;

    if shape_changed {
        config.spawn.shape = match shape_idx {
            0 => SpawnShape::Cube { size: 0.5 },
            1 => SpawnShape::Sphere { radius: 0.5 },
            2 => SpawnShape::Shell { inner: 0.3, outer: 0.5 },
            3 => SpawnShape::Ring { radius: 0.5, thickness: 0.1 },
            4 => SpawnShape::Point,
            5 => SpawnShape::Line { length: 1.0 },
            6 => SpawnShape::Plane { width: 1.0, depth: 1.0 },
            _ => SpawnShape::Sphere { radius: 0.5 },
        };
        changed = true;
    }

    match &mut config.spawn.shape {
        SpawnShape::Cube { size } => {
            changed |= ui.add(egui::Slider::new(size, 0.1..=2.0).text("Size")).changed();
        }
        SpawnShape::Sphere { radius } => {
            changed |= ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius")).changed();
        }
        SpawnShape::Shell { inner, outer } => {
            changed |= ui.add(egui::Slider::new(inner, 0.0..=1.9).text("Inner")).changed();
            changed |= ui.add(egui::Slider::new(outer, 0.1..=2.0).text("Outer")).changed();
        }
        SpawnShape::Ring { radius, thickness } => {
            changed |= ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(thickness, 0.01..=0.5).text("Thickness")).changed();
        }
        SpawnShape::Point => {}
        SpawnShape::Line { length } => {
            changed |= ui.add(egui::Slider::new(length, 0.1..=3.0).text("Length")).changed();
        }
        SpawnShape::Plane { width, depth } => {
            changed |= ui.add(egui::Slider::new(width, 0.1..=3.0).text("Width")).changed();
            changed |= ui.add(egui::Slider::new(depth, 0.1..=3.0).text("Depth")).changed();
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
            5 => InitialVelocity::Directional { direction: [1.0, 0.0, 0.0], speed: 0.1 },
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
            changed |= ui.add(egui::Slider::new(speed, 0.0..=2.0).text("Speed")).changed();
        }
        InitialVelocity::Directional { direction, speed } => {
            changed |= ui.add(egui::Slider::new(speed, 0.0..=2.0).text("Speed")).changed();
            ui.horizontal(|ui| {
                ui.label("Dir:");
                changed |= ui.add(egui::DragValue::new(&mut direction[0]).speed(0.01).prefix("x:")).changed();
                changed |= ui.add(egui::DragValue::new(&mut direction[1]).speed(0.01).prefix("y:")).changed();
                changed |= ui.add(egui::DragValue::new(&mut direction[2]).speed(0.01).prefix("z:")).changed();
            });
        }
    }

    ui.separator();
    ui.heading("Color");

    let color_variants = ColorMode::variants();
    let mut color_idx = match &config.spawn.color_mode {
        ColorMode::Uniform { .. } => 0,
        ColorMode::RandomHue { .. } => 1,
        ColorMode::ByPosition => 2,
        ColorMode::ByVelocity => 3,
        ColorMode::Gradient { .. } => 4,
    };

    if egui::ComboBox::from_label("Color Mode")
        .show_index(ui, &mut color_idx, color_variants.len(), |i| color_variants[i])
        .changed()
    {
        config.spawn.color_mode = match color_idx {
            0 => ColorMode::Uniform { r: 1.0, g: 0.5, b: 0.2 },
            1 => ColorMode::RandomHue { saturation: 0.8, value: 0.9 },
            2 => ColorMode::ByPosition,
            3 => ColorMode::ByVelocity,
            4 => ColorMode::Gradient { start: [1.0, 0.0, 0.0], end: [0.0, 0.0, 1.0] },
            _ => ColorMode::RandomHue { saturation: 0.8, value: 0.9 },
        };
        changed = true;
    }

    match &mut config.spawn.color_mode {
        ColorMode::Uniform { r, g, b } => {
            let mut color = [*r, *g, *b];
            if ui.color_edit_button_rgb(&mut color).changed() {
                *r = color[0];
                *g = color[1];
                *b = color[2];
                changed = true;
            }
        }
        ColorMode::RandomHue { saturation, value } => {
            changed |= ui.add(egui::Slider::new(saturation, 0.0..=1.0).text("Saturation")).changed();
            changed |= ui.add(egui::Slider::new(value, 0.0..=1.0).text("Value")).changed();
        }
        ColorMode::Gradient { start, end } => {
            ui.horizontal(|ui| {
                ui.label("Start:");
                if ui.color_edit_button_rgb(start).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("End:");
                if ui.color_edit_button_rgb(end).changed() {
                    changed = true;
                }
            });
        }
        _ => {}
    }

    changed
}
