//! UI panel for visual settings

use eframe::egui;
use crate::config::{
    BlendModeConfig, ColorMappingConfig, ColorMode,
    PaletteConfig, ParticleShapeConfig, SimConfig, WireframeMeshConfig,
};

pub fn render_visuals_panel(ui: &mut egui::Ui, config: &mut SimConfig) -> bool {
    let mut changed = false;
    let visuals = &mut config.visuals;

    ui.heading("Visuals");

    // Blend Mode
    egui::ComboBox::from_label("Blend Mode")
        .selected_text(format!("{:?}", visuals.blend_mode))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut visuals.blend_mode, BlendModeConfig::Alpha, "Alpha");
            ui.selectable_value(&mut visuals.blend_mode, BlendModeConfig::Additive, "Additive");
            ui.selectable_value(&mut visuals.blend_mode, BlendModeConfig::Multiply, "Multiply");
            ui.selectable_value(&mut visuals.blend_mode, BlendModeConfig::Screen, "Screen");
            ui.selectable_value(&mut visuals.blend_mode, BlendModeConfig::Overlay, "Overlay");
            ui.selectable_value(&mut visuals.blend_mode, BlendModeConfig::SoftLight, "Soft Light");
            ui.selectable_value(&mut visuals.blend_mode, BlendModeConfig::Subtractive, "Subtractive");
        });

    // Particle Shape
    ui.horizontal(|ui| {
        ui.label("Shape:");
        egui::ComboBox::from_id_salt("particle_shape")
            .selected_text(format!("{:?}", visuals.shape))
            .show_ui(ui, |ui| {
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::Circle, "Circle");
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::CircleHard, "Circle Hard");
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::Square, "Square");
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::Ring, "Ring");
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::Star, "Star");
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::Triangle, "Triangle");
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::Hexagon, "Hexagon");
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::Diamond, "Diamond");
            ui.selectable_value(&mut visuals.shape, ParticleShapeConfig::Point, "Point");
        });
    });

    ui.add_space(4.0);

    // Palette
    let palette_name = visuals.palette.name();
    egui::ComboBox::from_label("Palette")
        .selected_text(palette_name)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut visuals.palette, PaletteConfig::None, "None");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Viridis, "Viridis");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Magma, "Magma");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Plasma, "Plasma");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Inferno, "Inferno");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Rainbow, "Rainbow");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Sunset, "Sunset");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Ocean, "Ocean");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Fire, "Fire");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Ice, "Ice");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Neon, "Neon");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Forest, "Forest");
            ui.selectable_value(&mut visuals.palette, PaletteConfig::Grayscale, "Grayscale");
            ui.separator();
            if ui.selectable_label(matches!(visuals.palette, PaletteConfig::Custom { .. }), "Custom").clicked() {
                if !matches!(visuals.palette, PaletteConfig::Custom { .. }) {
                    visuals.palette = PaletteConfig::default_custom();
                }
            }
        });

    // Custom palette editor
    if let PaletteConfig::Custom { colors } = &mut visuals.palette {
        ui.group(|ui| {
            ui.label("Custom Palette Colors:");
            // Show color gradient preview
            let preview_rect = ui.available_rect_before_wrap();
            let preview_height = 20.0;
            let preview_width = preview_rect.width().min(200.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(preview_width, preview_height), egui::Sense::hover());

            // Draw gradient preview
            let painter = ui.painter();
            for i in 0..100 {
                let t = i as f32 / 99.0;
                let segment = t * 4.0;
                let idx = (segment as usize).min(3);
                let frac = segment.fract();
                let c1 = colors[idx];
                let c2 = colors[(idx + 1).min(4)];
                let color = [
                    c1[0] + (c2[0] - c1[0]) * frac,
                    c1[1] + (c2[1] - c1[1]) * frac,
                    c1[2] + (c2[2] - c1[2]) * frac,
                ];
                let x = rect.left() + (i as f32 / 99.0) * rect.width();
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8,
                    )),
                );
            }

            ui.add_space(4.0);

            // Color stop editors
            let labels = ["Stop 1", "Stop 2", "Stop 3", "Stop 4", "Stop 5"];
            for (i, label) in labels.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(*label);
                    ui.color_edit_button_rgb(&mut colors[i]);
                });
            }
        });
    }

    // Color Mapping (only show if palette is not None)
    if visuals.palette != PaletteConfig::None {
        let current_mapping = visuals.color_mapping.name();
        egui::ComboBox::from_label("Color Mapping")
            .selected_text(current_mapping)
            .show_ui(ui, |ui| {
                if ui.selectable_label(matches!(visuals.color_mapping, ColorMappingConfig::None), "None").clicked() {
                    visuals.color_mapping = ColorMappingConfig::None;
                }
                if ui.selectable_label(matches!(visuals.color_mapping, ColorMappingConfig::Index), "Index").clicked() {
                    visuals.color_mapping = ColorMappingConfig::Index;
                }
                if ui.selectable_label(matches!(visuals.color_mapping, ColorMappingConfig::Speed { .. }), "Speed").clicked() {
                    visuals.color_mapping = ColorMappingConfig::Speed { min: 0.0, max: 1.0 };
                }
                if ui.selectable_label(matches!(visuals.color_mapping, ColorMappingConfig::Age { .. }), "Age").clicked() {
                    visuals.color_mapping = ColorMappingConfig::Age { max_age: 5.0 };
                }
                if ui.selectable_label(matches!(visuals.color_mapping, ColorMappingConfig::PositionY { .. }), "Position Y").clicked() {
                    visuals.color_mapping = ColorMappingConfig::PositionY { min: -1.0, max: 1.0 };
                }
                if ui.selectable_label(matches!(visuals.color_mapping, ColorMappingConfig::Distance { .. }), "Distance").clicked() {
                    visuals.color_mapping = ColorMappingConfig::Distance { max_dist: 1.0 };
                }
                if ui.selectable_label(matches!(visuals.color_mapping, ColorMappingConfig::Random), "Random").clicked() {
                    visuals.color_mapping = ColorMappingConfig::Random;
                }
            });

        // Show parameters for mappings that have them
        match &mut visuals.color_mapping {
            ColorMappingConfig::Speed { min, max } => {
                ui.horizontal(|ui| {
                    ui.label("Speed Range:");
                    ui.add(egui::DragValue::new(min).speed(0.1).prefix("Min: "));
                    ui.add(egui::DragValue::new(max).speed(0.1).prefix("Max: "));
                });
            }
            ColorMappingConfig::Age { max_age } => {
                ui.add(egui::Slider::new(max_age, 0.1..=20.0).text("Max Age"));
            }
            ColorMappingConfig::PositionY { min, max } => {
                ui.horizontal(|ui| {
                    ui.label("Y Range:");
                    ui.add(egui::DragValue::new(min).speed(0.1).prefix("Min: "));
                    ui.add(egui::DragValue::new(max).speed(0.1).prefix("Max: "));
                });
            }
            ColorMappingConfig::Distance { max_dist } => {
                ui.add(egui::Slider::new(max_dist, 0.1..=5.0).text("Max Distance"));
            }
            _ => {}
        }
    }

    ui.add_space(4.0);

    // Background Color
    ui.horizontal(|ui| {
        ui.label("Background:");
        let mut color = visuals.background_color;
        if ui.color_edit_button_rgb(&mut color).changed() {
            visuals.background_color = color;
        }
    });

    ui.add_space(4.0);
    ui.separator();

    // Spawn Color (initial particle color - requires reset to apply)
    ui.heading("Spawn Color");
    ui.label("(Requires reset to apply)");

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

    ui.add_space(4.0);
    ui.separator();

    // Trail Length
    ui.add(egui::Slider::new(&mut visuals.trail_length, 0..=50).text("Trail Length"));

    // Trail gradient colors (show when trails are enabled)
    if visuals.trail_length > 0 {
        // Enable/disable custom trail colors
        let has_custom_trail_colors = visuals.trail_start_color.is_some();
        let mut enable_gradient = has_custom_trail_colors;
        if ui.checkbox(&mut enable_gradient, "Trail Gradient")
            .on_hover_text("Customize trail start and end colors")
            .changed()
        {
            if enable_gradient && visuals.trail_start_color.is_none() {
                visuals.trail_start_color = Some([0.7, 0.85, 1.0]);
                visuals.trail_end_color = Some([0.2, 0.3, 0.5]);
            } else if !enable_gradient {
                visuals.trail_start_color = None;
                visuals.trail_end_color = None;
            }
        }

        if let (Some(start), Some(end)) = (&mut visuals.trail_start_color, &mut visuals.trail_end_color) {
            ui.horizontal(|ui| {
                ui.label("Start:");
                ui.color_edit_button_rgb(start);
                ui.label("End:");
                ui.color_edit_button_rgb(end);
            });
        }
    }

    // Connections
    ui.checkbox(&mut visuals.connections_enabled, "Connections");
    if visuals.connections_enabled {
        ui.add(egui::Slider::new(&mut visuals.connections_radius, 0.01..=0.5).text("Connection Radius"));
        ui.add(egui::Slider::new(&mut visuals.connections_thickness, 0.5..=5.0).text("Connection Thickness"));
        ui.horizontal(|ui| {
            ui.label("Connection Color:");
            ui.color_edit_button_rgb(&mut visuals.connections_color);
        });
    }

    // Velocity Stretch
    ui.checkbox(&mut visuals.velocity_stretch, "Velocity Stretch");
    if visuals.velocity_stretch {
        ui.add(egui::Slider::new(&mut visuals.velocity_stretch_factor, 1.0..=5.0).text("Stretch Factor"));
    }

    // Spatial Grid Debug
    ui.add(egui::Slider::new(&mut visuals.spatial_grid_opacity, 0.0..=1.0).text("Grid Opacity"));

    ui.add_space(4.0);
    ui.separator();

    // Wireframe
    egui::ComboBox::from_label("Wireframe")
        .selected_text(visuals.wireframe.name())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::None, "None");
            ui.separator();
            ui.label(egui::RichText::new("Platonic Solids").small().weak());
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Tetrahedron, "Tetrahedron");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Cube, "Cube");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Octahedron, "Octahedron");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Icosahedron, "Icosahedron");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Dodecahedron, "Dodecahedron");
            ui.separator();
            ui.label(egui::RichText::new("Other Shapes").small().weak());
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Diamond, "Diamond");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Pyramid, "Pyramid");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Star, "Star");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Cross, "Cross");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Axes, "Axes (XYZ)");
            ui.separator();
            ui.label(egui::RichText::new("Parametric").small().weak());
            if ui.selectable_label(matches!(visuals.wireframe, WireframeMeshConfig::Prism { .. }), "Prism").clicked() {
                visuals.wireframe = WireframeMeshConfig::Prism { sides: 6 };
            }
            if ui.selectable_label(matches!(visuals.wireframe, WireframeMeshConfig::Spiral { .. }), "Spiral").clicked() {
                visuals.wireframe = WireframeMeshConfig::Spiral { turns: 2.0, segments: 24 };
            }
            if ui.selectable_label(matches!(visuals.wireframe, WireframeMeshConfig::Sphere { .. }), "Sphere").clicked() {
                visuals.wireframe = WireframeMeshConfig::Sphere { rings: 6, segments: 12 };
            }
        });

    // Show parameters for parametric shapes
    match &mut visuals.wireframe {
        WireframeMeshConfig::Prism { sides } => {
            ui.horizontal(|ui| {
                ui.label("Sides:");
                ui.add(egui::DragValue::new(sides).range(3..=12));
            });
        }
        WireframeMeshConfig::Spiral { turns, segments } => {
            ui.horizontal(|ui| {
                ui.label("Turns:");
                ui.add(egui::DragValue::new(turns).speed(0.1).range(0.5..=5.0));
                ui.label("Segments:");
                ui.add(egui::DragValue::new(segments).range(8..=64));
            });
        }
        WireframeMeshConfig::Sphere { rings, segments } => {
            ui.horizontal(|ui| {
                ui.label("Rings:");
                ui.add(egui::DragValue::new(rings).range(2..=16));
                ui.label("Segments:");
                ui.add(egui::DragValue::new(segments).range(4..=24));
            });
        }
        _ => {}
    }

    if visuals.wireframe != WireframeMeshConfig::None {
        ui.add(egui::Slider::new(&mut visuals.wireframe_thickness, 0.001..=0.02).text("Line Thickness"));
    }

    changed
}
