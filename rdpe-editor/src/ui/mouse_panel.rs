//! UI panel for mouse interaction settings

use eframe::egui;
use crate::config::{MouseConfig, MousePower};

pub fn render_mouse_panel(ui: &mut egui::Ui, mouse: &mut MouseConfig) -> bool {
    let mut changed = false;

    ui.heading("Mouse Power");

    // Power selection
    let variants = MousePower::variants();
    let mut power_idx = mouse.power.to_index();

    if egui::ComboBox::from_label("Power")
        .selected_text(variants[power_idx])
        .show_index(ui, &mut power_idx, variants.len(), |i| variants[i])
        .changed()
    {
        mouse.power = MousePower::from_index(power_idx);
        changed = true;
    }

    // Show description of selected power
    let description = match mouse.power {
        MousePower::None => "No mouse interaction",
        MousePower::Attract => "Shift+Click to pull particles toward cursor",
        MousePower::Repel => "Shift+Click to push particles away from cursor",
        MousePower::Vortex => "Shift+Click to swirl particles around cursor",
        MousePower::Explode => "Shift+Click to burst particles outward",
        MousePower::GravityWell => "Shift+Click to create strong point gravity",
        MousePower::Paint => "Shift+Click to color particles near cursor",
        MousePower::Turbulence => "Shift+Click to add chaos near cursor",
        MousePower::Freeze => "Shift+Click to slow/stop particles",
        MousePower::Kill => "Shift+Click to destroy particles in radius",
        MousePower::Spawn => "Shift+Click to spawn particles at cursor",
        MousePower::BlackHole => "Shift+Click to suck in and destroy particles",
        MousePower::Orbit => "Shift+Click to make particles orbit cursor",
        MousePower::Scatter => "Shift+Click to scatter particles randomly",
        MousePower::Wind => "Shift+Click to blow particles outward horizontally",
        MousePower::Pulse => "Shift+Click to emit rhythmic expanding waves",
        MousePower::Repulsor => "Shift+Click to push particles in a ring shape",
        MousePower::SpiralIn => "Shift+Click to spiral particles inward like a drain",
        MousePower::RandomVelocity => "Shift+Click to randomize particle velocities",
    };
    ui.label(egui::RichText::new(description).small().weak());

    if mouse.power != MousePower::None {
        ui.label(egui::RichText::new("Hold Shift + Left Mouse in viewport").italics().weak());
    }

    ui.add_space(8.0);

    // Only show parameters if a power is selected
    if mouse.power != MousePower::None {
        ui.separator();
        ui.heading("Parameters");

        // Radius (world space units)
        changed |= ui
            .add(egui::Slider::new(&mut mouse.radius, 0.1..=2.0).text("Radius"))
            .changed();

        // Strength
        changed |= ui
            .add(egui::Slider::new(&mut mouse.strength, 0.1..=20.0).text("Strength").logarithmic(true))
            .changed();

        // Color (only for Paint and Spawn)
        if matches!(mouse.power, MousePower::Paint | MousePower::Spawn) {
            ui.horizontal(|ui| {
                ui.label("Color:");
                changed |= ui.color_edit_button_rgb(&mut mouse.color).changed();
            });
        }

        ui.add_space(8.0);

        // Quick presets
        ui.separator();
        ui.label("Quick Presets:");
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Gentle").clicked() {
                mouse.radius = 0.3;
                mouse.strength = 2.0;
                changed = true;
            }
            if ui.small_button("Medium").clicked() {
                mouse.radius = 0.5;
                mouse.strength = 5.0;
                changed = true;
            }
            if ui.small_button("Strong").clicked() {
                mouse.radius = 0.8;
                mouse.strength = 10.0;
                changed = true;
            }
            if ui.small_button("Massive").clicked() {
                mouse.radius = 1.5;
                mouse.strength = 20.0;
                changed = true;
            }
        });
    }

    changed
}
