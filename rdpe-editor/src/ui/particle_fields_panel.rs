//! Particle fields configuration panel
//!
//! Allows users to define custom fields for particles beyond the base fields
//! (position, velocity, color, age, alive, scale).

use crate::config::*;
use egui::Ui;

/// Render the particle fields configuration panel.
///
/// Returns true if a rebuild is needed (fields were added/removed/modified).
pub fn render_particle_fields_panel(ui: &mut Ui, config: &mut SimConfig) -> bool {
    let mut changed = false;

    ui.heading("Custom Particle Fields");
    ui.label("Define additional state that particles can carry.");
    ui.add_space(4.0);

    // Show base fields (read-only info)
    ui.collapsing("Base Fields (always present)", |ui| {
        ui.label("These fields are built-in and cannot be removed:");
        ui.add_space(4.0);
        egui::Grid::new("base_fields_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("position");
                ui.label("vec3 - Particle position in world space");
                ui.end_row();

                ui.label("velocity");
                ui.label("vec3 - Particle velocity");
                ui.end_row();

                ui.label("color");
                ui.label("vec3 - RGB color for rendering");
                ui.end_row();

                ui.label("age");
                ui.label("f32 - Time since spawn (seconds)");
                ui.end_row();

                ui.label("alive");
                ui.label("u32 - 1 = alive, 0 = dead");
                ui.end_row();

                ui.label("scale");
                ui.label("f32 - Size multiplier");
                ui.end_row();

                ui.label("particle_type");
                ui.label("u32 - Type identifier for typed rules (default: 0)");
                ui.end_row();
            });
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // Custom fields section
    ui.horizontal(|ui| {
        ui.heading("Custom Fields");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ Add Field").clicked() {
                // Generate unique name
                let base_name = "custom";
                let mut name = base_name.to_string();
                let mut counter = 1;
                while config.particle_fields.iter().any(|f| f.name == name) {
                    name = format!("{}_{}", base_name, counter);
                    counter += 1;
                }
                config.particle_fields.push(ParticleFieldDef::new(name, ParticleFieldType::F32));
                changed = true;
            }
        });
    });

    ui.add_space(4.0);

    if config.particle_fields.is_empty() {
        ui.label("No custom fields defined. Click '+ Add Field' to add one.");
        ui.add_space(4.0);
        ui.label("Custom fields let particles carry extra state like:");
        ui.label("  - timer, charge, health, target_id, etc.");
    } else {
        // Show current memory layout info
        let layout = config.particle_layout();
        ui.label(format!(
            "Particle stride: {} bytes ({} base + {} custom)",
            layout.stride,
            layout.scale_offset + 4, // Base fields end after scale
            layout.stride - (layout.scale_offset + 4)
        ));
        ui.add_space(8.0);

        // List of custom fields with edit/delete
        let mut to_remove: Option<usize> = None;

        for (idx, field) in config.particle_fields.iter_mut().enumerate() {
            ui.push_id(idx, |ui| {
                ui.horizontal(|ui| {
                    // Field name (editable)
                    let name_response = ui.add(
                        egui::TextEdit::singleline(&mut field.name)
                            .desired_width(120.0)
                            .hint_text("field name")
                    );
                    if name_response.changed() {
                        changed = true;
                    }

                    // Validate name
                    if !field.is_valid_name() {
                        ui.colored_label(egui::Color32::RED, "!");
                    }

                    // Type dropdown
                    let types = ParticleFieldType::variants();
                    let mut type_idx = match field.field_type {
                        ParticleFieldType::F32 => 0,
                        ParticleFieldType::Vec2 => 1,
                        ParticleFieldType::Vec3 => 2,
                        ParticleFieldType::Vec4 => 3,
                        ParticleFieldType::U32 => 4,
                        ParticleFieldType::I32 => 5,
                    };

                    let type_changed = egui::ComboBox::from_id_salt(format!("field_type_{}", idx))
                        .width(70.0)
                        .selected_text(types[type_idx])
                        .show_index(ui, &mut type_idx, types.len(), |i| types[i])
                        .changed();

                    if type_changed {
                        field.field_type = ParticleFieldType::from_variant(types[type_idx])
                            .unwrap_or(ParticleFieldType::F32);
                        changed = true;
                    }

                    // Show byte size
                    ui.label(format!("({} bytes)", field.field_type.byte_size()));

                    // Delete button
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("X").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                });
            });
        }

        // Remove field if requested
        if let Some(idx) = to_remove {
            config.particle_fields.remove(idx);
            changed = true;
        }
    }

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    // Usage info
    ui.collapsing("How to use custom fields", |ui| {
        ui.label("Custom fields are accessible in rule WGSL code as p.<field_name>");
        ui.add_space(4.0);
        ui.label("Example with a 'timer' field (f32):");
        ui.code("// Decrement timer\np.timer -= uniforms.delta_time;\n\n// Do something when timer expires\nif (p.timer <= 0.0) {\n    p.timer = 2.0; // Reset\n    p.velocity *= -1.0; // Reverse\n}");
        ui.add_space(8.0);
        ui.label("Example with a 'target' field (vec3):");
        ui.code("// Move toward target\nlet dir = normalize(p.target - p.position);\np.velocity += dir * 0.1;");
    });

    changed
}
