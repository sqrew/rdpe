//! Code export panel - shows generated Rust code

use crate::config::SimConfig;
use crate::code_export::generate_code;
use egui::{ScrollArea, TextEdit, Ui};

/// State for the export panel
#[derive(Default)]
pub struct ExportPanelState {
    /// Whether the panel is open
    pub open: bool,
    /// Cached generated code
    pub code: String,
    /// Whether code was just copied
    pub just_copied: bool,
    /// Timer for "Copied!" feedback
    pub copy_feedback_timer: f32,
}

impl ExportPanelState {
    /// Regenerate code from config
    pub fn regenerate(&mut self, config: &SimConfig) {
        self.code = generate_code(config);
    }
}

/// Render the export panel as a window
pub fn render_export_window(
    ctx: &egui::Context,
    state: &mut ExportPanelState,
    config: &SimConfig,
    delta_time: f32,
) {
    if !state.open {
        return;
    }

    // Update copy feedback timer
    if state.copy_feedback_timer > 0.0 {
        state.copy_feedback_timer -= delta_time;
        if state.copy_feedback_timer <= 0.0 {
            state.just_copied = false;
        }
    }

    let mut open = state.open;
    egui::Window::new("Export to Code")
        .open(&mut open)
        .default_size([600.0, 500.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Regenerate").clicked() {
                    state.regenerate(config);
                }

                if state.just_copied {
                    ui.label(egui::RichText::new("Copied!").color(egui::Color32::GREEN));
                } else if ui.button("Copy to Clipboard").clicked() {
                    ui.ctx().copy_text(state.code.clone());
                    state.just_copied = true;
                    state.copy_feedback_timer = 2.0;
                }

                ui.separator();

                ui.label(
                    egui::RichText::new(format!("{} lines", state.code.lines().count()))
                        .weak()
                        .small(),
                );
            });

            ui.separator();

            // Code display
            ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let code_style = egui::TextStyle::Monospace;
                    let mut code = state.code.clone();
                    ui.add(
                        TextEdit::multiline(&mut code)
                            .font(code_style)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(30)
                            .interactive(false),
                    );
                });
        });
    state.open = open;
}

/// Render an "Export" button that opens the export panel
pub fn render_export_button(ui: &mut Ui, state: &mut ExportPanelState, config: &SimConfig) -> bool {
    let clicked = ui.button("Export to Code").clicked();
    if clicked {
        state.open = true;
        state.regenerate(config);
    }
    clicked
}
