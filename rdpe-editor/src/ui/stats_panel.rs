//! Statistics panel for simulation analysis.

use crate::embedded::{FieldStats, SimulationStats};
use egui::Ui;

/// Render the statistics panel.
pub fn render_stats_panel(ui: &mut Ui, stats: &SimulationStats) {
    ui.heading("Statistics");

    ui.label(
        egui::RichText::new("Live simulation metrics (updated periodically)")
            .small()
            .weak(),
    );

    ui.add_space(8.0);

    // Particle counts
    ui.group(|ui| {
        ui.label(egui::RichText::new("Particles").strong());
        ui.horizontal(|ui| {
            ui.label("Alive:");
            ui.label(format!("{} / {}", stats.alive_particles, stats.total_particles));
        });
        if stats.total_particles > 0 {
            let pct = (stats.alive_particles as f32 / stats.total_particles as f32) * 100.0;
            ui.add(egui::ProgressBar::new(pct / 100.0).text(format!("{:.1}%", pct)));
        }
    });

    ui.add_space(4.0);

    // Spatial metrics
    ui.group(|ui| {
        ui.label(egui::RichText::new("Spatial").strong());

        ui.horizontal(|ui| {
            ui.label("Center of Mass:");
        });
        ui.label(
            egui::RichText::new(format!(
                "  ({:.3}, {:.3}, {:.3})",
                stats.center_of_mass.x, stats.center_of_mass.y, stats.center_of_mass.z
            ))
            .monospace()
            .small(),
        );

        ui.horizontal(|ui| {
            ui.label("Bounding Box:");
        });
        ui.label(
            egui::RichText::new(format!(
                "  Min: ({:.2}, {:.2}, {:.2})",
                stats.bounding_min.x, stats.bounding_min.y, stats.bounding_min.z
            ))
            .monospace()
            .small(),
        );
        ui.label(
            egui::RichText::new(format!(
                "  Max: ({:.2}, {:.2}, {:.2})",
                stats.bounding_max.x, stats.bounding_max.y, stats.bounding_max.z
            ))
            .monospace()
            .small(),
        );
    });

    ui.add_space(4.0);

    // Velocity metrics
    ui.group(|ui| {
        ui.label(egui::RichText::new("Velocity").strong());

        ui.horizontal(|ui| {
            ui.label("Avg Speed:");
            ui.label(format!("{:.4}", stats.avg_speed));
        });

        ui.horizontal(|ui| {
            ui.label("Max Speed:");
            ui.label(format!("{:.4}", stats.max_speed));
        });

        ui.horizontal(|ui| {
            ui.label("Avg Direction:");
        });
        ui.label(
            egui::RichText::new(format!(
                "  ({:.3}, {:.3}, {:.3})",
                stats.avg_velocity.x, stats.avg_velocity.y, stats.avg_velocity.z
            ))
            .monospace()
            .small(),
        );
    });

    // Per-field statistics
    if !stats.field_stats.is_empty() {
        ui.add_space(8.0);
        ui.separator();
        ui.label(egui::RichText::new("Per-Field Statistics").strong());

        for field in &stats.field_stats {
            render_field_stats(ui, field);
        }
    }
}

fn render_field_stats(ui: &mut Ui, field: &FieldStats) {
    let id = ui.make_persistent_id(format!("field_stats_{}", field.name));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.label(&field.name);
        })
        .body(|ui| {
            if field.components == 1 {
                // Scalar field
                ui.horizontal(|ui| {
                    ui.label("Min:");
                    ui.label(egui::RichText::new(format!("{:.4}", field.min.x)).monospace());
                });
                ui.horizontal(|ui| {
                    ui.label("Max:");
                    ui.label(egui::RichText::new(format!("{:.4}", field.max.x)).monospace());
                });
                ui.horizontal(|ui| {
                    ui.label("Avg:");
                    ui.label(egui::RichText::new(format!("{:.4}", field.avg.x)).monospace());
                });
            } else {
                // Vector field
                ui.label(
                    egui::RichText::new(format!(
                        "Min: ({:.3}, {:.3}, {:.3})",
                        field.min.x, field.min.y, field.min.z
                    ))
                    .monospace()
                    .small(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Max: ({:.3}, {:.3}, {:.3})",
                        field.max.x, field.max.y, field.max.z
                    ))
                    .monospace()
                    .small(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Avg: ({:.3}, {:.3}, {:.3})",
                        field.avg.x, field.avg.y, field.avg.z
                    ))
                    .monospace()
                    .small(),
                );
            }
        });
}
