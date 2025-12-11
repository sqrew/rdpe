//! Rules editing panel
#![allow(clippy::type_complexity)]

mod helpers;
mod renderers;
mod templates;

use crate::config::RuleConfig;
use egui::Ui;

pub use templates::RULE_TEMPLATES;

use renderers::render_rule_params;

pub fn render_rules_panel(ui: &mut Ui, rules: &mut Vec<RuleConfig>) -> bool {
    let mut changed = false;
    let mut remove_idx = None;
    let mut move_up_idx = None;
    let mut move_down_idx = None;

    ui.heading("Rules");
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
