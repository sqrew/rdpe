//! UI panel for post-processing effects

use crate::config::{PostProcessConfig, PostProcessEffect};
use eframe::egui;

pub fn render_post_process_panel(ui: &mut egui::Ui, config: &mut PostProcessConfig) -> bool {
    let mut changed = false;

    ui.heading("Post-Processing");

    ui.label(
        egui::RichText::new("Stack multiple fullscreen effects")
            .small()
            .weak(),
    );

    // Enable toggle
    if ui
        .checkbox(&mut config.enabled, "Enable Post-Processing")
        .changed()
    {
        changed = true;
    }

    if !config.enabled {
        ui.label(egui::RichText::new("Post-processing is disabled").weak());
        return changed;
    }

    ui.separator();

    // Add effect menu
    ui.horizontal(|ui| {
        ui.label("Add Effect:");
        ui.menu_button("+ Add...", |ui| {
            ui.set_min_width(150.0);

            ui.label(egui::RichText::new("Glow & Light").small().strong());
            if ui.button("Bloom").clicked() {
                config.effects.push(PostProcessEffect::default_bloom());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Edge Glow").clicked() {
                config.effects.push(PostProcessEffect::default_edge_glow());
                changed = true;
                ui.close_menu();
            }

            ui.separator();
            ui.label(egui::RichText::new("Color").small().strong());
            if ui.button("Grayscale").clicked() {
                config.effects.push(PostProcessEffect::default_grayscale());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Sepia").clicked() {
                config.effects.push(PostProcessEffect::default_sepia());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Posterize").clicked() {
                config.effects.push(PostProcessEffect::default_posterize());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Invert").clicked() {
                config.effects.push(PostProcessEffect::default_invert());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Tint").clicked() {
                config.effects.push(PostProcessEffect::default_tint());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Duotone").clicked() {
                config.effects.push(PostProcessEffect::default_duotone());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Thermal").clicked() {
                config.effects.push(PostProcessEffect::default_thermal());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Color Adjust").clicked() {
                config.effects.push(PostProcessEffect::default_color_adjust());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Hue Shift").clicked() {
                config.effects.push(PostProcessEffect::default_hue_shift());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Channel Mixer").clicked() {
                config.effects.push(PostProcessEffect::default_channel_mixer());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Exposure").clicked() {
                config.effects.push(PostProcessEffect::default_exposure());
                changed = true;
                ui.close_menu();
            }

            ui.separator();
            ui.label(egui::RichText::new("Distortion").small().strong());
            if ui.button("Chromatic Aberration").clicked() {
                config.effects.push(PostProcessEffect::default_chromatic_aberration());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Barrel Distortion").clicked() {
                config.effects.push(PostProcessEffect::default_barrel_distortion());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Ripple").clicked() {
                config.effects.push(PostProcessEffect::default_ripple());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Radial Blur").clicked() {
                config.effects.push(PostProcessEffect::default_radial_blur());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Kaleidoscope").clicked() {
                config.effects.push(PostProcessEffect::default_kaleidoscope());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Emboss").clicked() {
                config.effects.push(PostProcessEffect::default_emboss());
                changed = true;
                ui.close_menu();
            }

            ui.separator();
            ui.label(egui::RichText::new("Film & Retro").small().strong());
            if ui.button("CRT").clicked() {
                config.effects.push(PostProcessEffect::default_crt());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Film Grain").clicked() {
                config.effects.push(PostProcessEffect::default_film_grain());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Pixelate").clicked() {
                config.effects.push(PostProcessEffect::default_pixelate());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Night Vision").clicked() {
                config.effects.push(PostProcessEffect::default_night_vision());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Glitch").clicked() {
                config.effects.push(PostProcessEffect::default_glitch());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Dithering").clicked() {
                config.effects.push(PostProcessEffect::default_dithering());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Halftone").clicked() {
                config.effects.push(PostProcessEffect::default_halftone());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Scanline Interlace").clicked() {
                config.effects.push(PostProcessEffect::default_scanline_interlace());
                changed = true;
                ui.close_menu();
            }
            if ui.button("VHS").clicked() {
                config.effects.push(PostProcessEffect::default_vhs());
                changed = true;
                ui.close_menu();
            }

            ui.separator();
            ui.label(egui::RichText::new("Enhancement").small().strong());
            if ui.button("Sharpen").clicked() {
                config.effects.push(PostProcessEffect::default_sharpen());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Blur").clicked() {
                config.effects.push(PostProcessEffect::default_blur());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Vignette").clicked() {
                config.effects.push(PostProcessEffect::default_vignette());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Sobel Outline").clicked() {
                config.effects.push(PostProcessEffect::default_sobel_outline());
                changed = true;
                ui.close_menu();
            }
            if ui.button("Letterbox").clicked() {
                config.effects.push(PostProcessEffect::default_letterbox());
                changed = true;
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Custom (WGSL)").clicked() {
                config.effects.push(PostProcessEffect::default_custom());
                changed = true;
                ui.close_menu();
            }
        });
    });

    if config.effects.is_empty() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("No effects added. Use the dropdown above to add effects.")
                .weak()
                .italics(),
        );
        return changed;
    }

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(format!("{} effect(s) in chain", config.effects.len()))
            .small()
            .weak(),
    );

    ui.separator();

    // Effect list
    let mut to_remove: Option<usize> = None;
    let mut to_move_up: Option<usize> = None;
    let mut to_move_down: Option<usize> = None;
    let effect_count = config.effects.len();

    for (i, effect) in config.effects.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            egui::CollapsingHeader::new(format!("{}. {}", i + 1, effect.name()))
                .default_open(true)
                .show(ui, |ui| {
                    // Description
                    ui.label(egui::RichText::new(effect.description()).small().weak());

                    ui.add_space(4.0);

                    // Controls row
                    ui.horizontal(|ui| {
                        if ui.small_button("🗑 Remove").clicked() {
                            to_remove = Some(i);
                        }
                        if i > 0 && ui.small_button("⬆ Up").clicked() {
                            to_move_up = Some(i);
                        }
                        if i < effect_count - 1 && ui.small_button("⬇ Down").clicked() {
                            to_move_down = Some(i);
                        }
                    });

                    ui.separator();

                    // Effect-specific parameters
                    changed |= render_effect_params(ui, effect);
                });
        });

        ui.add_space(2.0);
    }

    // Handle removals and moves
    if let Some(idx) = to_remove {
        config.effects.remove(idx);
        changed = true;
    }
    if let Some(idx) = to_move_up {
        config.effects.swap(idx, idx - 1);
        changed = true;
    }
    if let Some(idx) = to_move_down {
        config.effects.swap(idx, idx + 1);
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

/// Render parameter controls for a specific effect
fn render_effect_params(ui: &mut egui::Ui, effect: &mut PostProcessEffect) -> bool {
    let mut changed = false;

    match effect {
        PostProcessEffect::Bloom { intensity, threshold, radius } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(threshold, 0.0..=1.0).text("Threshold"))
                .on_hover_text("Brightness threshold for bloom").changed();
            changed |= ui.add(egui::Slider::new(radius, 0.001..=0.01).text("Radius"))
                .on_hover_text("Blur radius").changed();
        }

        PostProcessEffect::ChromaticAberration { intensity, radial } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.checkbox(radial, "Radial").on_hover_text("Aberration radiates from center").changed();
        }

        PostProcessEffect::Vignette { intensity, softness } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(softness, 0.0..=1.0).text("Softness")).changed();
        }

        PostProcessEffect::CRT { intensity, scanline_count } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(scanline_count, 100.0..=600.0).text("Scanlines")).changed();
        }

        PostProcessEffect::FilmGrain { intensity, size } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(size, 0.5..=4.0).text("Grain Size")).changed();
        }

        PostProcessEffect::Pixelate { block_size } => {
            changed |= ui.add(egui::Slider::new(block_size, 1.0..=16.0).text("Block Size")).changed();
        }

        PostProcessEffect::Grayscale { intensity } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
        }

        PostProcessEffect::Sepia { intensity } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
        }

        PostProcessEffect::Posterize { levels } => {
            changed |= ui.add(egui::Slider::new(levels, 2.0..=16.0).text("Color Levels")).changed();
        }

        PostProcessEffect::Sharpen { intensity } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
        }

        PostProcessEffect::Blur { radius } => {
            changed |= ui.add(egui::Slider::new(radius, 0.5..=8.0).text("Radius")).changed();
        }

        PostProcessEffect::EdgeGlow { intensity, threshold, color } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(threshold, 0.0..=1.0).text("Threshold")).changed();
            ui.horizontal(|ui| {
                ui.label("Glow Color:");
                let mut rgb = egui::Color32::from_rgb(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut rgb).changed() {
                    color[0] = rgb.r() as f32 / 255.0;
                    color[1] = rgb.g() as f32 / 255.0;
                    color[2] = rgb.b() as f32 / 255.0;
                    changed = true;
                }
            });
        }

        PostProcessEffect::BarrelDistortion { intensity } => {
            changed |= ui.add(egui::Slider::new(intensity, -1.0..=1.0).text("Distortion"))
                .on_hover_text("Positive = barrel, Negative = pincushion").changed();
        }

        PostProcessEffect::Ripple { intensity, frequency, speed } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=0.1).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(frequency, 5.0..=50.0).text("Frequency")).changed();
            changed |= ui.add(egui::Slider::new(speed, 0.5..=10.0).text("Speed")).changed();
        }

        PostProcessEffect::Glitch { intensity, block_size } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(block_size, 0.01..=0.2).text("Block Size")).changed();
        }

        PostProcessEffect::NightVision { intensity } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
        }

        PostProcessEffect::Invert { intensity } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
        }

        PostProcessEffect::Tint { color, intensity } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
            ui.horizontal(|ui| {
                ui.label("Tint Color:");
                let mut rgb = egui::Color32::from_rgb(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut rgb).changed() {
                    color[0] = rgb.r() as f32 / 255.0;
                    color[1] = rgb.g() as f32 / 255.0;
                    color[2] = rgb.b() as f32 / 255.0;
                    changed = true;
                }
            });
        }

        PostProcessEffect::Custom { code } => {
            ui.label(
                egui::RichText::new(
                    "Write WGSL code. Use 'current_color' variable.\n\
                     Available: uv, pp_time, pp_resolution, input_texture, input_sampler",
                )
                .small()
                .weak(),
            );

            let editor = egui::TextEdit::multiline(code)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(8);

            if ui.add(editor).changed() {
                changed = true;
            }
        }

        PostProcessEffect::Dithering { intensity, scale } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(scale, 0.5..=4.0).text("Scale"))
                .on_hover_text("Size of dither pattern").changed();
        }

        PostProcessEffect::Halftone { scale, angle } => {
            changed |= ui.add(egui::Slider::new(scale, 2.0..=16.0).text("Dot Size")).changed();
            changed |= ui.add(egui::Slider::new(angle, 0.0..=90.0).text("Angle"))
                .on_hover_text("Rotation of dot pattern in degrees").changed();
        }

        PostProcessEffect::ScanlineInterlace { intensity, speed } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(speed, 0.0..=10.0).text("Speed"))
                .on_hover_text("How fast the interlace pattern moves").changed();
        }

        PostProcessEffect::RadialBlur { intensity, center_x, center_y } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(center_x, 0.0..=1.0).text("Center X")).changed();
            changed |= ui.add(egui::Slider::new(center_y, 0.0..=1.0).text("Center Y")).changed();
        }

        PostProcessEffect::Kaleidoscope { segments, rotation } => {
            changed |= ui.add(egui::Slider::new(segments, 2.0..=16.0).text("Segments"))
                .on_hover_text("Number of mirror segments").changed();
            changed |= ui.add(egui::Slider::new(rotation, 0.0..=360.0).text("Rotation"))
                .on_hover_text("Pattern rotation in degrees").changed();
        }

        PostProcessEffect::Emboss { intensity, angle } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=3.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(angle, 0.0..=360.0).text("Light Angle"))
                .on_hover_text("Direction of emboss light in degrees").changed();
        }

        PostProcessEffect::Duotone { color1, color2 } => {
            ui.horizontal(|ui| {
                ui.label("Dark Color:");
                let mut rgb1 = egui::Color32::from_rgb(
                    (color1[0] * 255.0) as u8,
                    (color1[1] * 255.0) as u8,
                    (color1[2] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut rgb1).changed() {
                    color1[0] = rgb1.r() as f32 / 255.0;
                    color1[1] = rgb1.g() as f32 / 255.0;
                    color1[2] = rgb1.b() as f32 / 255.0;
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Light Color:");
                let mut rgb2 = egui::Color32::from_rgb(
                    (color2[0] * 255.0) as u8,
                    (color2[1] * 255.0) as u8,
                    (color2[2] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut rgb2).changed() {
                    color2[0] = rgb2.r() as f32 / 255.0;
                    color2[1] = rgb2.g() as f32 / 255.0;
                    color2[2] = rgb2.b() as f32 / 255.0;
                    changed = true;
                }
            });
        }

        PostProcessEffect::Thermal { intensity } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=1.0).text("Intensity")).changed();
        }

        PostProcessEffect::ColorAdjust { brightness, contrast, saturation } => {
            changed |= ui.add(egui::Slider::new(brightness, -1.0..=1.0).text("Brightness")).changed();
            changed |= ui.add(egui::Slider::new(contrast, 0.0..=3.0).text("Contrast")).changed();
            changed |= ui.add(egui::Slider::new(saturation, 0.0..=3.0).text("Saturation")).changed();
        }

        PostProcessEffect::SobelOutline { intensity, threshold, color } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(threshold, 0.0..=1.0).text("Threshold"))
                .on_hover_text("Edge detection threshold").changed();
            ui.horizontal(|ui| {
                ui.label("Outline Color:");
                let mut rgb = egui::Color32::from_rgb(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut rgb).changed() {
                    color[0] = rgb.r() as f32 / 255.0;
                    color[1] = rgb.g() as f32 / 255.0;
                    color[2] = rgb.b() as f32 / 255.0;
                    changed = true;
                }
            });
        }

        PostProcessEffect::HueShift { shift } => {
            changed |= ui.add(egui::Slider::new(shift, -1.0..=1.0).text("Hue Shift"))
                .on_hover_text("-1 to 1 rotates through the full color wheel").changed();
        }

        PostProcessEffect::ChannelMixer { red_source, green_source, blue_source } => {
            ui.label("Red channel from:");
            ui.horizontal(|ui| {
                changed |= ui.add(egui::DragValue::new(&mut red_source[0]).speed(0.01).prefix("R:")).changed();
                changed |= ui.add(egui::DragValue::new(&mut red_source[1]).speed(0.01).prefix("G:")).changed();
                changed |= ui.add(egui::DragValue::new(&mut red_source[2]).speed(0.01).prefix("B:")).changed();
            });
            ui.label("Green channel from:");
            ui.horizontal(|ui| {
                changed |= ui.add(egui::DragValue::new(&mut green_source[0]).speed(0.01).prefix("R:")).changed();
                changed |= ui.add(egui::DragValue::new(&mut green_source[1]).speed(0.01).prefix("G:")).changed();
                changed |= ui.add(egui::DragValue::new(&mut green_source[2]).speed(0.01).prefix("B:")).changed();
            });
            ui.label("Blue channel from:");
            ui.horizontal(|ui| {
                changed |= ui.add(egui::DragValue::new(&mut blue_source[0]).speed(0.01).prefix("R:")).changed();
                changed |= ui.add(egui::DragValue::new(&mut blue_source[1]).speed(0.01).prefix("G:")).changed();
                changed |= ui.add(egui::DragValue::new(&mut blue_source[2]).speed(0.01).prefix("B:")).changed();
            });
        }

        PostProcessEffect::Exposure { exposure } => {
            changed |= ui.add(egui::Slider::new(exposure, -3.0..=3.0).text("Exposure"))
                .on_hover_text("Stops of exposure adjustment").changed();
        }

        PostProcessEffect::Letterbox { aspect_ratio, bar_color } => {
            changed |= ui.add(egui::Slider::new(aspect_ratio, 1.0..=3.0).text("Aspect Ratio"))
                .on_hover_text("2.35 = Cinemascope, 1.85 = Widescreen, 1.33 = 4:3").changed();
            ui.horizontal(|ui| {
                ui.label("Bar Color:");
                let mut rgb = egui::Color32::from_rgb(
                    (bar_color[0] * 255.0) as u8,
                    (bar_color[1] * 255.0) as u8,
                    (bar_color[2] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut rgb).changed() {
                    bar_color[0] = rgb.r() as f32 / 255.0;
                    bar_color[1] = rgb.g() as f32 / 255.0;
                    bar_color[2] = rgb.b() as f32 / 255.0;
                    changed = true;
                }
            });
        }

        PostProcessEffect::VHS { intensity, tracking_noise } => {
            changed |= ui.add(egui::Slider::new(intensity, 0.0..=2.0).text("Intensity")).changed();
            changed |= ui.add(egui::Slider::new(tracking_noise, 0.0..=2.0).text("Tracking Noise"))
                .on_hover_text("Horizontal tracking errors").changed();
        }
    }

    changed
}
