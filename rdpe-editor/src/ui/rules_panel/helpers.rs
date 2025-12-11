//! Helper UI functions for rule rendering

use crate::config::Falloff;
use egui::Ui;

/// Renders a vec3 input widget with x, y, z drag values
pub(super) fn render_vec3(ui: &mut Ui, label: &str, v: &mut [f32; 3]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(format!("{}:", label));
        changed |= ui
            .add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("x:"))
            .changed();
        changed |= ui
            .add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("y:"))
            .changed();
        changed |= ui
            .add(egui::DragValue::new(&mut v[2]).speed(0.01).prefix("z:"))
            .changed();
    });
    changed
}

/// Renders a falloff selector combo box
pub(super) fn render_falloff(ui: &mut Ui, falloff: &mut Falloff) -> bool {
    let variants = Falloff::variants();
    let mut idx = match falloff {
        Falloff::Constant => 0,
        Falloff::Linear => 1,
        Falloff::Inverse => 2,
        Falloff::InverseSquare => 3,
        Falloff::Smooth => 4,
    };

    if egui::ComboBox::from_label("Falloff")
        .show_index(ui, &mut idx, variants.len(), |i| variants[i])
        .changed()
    {
        *falloff = match idx {
            0 => Falloff::Constant,
            1 => Falloff::Linear,
            2 => Falloff::Inverse,
            3 => Falloff::InverseSquare,
            _ => Falloff::Smooth,
        };
        return true;
    }
    false
}
