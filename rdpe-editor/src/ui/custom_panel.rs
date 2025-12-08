//! UI panel for custom uniforms and shaders

use eframe::egui;
use std::collections::HashMap;
use crate::config::{CustomShaderConfig, UniformValueConfig};

/// Uniform type options for the dropdown
#[derive(Clone, Copy, PartialEq)]
pub enum UniformType {
    F32,
    Vec2,
    Vec3,
    Vec4,
}

impl UniformType {
    fn name(&self) -> &'static str {
        match self {
            UniformType::F32 => "f32",
            UniformType::Vec2 => "vec2",
            UniformType::Vec3 => "vec3",
            UniformType::Vec4 => "vec4",
        }
    }

    fn default_value(&self) -> UniformValueConfig {
        match self {
            UniformType::F32 => UniformValueConfig::F32(1.0),
            UniformType::Vec2 => UniformValueConfig::Vec2([0.0, 0.0]),
            UniformType::Vec3 => UniformValueConfig::Vec3([0.0, 0.0, 0.0]),
            UniformType::Vec4 => UniformValueConfig::Vec4([0.0, 0.0, 0.0, 1.0]),
        }
    }

    fn from_value(value: &UniformValueConfig) -> Self {
        match value {
            UniformValueConfig::F32(_) => UniformType::F32,
            UniformValueConfig::Vec2(_) => UniformType::Vec2,
            UniformValueConfig::Vec3(_) => UniformType::Vec3,
            UniformValueConfig::Vec4(_) => UniformType::Vec4,
        }
    }
}

/// State for adding a new uniform
pub struct AddUniformState {
    pub name: String,
    pub uniform_type: UniformType,
}

impl Default for AddUniformState {
    fn default() -> Self {
        Self {
            name: String::new(),
            uniform_type: UniformType::F32,
        }
    }
}

/// Render the custom uniforms and shaders panel
pub fn render_custom_panel(
    ui: &mut egui::Ui,
    custom_uniforms: &mut HashMap<String, UniformValueConfig>,
    custom_shaders: &mut CustomShaderConfig,
    add_uniform_state: &mut AddUniformState,
) {
    // Custom Uniforms Section
    ui.heading("Custom Uniforms");

    ui.label(egui::RichText::new("Uniforms are passed to both compute and render shaders").small().weak());
    ui.add_space(4.0);

    // Add new uniform
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(6.0)
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut add_uniform_state.name)
                    .desired_width(100.0)
                    .hint_text("my_uniform"));

                ui.label("Type:");
                egui::ComboBox::from_id_salt("add_uniform_type")
                    .selected_text(add_uniform_state.uniform_type.name())
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut add_uniform_state.uniform_type, UniformType::F32, "f32");
                        ui.selectable_value(&mut add_uniform_state.uniform_type, UniformType::Vec2, "vec2");
                        ui.selectable_value(&mut add_uniform_state.uniform_type, UniformType::Vec3, "vec3");
                        ui.selectable_value(&mut add_uniform_state.uniform_type, UniformType::Vec4, "vec4");
                    });

                let name_valid = !add_uniform_state.name.is_empty()
                    && add_uniform_state.name.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && !add_uniform_state.name.chars().next().unwrap_or('0').is_numeric()
                    && !custom_uniforms.contains_key(&add_uniform_state.name);

                if ui.add_enabled(name_valid, egui::Button::new("Add")).clicked() {
                    custom_uniforms.insert(
                        add_uniform_state.name.clone(),
                        add_uniform_state.uniform_type.default_value(),
                    );
                    add_uniform_state.name.clear();
                }
            });
        });

    ui.add_space(4.0);

    // List existing uniforms
    let mut to_remove: Option<String> = None;

    // Sort uniforms for consistent display
    let mut uniform_list: Vec<_> = custom_uniforms.iter_mut().collect();
    uniform_list.sort_by(|a, b| a.0.cmp(b.0));

    for (name, value) in uniform_list {
        ui.push_id(name.as_str(), |ui| {
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .inner_margin(6.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(format!("{}: {}", name, UniformType::from_value(value).name()));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("X").clicked() {
                                to_remove = Some(name.clone());
                            }
                        });
                    });

                    // Value editor
                    render_uniform_value(ui, value);
                });
        });
        ui.add_space(2.0);
    }

    if let Some(name) = to_remove {
        custom_uniforms.remove(&name);
    }

    if custom_uniforms.is_empty() {
        ui.label(egui::RichText::new("No custom uniforms defined").weak().italics());
    }

    ui.add_space(8.0);
    ui.separator();

    // Custom Shaders Section
    ui.heading("Custom Shaders");

    // Vertex shader code
    ui.collapsing("Vertex Code", |ui| {
        ui.label(egui::RichText::new("Available variables: pos_offset, rotated_quad, size_mult, color_mod, uniforms.time").small().weak());
        ui.add_space(2.0);

        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut custom_shaders.vertex_code)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(6)
                        .hint_text("// Custom vertex shader code\n// e.g.: size_mult *= 1.0 + 0.2 * sin(uniforms.time);"),
                );
            });
    });

    ui.add_space(4.0);

    // Fragment shader code
    ui.collapsing("Fragment Code", |ui| {
        ui.label(egui::RichText::new("Available variables: frag_color, uv, center, alpha, uniforms.time").small().weak());
        ui.add_space(2.0);

        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut custom_shaders.fragment_code)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(6)
                        .hint_text("// Custom fragment shader code\n// e.g.: frag_color *= vec3(1.0, 0.5, 0.0);"),
                );
            });
    });

    // Show reference for custom uniforms
    if !custom_uniforms.is_empty() {
        ui.add_space(4.0);
        ui.collapsing("Uniform Reference", |ui| {
            ui.label(egui::RichText::new("Access your uniforms in shaders:").small());
            for (name, value) in custom_uniforms.iter() {
                ui.label(egui::RichText::new(format!(
                    "  uniforms.{}: {}",
                    name,
                    UniformType::from_value(value).name()
                )).small().code());
            }
        });
    }
}

fn render_uniform_value(ui: &mut egui::Ui, value: &mut UniformValueConfig) {
    match value {
        UniformValueConfig::F32(v) => {
            ui.add(egui::DragValue::new(v).speed(0.01).prefix("Value: "));
        }
        UniformValueConfig::Vec2(v) => {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("X: "));
                ui.add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("Y: "));
            });
        }
        UniformValueConfig::Vec3(v) => {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("X: "));
                ui.add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("Y: "));
                ui.add(egui::DragValue::new(&mut v[2]).speed(0.01).prefix("Z: "));
            });
        }
        UniformValueConfig::Vec4(v) => {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("X: "));
                ui.add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("Y: "));
            });
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut v[2]).speed(0.01).prefix("Z: "));
                ui.add(egui::DragValue::new(&mut v[3]).speed(0.01).prefix("W: "));
            });
        }
    }
}
