//! Rules editing panel
#![allow(clippy::type_complexity)]

mod helpers;
mod renderers;
mod templates;

use crate::config::{FieldConfigEntry, ParticleFieldDef, RuleConfig};
use egui::Ui;

pub use templates::RULE_TEMPLATES;

use renderers::render_rule_params;

/// Context for what shader features are available
pub struct ShaderContext<'a> {
    pub particle_fields: &'a [ParticleFieldDef],
    pub fields: &'a [FieldConfigEntry],
    pub adjacency_enabled: bool,
    pub has_neighbor_rules: bool,
}

pub fn render_rules_panel(ui: &mut Ui, rules: &mut Vec<RuleConfig>, ctx: &ShaderContext) -> bool {
    let mut changed = false;
    let mut remove_idx = None;
    let mut move_up_idx = None;
    let mut move_down_idx = None;

    ui.heading("Rules");

    // Available fields helper
    ui.collapsing("Available Fields", |ui| {
        ui.label("Base fields:");
        ui.horizontal_wrapped(|ui| {
            for field in &["position", "velocity", "age", "particle_type", "alive", "scale", "color"] {
                ui.code(*field);
            }
        });
        if !ctx.particle_fields.is_empty() {
            ui.separator();
            ui.label("Custom fields:");
            ui.horizontal_wrapped(|ui| {
                for field in ctx.particle_fields {
                    ui.code(format!("{}: {}", field.name, field.field_type.wgsl_type()));
                }
            });
        }
    });

    // Available functions helper
    ui.collapsing("Available Functions", |ui| {
        ui.label("Always available:");
        ui.horizontal_wrapped(|ui| {
            ui.code("rand(&seed) -> f32");
        });
        ui.small("Mutates seed, returns 0.0-1.0");

        if !ctx.fields.is_empty() {
            ui.separator();
            ui.label("Field functions:");
            ui.horizontal_wrapped(|ui| {
                ui.code("field_read(idx, pos) -> f32");
            });
            ui.horizontal_wrapped(|ui| {
                ui.code("field_write(idx, pos, val)");
            });
            ui.small(format!("Fields: {}", ctx.fields.iter().enumerate()
                .map(|(i, f)| format!("{}: {}", i, f.name))
                .collect::<Vec<_>>()
                .join(", ")));
        }

        if ctx.adjacency_enabled {
            ui.separator();
            ui.label("Adjacency functions:");
            ui.horizontal_wrapped(|ui| {
                ui.code("adjacency_count(idx) -> u32");
            });
            ui.horizontal_wrapped(|ui| {
                ui.code("adjacency_neighbor(idx, n) -> u32");
            });
            ui.small("Returns neighbor particle indices");
        }

        if ctx.has_neighbor_rules {
            ui.separator();
            ui.label("Neighbor context (in OnCollision):");
            ui.horizontal_wrapped(|ui| {
                ui.code("other");
            });
            ui.small("Reference to neighboring particle");
        }

        ui.separator();
        ui.label("Built-in variables:");
        ui.horizontal_wrapped(|ui| {
            ui.code("index");
            ui.code("uniforms.time");
            ui.code("uniforms.delta_time");
        });
    });

    ui.separator();

    // Add rule dropdown
    egui::ComboBox::from_label("Add Rule")
        .selected_text("Select...")
        .show_ui(ui, |ui| {
            for (category, templates) in RULE_TEMPLATES {
                ui.separator();
                ui.label(*category);
                for (name, factory) in *templates {
                    if ui.selectable_label(false, *name).clicked() {
                        rules.push(factory());
                        changed = true;
                    }
                }
            }
        });

    ui.separator();

    // List existing rules
    let rules_len = rules.len();
    for (idx, rule) in rules.iter_mut().enumerate() {
        let id = ui.make_persistent_id(format!("rule_{}", idx));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{}.", idx + 1));
                    ui.strong(rule.name());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("X").clicked() {
                            remove_idx = Some(idx);
                        }
                        if idx < rules_len - 1 && ui.small_button("v").clicked() {
                            move_down_idx = Some(idx);
                        }
                        if idx > 0 && ui.small_button("^").clicked() {
                            move_up_idx = Some(idx);
                        }
                    });
                });
            })
            .body(|ui| {
                changed |= render_rule_params(ui, rule);
            });
    }

    // Handle removals and reordering
    if let Some(idx) = remove_idx {
        rules.remove(idx);
        changed = true;
    }
    if let Some(idx) = move_up_idx {
        rules.swap(idx, idx - 1);
        changed = true;
    }
    if let Some(idx) = move_down_idx {
        rules.swap(idx, idx + 1);
        changed = true;
    }

    changed
}
