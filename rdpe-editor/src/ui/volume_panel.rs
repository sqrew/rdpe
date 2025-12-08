//! Volume rendering configuration panel

use crate::config::{PaletteConfig, VolumeRenderConfig};
use egui::Ui;

pub fn render_volume_panel(
    ui: &mut Ui,
    volume: &mut VolumeRenderConfig,
    num_fields: usize,
) -> bool {
    let mut changed = false;

    ui.heading("Volume Rendering");

    ui.label(
        egui::RichText::new("Render 3D fields as volumetric fog using ray marching")
            .small()
            .weak(),
    );

    // Enable toggle
    if ui.checkbox(&mut volume.enabled, "Enable Volume Rendering").changed() {
        changed = true;
    }

    if !volume.enabled {
        ui.label(egui::RichText::new("Volume rendering is disabled").weak());
        return changed;
    }

    if num_fields == 0 {
        ui.label(
            egui::RichText::new("No fields defined. Add a field first to use volume rendering.")
                .weak()
                .color(egui::Color32::YELLOW),
        );
        return changed;
    }

    ui.separator();

    // Field selection
    ui.horizontal(|ui| {
        ui.label("Field:");
        let field_idx = volume.field_index as usize;
        if egui::ComboBox::from_id_salt("volume_field")
            .selected_text(format!("Field {}", field_idx))
            .show_index(ui, &mut (volume.field_index as usize), num_fields, |i| {
                format!("Field {}", i)
            })
            .changed()
        {
            volume.field_index = field_idx as u32;
            changed = true;
        }
    });

    // Ray march steps
    let mut steps = volume.steps as i32;
    if ui
        .add(egui::Slider::new(&mut steps, 16..=256).text("Ray Steps"))
        .on_hover_text("Number of ray march steps. Higher = better quality, slower.")
        .changed()
    {
        volume.steps = steps as u32;
        changed = true;
    }

    // Density scale
    changed |= ui
        .add(
            egui::Slider::new(&mut volume.density_scale, 0.1..=20.0)
                .text("Density Scale")
                .logarithmic(true),
        )
        .on_hover_text("How opaque the volume appears")
        .changed();

    // Threshold
    changed |= ui
        .add(
            egui::Slider::new(&mut volume.threshold, 0.0..=0.1)
                .text("Threshold")
                .fixed_decimals(3),
        )
        .on_hover_text("Minimum density to render (values below are transparent)")
        .changed();

    // Additive blending
    if ui
        .checkbox(&mut volume.additive, "Additive (Glow)")
        .on_hover_text("Use additive blending for glow effect, or alpha blending for solid fog")
        .changed()
    {
        changed = true;
    }

    ui.separator();

    // Palette selection
    ui.label("Color Palette:");
    let palette_variants = PaletteConfig::variants();
    let mut palette_idx = match volume.palette {
        PaletteConfig::None => 0,
        PaletteConfig::Viridis => 1,
        PaletteConfig::Magma => 2,
        PaletteConfig::Plasma => 3,
        PaletteConfig::Inferno => 4,
        PaletteConfig::Rainbow => 5,
        PaletteConfig::Sunset => 6,
        PaletteConfig::Ocean => 7,
        PaletteConfig::Fire => 8,
        PaletteConfig::Ice => 9,
        PaletteConfig::Neon => 10,
        PaletteConfig::Forest => 11,
        PaletteConfig::Grayscale => 12,
    };

    if egui::ComboBox::from_id_salt("volume_palette")
        .selected_text(palette_variants[palette_idx])
        .show_index(ui, &mut palette_idx, palette_variants.len(), |i| {
            palette_variants[i]
        })
        .changed()
    {
        volume.palette = match palette_idx {
            0 => PaletteConfig::None,
            1 => PaletteConfig::Viridis,
            2 => PaletteConfig::Magma,
            3 => PaletteConfig::Plasma,
            4 => PaletteConfig::Inferno,
            5 => PaletteConfig::Rainbow,
            6 => PaletteConfig::Sunset,
            7 => PaletteConfig::Ocean,
            8 => PaletteConfig::Fire,
            9 => PaletteConfig::Ice,
            10 => PaletteConfig::Neon,
            11 => PaletteConfig::Forest,
            12 => PaletteConfig::Grayscale,
            _ => PaletteConfig::Inferno,
        };
        changed = true;
    }

    // Info about rebuild requirement
    if changed {
        ui.separator();
        ui.label(
            egui::RichText::new("Changes require rebuild (Ctrl+R)")
                .small()
                .color(egui::Color32::YELLOW),
        );
    }

    changed
}
