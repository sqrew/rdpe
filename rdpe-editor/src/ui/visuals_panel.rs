//! UI panel for visual settings

use eframe::egui;
use crate::config::{
    BlendModeConfig, ColorMappingConfig, ColorMode, PaletteConfig, ParticleShapeConfig,
    SimConfig, WireframeMeshConfig,
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
    egui::ComboBox::from_label("Palette")
        .selected_text(format!("{:?}", visuals.palette))
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
        });

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

    // Connections
    ui.checkbox(&mut visuals.connections_enabled, "Connections");
    if visuals.connections_enabled {
        ui.add(egui::Slider::new(&mut visuals.connections_radius, 0.01..=0.5).text("Connection Radius"));
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
        .selected_text(format!("{:?}", visuals.wireframe))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::None, "None");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Tetrahedron, "Tetrahedron");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Cube, "Cube");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Octahedron, "Octahedron");
            ui.selectable_value(&mut visuals.wireframe, WireframeMeshConfig::Icosahedron, "Icosahedron");
        });

    if visuals.wireframe != WireframeMeshConfig::None {
        ui.add(egui::Slider::new(&mut visuals.wireframe_thickness, 0.001..=0.02).text("Line Thickness"));
    }

    changed
}
