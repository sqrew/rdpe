//! Rule parameter renderers

use super::helpers::{render_falloff, render_vec3};
use crate::config::*;
use egui::Ui;

/// Renders the parameter UI for a given rule configuration
pub(super) fn render_rule_params(ui: &mut Ui, rule: &mut RuleConfig) -> bool {
    let mut changed = false;

    match rule {
        // === Basic Forces ===
        RuleConfig::Gravity(g) => {
            changed |= ui
                .add(egui::Slider::new(g, -10.0..=10.0).text("Strength"))
                .changed();
        }
        RuleConfig::Drag(d) => {
            changed |= ui
                .add(egui::Slider::new(d, 0.0..=10.0).text("Drag"))
                .changed();
        }
        RuleConfig::Acceleration { direction } => {
            changed |= render_vec3(ui, "Direction", direction);
        }

        // === Boundaries ===
        RuleConfig::BounceWalls | RuleConfig::WrapWalls => {
            ui.label("No parameters");
        }

        // === Point Forces ===
        RuleConfig::AttractTo { point, strength } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
        }
        RuleConfig::RepelFrom {
            point,
            strength,
            radius,
        } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=10.0).text("Radius"))
                .changed();
        }
        RuleConfig::PointGravity {
            point,
            strength,
            softening,
        } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui
                .add(egui::Slider::new(strength, -10.0..=10.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(softening, 0.001..=1.0).text("Softening"))
                .changed();
        }
        RuleConfig::Orbit { center, strength } => {
            changed |= render_vec3(ui, "Center", center);
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
        }
        RuleConfig::Spring {
            anchor,
            stiffness,
            damping,
        } => {
            changed |= render_vec3(ui, "Anchor", anchor);
            changed |= ui
                .add(egui::Slider::new(stiffness, 0.0..=10.0).text("Stiffness"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(damping, 0.0..=2.0).text("Damping"))
                .changed();
        }
        RuleConfig::Radial {
            point,
            strength,
            radius,
            falloff,
        } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui
                .add(egui::Slider::new(strength, -10.0..=10.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=5.0).text("Radius"))
                .changed();
            changed |= render_falloff(ui, falloff);
        }
        RuleConfig::Vortex {
            center,
            axis,
            strength,
        } => {
            changed |= render_vec3(ui, "Center", center);
            changed |= render_vec3(ui, "Axis", axis);
            changed |= ui
                .add(egui::Slider::new(strength, -10.0..=10.0).text("Strength"))
                .changed();
        }
        RuleConfig::Pulse {
            point,
            strength,
            frequency,
            radius,
        } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=5.0).text("Radius"))
                .changed();
        }

        // === Noise & Flow ===
        RuleConfig::Turbulence { scale, strength } => {
            changed |= ui
                .add(egui::Slider::new(scale, 0.1..=10.0).text("Scale"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
        }
        RuleConfig::Curl { scale, strength } => {
            changed |= ui
                .add(egui::Slider::new(scale, 0.1..=10.0).text("Scale"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
        }
        RuleConfig::Wind {
            direction,
            strength,
            turbulence,
        } => {
            changed |= render_vec3(ui, "Direction", direction);
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(turbulence, 0.0..=1.0).text("Turbulence"))
                .changed();
        }
        RuleConfig::PositionNoise {
            scale,
            strength,
            speed,
        } => {
            changed |= ui
                .add(egui::Slider::new(scale, 0.1..=10.0).text("Scale"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=1.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(speed, 0.0..=5.0).text("Speed"))
                .changed();
        }

        // === Steering ===
        RuleConfig::Seek {
            target,
            max_speed,
            max_force,
        } => {
            changed |= render_vec3(ui, "Target", target);
            changed |= ui
                .add(egui::Slider::new(max_speed, 0.1..=5.0).text("Max Speed"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max_force, 0.1..=5.0).text("Max Force"))
                .changed();
        }
        RuleConfig::Flee {
            target,
            max_speed,
            max_force,
            panic_radius,
        } => {
            changed |= render_vec3(ui, "Target", target);
            changed |= ui
                .add(egui::Slider::new(max_speed, 0.1..=5.0).text("Max Speed"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max_force, 0.1..=5.0).text("Max Force"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(panic_radius, 0.1..=5.0).text("Panic Radius"))
                .changed();
        }
        RuleConfig::Arrive {
            target,
            max_speed,
            max_force,
            slowing_radius,
        } => {
            changed |= render_vec3(ui, "Target", target);
            changed |= ui
                .add(egui::Slider::new(max_speed, 0.1..=5.0).text("Max Speed"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max_force, 0.1..=5.0).text("Max Force"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(slowing_radius, 0.1..=5.0).text("Slowing Radius"))
                .changed();
        }
        RuleConfig::Wander {
            strength,
            frequency,
        } => {
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"))
                .changed();
        }

        // === Flocking ===
        RuleConfig::Separate { radius, strength } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
        }
        RuleConfig::Cohere { radius, strength } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
        }
        RuleConfig::Align { radius, strength } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
        }
        RuleConfig::Flock {
            radius,
            separation,
            cohesion,
            alignment,
        } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(separation, 0.0..=5.0).text("Separation"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(cohesion, 0.0..=5.0).text("Cohesion"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(alignment, 0.0..=5.0).text("Alignment"))
                .changed();
        }
        RuleConfig::Avoid { radius, strength } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
        }

        // === Physics ===
        RuleConfig::Collide {
            radius,
            restitution,
        } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.001..=0.5).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(restitution, 0.0..=1.0).text("Restitution"))
                .changed();
        }
        RuleConfig::NBodyGravity {
            strength,
            softening,
            radius,
        } => {
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(softening, 0.001..=0.5).text("Softening"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=5.0).text("Radius"))
                .changed();
        }
        RuleConfig::LennardJones {
            epsilon,
            sigma,
            cutoff,
        } => {
            changed |= ui
                .add(egui::Slider::new(epsilon, 0.0..=2.0).text("Epsilon"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(sigma, 0.01..=0.5).text("Sigma"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(cutoff, 0.01..=1.0).text("Cutoff"))
                .changed();
        }
        RuleConfig::Viscosity { radius, strength } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=0.5).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=2.0).text("Strength"))
                .changed();
        }
        RuleConfig::Pressure {
            radius,
            strength,
            target_density,
        } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=0.5).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(target_density, 1.0..=50.0).text("Target Density"))
                .changed();
        }
        RuleConfig::SurfaceTension {
            radius,
            strength,
            threshold,
        } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=0.5).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(threshold, 1.0..=20.0).text("Threshold"))
                .changed();
        }
        RuleConfig::Magnetism {
            radius,
            strength,
            same_repel,
        } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
            changed |= ui.checkbox(same_repel, "Same Polarity Repels").changed();
        }

        // === Constraints ===
        RuleConfig::SpeedLimit { min, max } => {
            changed |= ui
                .add(egui::Slider::new(min, 0.0..=5.0).text("Min"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max, 0.0..=10.0).text("Max"))
                .changed();
        }
        RuleConfig::Buoyancy { surface_y, density } => {
            changed |= ui
                .add(egui::Slider::new(surface_y, -2.0..=2.0).text("Surface Y"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(density, 0.0..=2.0).text("Density"))
                .changed();
        }
        RuleConfig::Friction {
            ground_y,
            strength,
            threshold,
        } => {
            changed |= ui
                .add(egui::Slider::new(ground_y, -2.0..=2.0).text("Ground Y"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=1.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(threshold, 0.0..=0.2).text("Threshold"))
                .changed();
        }

        // === Lifecycle ===
        RuleConfig::Age => {
            ui.label("Increments particle age each frame");
        }
        RuleConfig::Lifetime(t) => {
            changed |= ui
                .add(egui::Slider::new(t, 0.1..=30.0).text("Lifetime"))
                .changed();
        }
        RuleConfig::FadeOut(t) => {
            changed |= ui
                .add(egui::Slider::new(t, 0.1..=30.0).text("Duration"))
                .changed();
        }
        RuleConfig::ShrinkOut(t) => {
            changed |= ui
                .add(egui::Slider::new(t, 0.1..=30.0).text("Duration"))
                .changed();
        }
        RuleConfig::ColorOverLife {
            start,
            end,
            duration,
        } => {
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
            changed |= ui
                .add(egui::Slider::new(duration, 0.1..=30.0).text("Duration"))
                .changed();
        }
        RuleConfig::ColorBySpeed {
            slow_color,
            fast_color,
            max_speed,
        } => {
            ui.horizontal(|ui| {
                ui.label("Slow:");
                if ui.color_edit_button_rgb(slow_color).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Fast:");
                if ui.color_edit_button_rgb(fast_color).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(max_speed, 0.1..=10.0).text("Max Speed"))
                .changed();
        }
        RuleConfig::ColorByAge {
            young_color,
            old_color,
            max_age,
        } => {
            ui.horizontal(|ui| {
                ui.label("Young:");
                if ui.color_edit_button_rgb(young_color).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Old:");
                if ui.color_edit_button_rgb(old_color).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(max_age, 0.1..=30.0).text("Max Age"))
                .changed();
        }
        RuleConfig::ScaleBySpeed {
            min_scale,
            max_scale,
            max_speed,
        } => {
            changed |= ui
                .add(egui::Slider::new(min_scale, 0.1..=2.0).text("Min Scale"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max_scale, 0.1..=5.0).text("Max Scale"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max_speed, 0.1..=10.0).text("Max Speed"))
                .changed();
        }

        // === Typed Interactions ===
        RuleConfig::Chase {
            self_type,
            target_type,
            radius,
            strength,
        } => {
            changed |= ui
                .add(egui::Slider::new(self_type, 0..=7).text("Self Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(target_type, 0..=7).text("Target Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
        }
        RuleConfig::Evade {
            self_type,
            threat_type,
            radius,
            strength,
        } => {
            changed |= ui
                .add(egui::Slider::new(self_type, 0..=7).text("Self Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(threat_type, 0..=7).text("Threat Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
        }
        RuleConfig::Convert {
            from_type,
            trigger_type,
            to_type,
            radius,
            probability,
        } => {
            changed |= ui
                .add(egui::Slider::new(from_type, 0..=7).text("From Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(trigger_type, 0..=7).text("Trigger Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(to_type, 0..=7).text("To Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(radius, 0.1..=2.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(probability, 0.0..=1.0).text("Probability"))
                .changed();
        }

        // === Events ===
        RuleConfig::Shockwave {
            origin,
            speed,
            width,
            strength,
            repeat,
        } => {
            changed |= render_vec3(ui, "Origin", origin);
            changed |= ui
                .add(egui::Slider::new(speed, 0.1..=10.0).text("Speed"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(width, 0.01..=1.0).text("Width"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(repeat, 0.1..=10.0).text("Repeat"))
                .changed();
        }
        RuleConfig::Oscillate {
            axis,
            amplitude,
            frequency,
            spatial_scale,
        } => {
            changed |= render_vec3(ui, "Axis", axis);
            changed |= ui
                .add(egui::Slider::new(amplitude, 0.0..=1.0).text("Amplitude"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(spatial_scale, 0.1..=10.0).text("Spatial Scale"))
                .changed();
        }
        RuleConfig::RespawnBelow {
            threshold_y,
            spawn_y,
            reset_velocity,
        } => {
            changed |= ui
                .add(egui::Slider::new(threshold_y, -5.0..=0.0).text("Threshold Y"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(spawn_y, 0.0..=5.0).text("Spawn Y"))
                .changed();
            changed |= ui.checkbox(reset_velocity, "Reset Velocity").changed();
        }

        // === Conditional ===
        RuleConfig::Maybe {
            probability,
            action,
        } => {
            changed |= ui
                .add(egui::Slider::new(probability, 0.0..=1.0).text("Probability"))
                .changed();
            ui.label("Action (WGSL):");
            if ui.text_edit_multiline(action).changed() {
                changed = true;
            }
        }
        RuleConfig::Trigger { condition, action } => {
            ui.label("Condition (WGSL):");
            if ui.text_edit_singleline(condition).changed() {
                changed = true;
            }
            ui.label("Action (WGSL):");
            if ui.text_edit_multiline(action).changed() {
                changed = true;
            }
        }

        // === Custom ===
        RuleConfig::Custom { code } => {
            ui.label("WGSL Code:");
            if ui.text_edit_multiline(code).changed() {
                changed = true;
            }
        }
        RuleConfig::NeighborCustom { code } => {
            ui.label("WGSL Code (per neighbor):");
            if ui.text_edit_multiline(code).changed() {
                changed = true;
            }
        }
        RuleConfig::OnCollision { radius, response } => {
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=0.5).text("Radius"))
                .changed();
            ui.label("Response (WGSL):");
            if ui.text_edit_multiline(response).changed() {
                changed = true;
            }
        }
        RuleConfig::CustomDynamic { code, params } => {
            ui.label("WGSL Code:");
            ui.label(
                egui::RichText::new("Access params via uniforms.rule_N_paramname")
                    .small()
                    .weak(),
            );
            if ui.text_edit_multiline(code).changed() {
                changed = true;
            }
            ui.separator();

            // Parameters with add/remove
            ui.horizontal(|ui| {
                ui.label("Parameters:");
                if ui
                    .small_button("+")
                    .on_hover_text("Add parameter")
                    .clicked()
                {
                    let new_name = format!("param_{}", params.len());
                    params.push((new_name, 1.0));
                    changed = true;
                }
            });

            let mut to_remove = None;
            for (idx, (name, value)) in params.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    // Editable name
                    let mut name_edit = name.clone();
                    if ui
                        .add(egui::TextEdit::singleline(&mut name_edit).desired_width(80.0))
                        .changed()
                    {
                        *name = name_edit;
                        changed = true;
                    }
                    ui.label("=");
                    if ui.add(egui::DragValue::new(value).speed(0.01)).changed() {
                        changed = true;
                    }
                    if ui.small_button("X").on_hover_text("Remove").clicked() {
                        to_remove = Some(idx);
                    }
                });
            }
            if let Some(idx) = to_remove {
                params.remove(idx);
                changed = true;
            }
        }
        RuleConfig::NeighborCustomDynamic { code, params } => {
            ui.label("Neighbor WGSL Code:");
            ui.label(
                egui::RichText::new(
                    "Available: neighbor_dist, neighbor_dir, neighbor_pos, neighbor_vel, other",
                )
                .small()
                .weak(),
            );
            ui.label(
                egui::RichText::new("Access params via uniforms.rule_N_paramname")
                    .small()
                    .weak(),
            );
            if ui.text_edit_multiline(code).changed() {
                changed = true;
            }
            ui.separator();

            // Parameters with add/remove
            ui.horizontal(|ui| {
                ui.label("Parameters:");
                if ui
                    .small_button("+")
                    .on_hover_text("Add parameter")
                    .clicked()
                {
                    let new_name = format!("param_{}", params.len());
                    params.push((new_name, 1.0));
                    changed = true;
                }
            });

            let mut to_remove = None;
            for (idx, (name, value)) in params.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    // Editable name
                    let mut name_edit = name.clone();
                    if ui
                        .add(egui::TextEdit::singleline(&mut name_edit).desired_width(80.0))
                        .changed()
                    {
                        *name = name_edit;
                        changed = true;
                    }
                    ui.label("=");
                    if ui.add(egui::DragValue::new(value).speed(0.01)).changed() {
                        changed = true;
                    }
                    if ui.small_button("X").on_hover_text("Remove").clicked() {
                        to_remove = Some(idx);
                    }
                });
            }
            if let Some(idx) = to_remove {
                params.remove(idx);
                changed = true;
            }
        }

        // === Event Hooks ===
        RuleConfig::OnCondition { condition, action } => {
            ui.label("Condition (WGSL):");
            if ui.text_edit_singleline(condition).changed() {
                changed = true;
            }
            ui.label("Action (WGSL):");
            if ui.text_edit_multiline(action).changed() {
                changed = true;
            }
        }
        RuleConfig::OnDeath { action } => {
            ui.label("On Death Action (WGSL):");
            if ui.text_edit_multiline(action).changed() {
                changed = true;
            }
        }
        RuleConfig::OnInterval { interval, action } => {
            changed |= ui
                .add(egui::Slider::new(interval, 0.01..=10.0).text("Interval (s)"))
                .changed();
            ui.label("Action (WGSL):");
            if ui.text_edit_multiline(action).changed() {
                changed = true;
            }
        }
        RuleConfig::OnSpawn { action } => {
            ui.label("On Spawn Action (WGSL):");
            if ui.text_edit_multiline(action).changed() {
                changed = true;
            }
        }

        // === Growth & Decay ===
        RuleConfig::Grow { rate, min, max } => {
            changed |= ui
                .add(egui::Slider::new(rate, -2.0..=2.0).text("Rate"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(min, 0.0..=1.0).text("Min Scale"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max, 0.1..=5.0).text("Max Scale"))
                .changed();
        }
        RuleConfig::Decay { field, rate } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(rate, 0.0..=5.0).text("Rate"))
                .changed();
        }
        RuleConfig::Die { condition } => {
            ui.label("Death Condition (WGSL):");
            if ui.text_edit_singleline(condition).changed() {
                changed = true;
            }
        }
        RuleConfig::DLA {
            seed_type,
            mobile_type,
            stick_radius,
            diffusion_strength,
        } => {
            changed |= ui
                .add(egui::Slider::new(seed_type, 0..=7).text("Seed Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(mobile_type, 0..=7).text("Mobile Type"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(stick_radius, 0.01..=0.5).text("Stick Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(diffusion_strength, 0.0..=2.0).text("Diffusion"))
                .changed();
        }

        // === Field Operations ===
        RuleConfig::CopyField { from, to } => {
            ui.horizontal(|ui| {
                ui.label("From:");
                if ui.text_edit_singleline(from).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("To:");
                if ui.text_edit_singleline(to).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Current { field, strength } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=5.0).text("Strength"))
                .changed();
        }

        // === Math / Signal ===
        RuleConfig::Lerp {
            field,
            target,
            rate,
        } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(target, -10.0..=10.0).text("Target"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(rate, 0.0..=10.0).text("Rate"))
                .changed();
        }
        RuleConfig::Clamp { field, min, max } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(min, -10.0..=10.0).text("Min"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max, -10.0..=10.0).text("Max"))
                .changed();
        }
        RuleConfig::Remap {
            field,
            in_min,
            in_max,
            out_min,
            out_max,
        } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(in_min, -10.0..=10.0).text("In Min"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(in_max, -10.0..=10.0).text("In Max"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(out_min, -10.0..=10.0).text("Out Min"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(out_max, -10.0..=10.0).text("Out Max"))
                .changed();
        }
        RuleConfig::Quantize { field, step } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(step, 0.01..=1.0).text("Step Size"))
                .changed();
        }
        RuleConfig::Noise {
            field,
            amplitude,
            frequency,
        } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(amplitude, 0.0..=2.0).text("Amplitude"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency"))
                .changed();
        }

        // Springs
        RuleConfig::ChainSprings {
            stiffness,
            damping,
            rest_length,
            max_stretch,
        } => {
            changed |= ui
                .add(
                    egui::Slider::new(stiffness, 1.0..=1000.0)
                        .logarithmic(true)
                        .text("Stiffness"),
                )
                .changed();
            changed |= ui
                .add(egui::Slider::new(damping, 0.0..=50.0).text("Damping"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(rest_length, 0.001..=0.5).text("Rest Length"))
                .changed();
            ui.horizontal(|ui| {
                let mut has_max = max_stretch.is_some();
                if ui.checkbox(&mut has_max, "Max Stretch").changed() {
                    if has_max {
                        *max_stretch = Some(1.5);
                    } else {
                        *max_stretch = None;
                    }
                    changed = true;
                }
                if let Some(max_s) = max_stretch {
                    changed |= ui
                        .add(egui::Slider::new(max_s, 1.0..=3.0).text(""))
                        .changed();
                }
            });
        }
        RuleConfig::RadialSprings {
            hub_stiffness,
            ring_stiffness,
            damping,
            hub_length,
            ring_length,
        } => {
            changed |= ui
                .add(
                    egui::Slider::new(hub_stiffness, 1.0..=500.0)
                        .logarithmic(true)
                        .text("Hub Stiffness"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(ring_stiffness, 1.0..=500.0)
                        .logarithmic(true)
                        .text("Ring Stiffness"),
                )
                .changed();
            changed |= ui
                .add(egui::Slider::new(damping, 0.0..=50.0).text("Damping"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(hub_length, 0.01..=1.0).text("Hub Length"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(ring_length, 0.01..=1.0).text("Ring Length"))
                .changed();
        }
        RuleConfig::BondSprings {
            bonds,
            stiffness,
            damping,
            rest_length,
            max_stretch,
        } => {
            ui.label("Bond Fields (particle field names):");
            let mut remove_idx = None;
            for (i, bond) in bonds.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui.text_edit_singleline(bond).changed() {
                        changed = true;
                    }
                    if ui.small_button("X").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                bonds.remove(idx);
                changed = true;
            }
            if ui.button("+ Add Bond Field").clicked() {
                bonds.push("bond0".into());
                changed = true;
            }
            ui.separator();
            changed |= ui
                .add(
                    egui::Slider::new(stiffness, 1.0..=1000.0)
                        .logarithmic(true)
                        .text("Stiffness"),
                )
                .changed();
            changed |= ui
                .add(egui::Slider::new(damping, 0.0..=50.0).text("Damping"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(rest_length, 0.001..=1.0).text("Rest Length"))
                .changed();
            let mut has_max = max_stretch.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut has_max, "Max Stretch").changed() {
                    if has_max && max_stretch.is_none() {
                        *max_stretch = Some(1.5);
                    } else if !has_max {
                        *max_stretch = None;
                    }
                    changed = true;
                }
                if let Some(max_s) = max_stretch {
                    changed |= ui
                        .add(egui::Slider::new(max_s, 1.0..=3.0).text(""))
                        .changed();
                }
            });
        }

        // State Machine
        RuleConfig::State { field, transitions } => {
            ui.horizontal(|ui| {
                ui.label("State Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            ui.separator();
            ui.label("Transitions (from → to when condition):");
            let mut remove_idx = None;
            for (i, (from, to, condition)) in transitions.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label("From:");
                    changed |= ui.add(egui::DragValue::new(from)).changed();
                    ui.label("To:");
                    changed |= ui.add(egui::DragValue::new(to)).changed();
                    if ui.small_button("X").clicked() {
                        remove_idx = Some(i);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Condition:");
                    if ui.text_edit_singleline(condition).changed() {
                        changed = true;
                    }
                });
                ui.separator();
            }
            if let Some(idx) = remove_idx {
                transitions.remove(idx);
                changed = true;
            }
            if ui.button("+ Add Transition").clicked() {
                transitions.push((0, 1, "p.age > 1.0".into()));
                changed = true;
            }
        }
        RuleConfig::Agent {
            state_field,
            prev_state_field,
            state_timer_field,
            states,
        } => {
            ui.horizontal(|ui| {
                ui.label("State Field:");
                if ui.text_edit_singleline(state_field).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Prev State Field:");
                if ui.text_edit_singleline(prev_state_field).changed() {
                    changed = true;
                }
            });
            let mut has_timer = state_timer_field.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut has_timer, "State Timer").changed() {
                    if has_timer && state_timer_field.is_none() {
                        *state_timer_field = Some("state_timer".into());
                    } else if !has_timer {
                        *state_timer_field = None;
                    }
                    changed = true;
                }
                if let Some(timer) = state_timer_field {
                    if ui.text_edit_singleline(timer).changed() {
                        changed = true;
                    }
                }
            });
            ui.separator();
            ui.label("States:");
            let mut remove_state_idx = None;
            for (si, state) in states.iter_mut().enumerate() {
                let state_header = state
                    .name
                    .as_ref()
                    .map(|n| format!("State {} ({})", state.id, n))
                    .unwrap_or(format!("State {}", state.id));
                egui::CollapsingHeader::new(state_header)
                    .id_salt(format!("agent_state_{}", si))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("ID:");
                            if ui.add(egui::DragValue::new(&mut state.id)).changed() {
                                changed = true;
                            }
                            if ui.small_button("X Remove State").clicked() {
                                remove_state_idx = Some(si);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            let mut name_str = state.name.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut name_str).changed() {
                                state.name = if name_str.is_empty() {
                                    None
                                } else {
                                    Some(name_str)
                                };
                                changed = true;
                            }
                        });
                        ui.collapsing("On Enter", |ui| {
                            let mut code = state.on_enter.clone().unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::multiline(&mut code)
                                        .code_editor()
                                        .desired_rows(2),
                                )
                                .changed()
                            {
                                state.on_enter = if code.is_empty() { None } else { Some(code) };
                                changed = true;
                            }
                        });
                        ui.collapsing("On Update", |ui| {
                            let mut code = state.on_update.clone().unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::multiline(&mut code)
                                        .code_editor()
                                        .desired_rows(2),
                                )
                                .changed()
                            {
                                state.on_update = if code.is_empty() { None } else { Some(code) };
                                changed = true;
                            }
                        });
                        ui.collapsing("On Exit", |ui| {
                            let mut code = state.on_exit.clone().unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::multiline(&mut code)
                                        .code_editor()
                                        .desired_rows(2),
                                )
                                .changed()
                            {
                                state.on_exit = if code.is_empty() { None } else { Some(code) };
                                changed = true;
                            }
                        });
                        ui.label("Transitions:");
                        let mut remove_trans_idx = None;
                        for (ti, trans) in state.transitions.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label("→");
                                if ui.add(egui::DragValue::new(&mut trans.to)).changed() {
                                    changed = true;
                                }
                                ui.label("Pri:");
                                if ui.add(egui::DragValue::new(&mut trans.priority)).changed() {
                                    changed = true;
                                }
                                if ui.small_button("X").clicked() {
                                    remove_trans_idx = Some(ti);
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("When:");
                                if ui.text_edit_singleline(&mut trans.condition).changed() {
                                    changed = true;
                                }
                            });
                        }
                        if let Some(idx) = remove_trans_idx {
                            state.transitions.remove(idx);
                            changed = true;
                        }
                        if ui.small_button("+ Transition").clicked() {
                            state.transitions.push(TransitionConfig {
                                to: 0,
                                condition: "false".into(),
                                priority: 0,
                            });
                            changed = true;
                        }
                    });
            }
            if let Some(idx) = remove_state_idx {
                states.remove(idx);
                changed = true;
            }
            if ui.button("+ Add State").clicked() {
                let new_id = states.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                states.push(AgentStateConfig::new(new_id));
                changed = true;
            }
        }

        // Conditional (simplified)
        RuleConfig::Switch {
            condition,
            then_code,
            else_code,
        } => {
            ui.label("Condition:");
            if ui
                .add(
                    egui::TextEdit::multiline(condition)
                        .code_editor()
                        .desired_rows(1),
                )
                .changed()
            {
                changed = true;
            }
            ui.label("Then (WGSL):");
            if ui
                .add(
                    egui::TextEdit::multiline(then_code)
                        .code_editor()
                        .desired_rows(3),
                )
                .changed()
            {
                changed = true;
            }
            let mut has_else = else_code.is_some();
            if ui.checkbox(&mut has_else, "Else Branch").changed() {
                if has_else && else_code.is_none() {
                    *else_code = Some("// else code".into());
                } else if !has_else {
                    *else_code = None;
                }
                changed = true;
            }
            if let Some(code) = else_code {
                ui.label("Else (WGSL):");
                if ui
                    .add(
                        egui::TextEdit::multiline(code)
                            .code_editor()
                            .desired_rows(3),
                    )
                    .changed()
                {
                    changed = true;
                }
            }
        }
        RuleConfig::TypedNeighbor {
            self_type,
            other_type,
            radius,
            code,
        } => {
            let mut has_self_type = self_type.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut has_self_type, "Self Type").changed() {
                    if has_self_type && self_type.is_none() {
                        *self_type = Some(0);
                    } else if !has_self_type {
                        *self_type = None;
                    }
                    changed = true;
                }
                if let Some(t) = self_type {
                    if ui.add(egui::DragValue::new(t)).changed() {
                        changed = true;
                    }
                }
            });
            let mut has_other_type = other_type.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut has_other_type, "Other Type").changed() {
                    if has_other_type && other_type.is_none() {
                        *other_type = Some(0);
                    } else if !has_other_type {
                        *other_type = None;
                    }
                    changed = true;
                }
                if let Some(t) = other_type {
                    if ui.add(egui::DragValue::new(t)).changed() {
                        changed = true;
                    }
                }
            });
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            ui.label("Neighbor WGSL Code:");
            ui.label(
                egui::RichText::new("Available: neighbor_dist, neighbor_dir, neighbor_pos, other")
                    .small()
                    .weak(),
            );
            if ui
                .add(
                    egui::TextEdit::multiline(code)
                        .code_editor()
                        .desired_rows(4),
                )
                .changed()
            {
                changed = true;
            }
        }

        // Advanced Physics
        RuleConfig::DensityBuoyancy {
            density_field,
            medium_density,
            strength,
        } => {
            ui.horizontal(|ui| {
                ui.label("Density Field:");
                if ui.text_edit_singleline(density_field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(medium_density, 0.1..=10.0).text("Medium Density"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.1..=20.0).text("Strength"))
                .changed();
        }
        RuleConfig::Diffuse {
            field,
            rate,
            radius,
        } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(rate, 0.0..=1.0).text("Rate"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=0.5).text("Radius"))
                .changed();
        }
        RuleConfig::Mass { field } => {
            ui.horizontal(|ui| {
                ui.label("Mass Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Refractory {
            trigger,
            charge,
            active_threshold,
            depletion_rate,
            regen_rate,
        } => {
            ui.horizontal(|ui| {
                ui.label("Trigger Field:");
                if ui.text_edit_singleline(trigger).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Charge Field:");
                if ui.text_edit_singleline(charge).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(active_threshold, 0.0..=1.0).text("Active Threshold"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(depletion_rate, 0.0..=5.0).text("Depletion Rate"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(regen_rate, 0.0..=2.0).text("Regen Rate"))
                .changed();
        }

        // Math / Signal
        RuleConfig::Smooth {
            field,
            target,
            rate,
        } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(target, -10.0..=10.0).text("Target"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(rate, 0.0..=10.0).text("Rate"))
                .changed();
        }
        RuleConfig::Modulo { field, min, max } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(min, -10.0..=10.0).text("Min"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(max, -10.0..=10.0).text("Max"))
                .changed();
        }
        RuleConfig::Copy {
            from,
            to,
            scale,
            offset,
        } => {
            ui.horizontal(|ui| {
                ui.label("From:");
                if ui.text_edit_singleline(from).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("To:");
                if ui.text_edit_singleline(to).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(scale, -10.0..=10.0).text("Scale"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(offset, -10.0..=10.0).text("Offset"))
                .changed();
        }
        RuleConfig::Threshold {
            input_field,
            output_field,
            threshold,
            above,
            below,
        } => {
            ui.horizontal(|ui| {
                ui.label("Input:");
                if ui.text_edit_singleline(input_field).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output_field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(threshold, -10.0..=10.0).text("Threshold"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(above, 0.0..=1.0).text("Above Value"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(below, 0.0..=1.0).text("Below Value"))
                .changed();
        }
        RuleConfig::Gate { condition, action } => {
            ui.label("Condition (WGSL):");
            if ui.text_edit_multiline(condition).changed() {
                changed = true;
            }
            ui.label("Action (WGSL):");
            if ui.text_edit_multiline(action).changed() {
                changed = true;
            }
        }
        RuleConfig::Tween {
            field,
            from,
            to,
            duration,
            timer_field,
        } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(from, 0.0..=10.0).text("From"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(to, 0.0..=10.0).text("To"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(duration, 0.1..=30.0).text("Duration"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Timer Field:");
                if ui.text_edit_singleline(timer_field).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Periodic {
            interval,
            phase_field,
            action,
        } => {
            changed |= ui
                .add(egui::Slider::new(interval, 0.01..=10.0).text("Interval"))
                .changed();
            ui.horizontal(|ui| {
                let mut has_phase = phase_field.is_some();
                if ui.checkbox(&mut has_phase, "Phase Field").changed() {
                    if has_phase {
                        *phase_field = Some("phase".into());
                    } else {
                        *phase_field = None;
                    }
                    changed = true;
                }
                if let Some(pf) = phase_field {
                    if ui.text_edit_singleline(pf).changed() {
                        changed = true;
                    }
                }
            });
            ui.label("Action (WGSL):");
            if ui.text_edit_multiline(action).changed() {
                changed = true;
            }
        }

        // Field Interactions
        RuleConfig::Deposit {
            field_index,
            source,
            amount,
        } => {
            changed |= ui
                .add(egui::Slider::new(field_index, 0..=7).text("Field Index"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Source:");
                if ui.text_edit_singleline(source).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(amount, 0.0..=10.0).text("Amount"))
                .changed();
        }
        RuleConfig::Sense {
            field_index,
            target,
        } => {
            changed |= ui
                .add(egui::Slider::new(field_index, 0..=7).text("Field Index"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Target:");
                if ui.text_edit_singleline(target).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Consume {
            field_index,
            target,
            rate,
        } => {
            changed |= ui
                .add(egui::Slider::new(field_index, 0..=7).text("Field Index"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Target:");
                if ui.text_edit_singleline(target).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(rate, 0.0..=10.0).text("Rate"))
                .changed();
        }
        RuleConfig::Gradient {
            field,
            strength,
            ascending,
        } => {
            changed |= ui
                .add(egui::Slider::new(field, 0..=7).text("Field Index"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
            changed |= ui.checkbox(ascending, "Ascending").changed();
        }

        // Neighbor Field Operations
        RuleConfig::Accumulate {
            source,
            target,
            radius,
            operation,
            falloff,
        } => {
            ui.horizontal(|ui| {
                ui.label("Source:");
                if ui.text_edit_singleline(source).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Target:");
                if ui.text_edit_singleline(target).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Operation:");
                if ui.text_edit_singleline(operation).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                let mut has_falloff = falloff.is_some();
                if ui.checkbox(&mut has_falloff, "Falloff").changed() {
                    if has_falloff {
                        *falloff = Some(Falloff::Linear);
                    } else {
                        *falloff = None;
                    }
                    changed = true;
                }
                if let Some(f) = falloff {
                    changed |= render_falloff(ui, f);
                }
            });
        }
        RuleConfig::Signal {
            source,
            target,
            radius,
            strength,
            falloff,
        } => {
            ui.horizontal(|ui| {
                ui.label("Source:");
                if ui.text_edit_singleline(source).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Target:");
                if ui.text_edit_singleline(target).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(strength, 0.0..=10.0).text("Strength"))
                .changed();
            ui.horizontal(|ui| {
                let mut has_falloff = falloff.is_some();
                if ui.checkbox(&mut has_falloff, "Falloff").changed() {
                    if has_falloff {
                        *falloff = Some(Falloff::Linear);
                    } else {
                        *falloff = None;
                    }
                    changed = true;
                }
                if let Some(f) = falloff {
                    changed |= render_falloff(ui, f);
                }
            });
        }
        RuleConfig::Absorb {
            target_type,
            radius,
            source_field,
            target_field,
        } => {
            ui.horizontal(|ui| {
                let mut has_type = target_type.is_some();
                if ui.checkbox(&mut has_type, "Target Type").changed() {
                    if has_type {
                        *target_type = Some(0);
                    } else {
                        *target_type = None;
                    }
                    changed = true;
                }
                if let Some(t) = target_type {
                    changed |= ui.add(egui::Slider::new(t, 0..=7).text("")).changed();
                }
            });
            changed |= ui
                .add(egui::Slider::new(radius, 0.01..=1.0).text("Radius"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Source Field:");
                if ui.text_edit_singleline(source_field).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Target Field:");
                if ui.text_edit_singleline(target_field).changed() {
                    changed = true;
                }
            });
        }

        // Logic Gates
        RuleConfig::And { a, b, output } => {
            ui.horizontal(|ui| {
                ui.label("A:");
                if ui.text_edit_singleline(a).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("B:");
                if ui.text_edit_singleline(b).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Or { a, b, output } => {
            ui.horizontal(|ui| {
                ui.label("A:");
                if ui.text_edit_singleline(a).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("B:");
                if ui.text_edit_singleline(b).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Not { input, output, max } => {
            ui.horizontal(|ui| {
                ui.label("Input:");
                if ui.text_edit_singleline(input).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(max, 0.0..=10.0).text("Max"))
                .changed();
        }
        RuleConfig::Xor { a, b, output } => {
            ui.horizontal(|ui| {
                ui.label("A:");
                if ui.text_edit_singleline(a).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("B:");
                if ui.text_edit_singleline(b).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Hysteresis {
            input,
            output,
            low_threshold,
            high_threshold,
            on_value,
            off_value,
        } => {
            ui.horizontal(|ui| {
                ui.label("Input:");
                if ui.text_edit_singleline(input).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(low_threshold, 0.0..=1.0).text("Low Threshold"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(high_threshold, 0.0..=1.0).text("High Threshold"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(on_value, 0.0..=1.0).text("On Value"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(off_value, 0.0..=1.0).text("Off Value"))
                .changed();
        }
        RuleConfig::Latch {
            output,
            set_condition,
            reset_condition,
            set_value,
            reset_value,
        } => {
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
            ui.label("Set Condition (WGSL):");
            if ui.text_edit_multiline(set_condition).changed() {
                changed = true;
            }
            ui.label("Reset Condition (WGSL):");
            if ui.text_edit_multiline(reset_condition).changed() {
                changed = true;
            }
            changed |= ui
                .add(egui::Slider::new(set_value, 0.0..=1.0).text("Set Value"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(reset_value, 0.0..=1.0).text("Reset Value"))
                .changed();
        }
        RuleConfig::Edge {
            input,
            prev_field,
            output,
            threshold,
            rising,
            falling,
        } => {
            ui.horizontal(|ui| {
                ui.label("Input:");
                if ui.text_edit_singleline(input).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Prev Field:");
                if ui.text_edit_singleline(prev_field).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(threshold, 0.0..=1.0).text("Threshold"))
                .changed();
            changed |= ui.checkbox(rising, "Rising Edge").changed();
            changed |= ui.checkbox(falling, "Falling Edge").changed();
        }
        RuleConfig::Select {
            condition,
            then_field,
            else_field,
            output,
        } => {
            ui.label("Condition (WGSL):");
            if ui.text_edit_multiline(condition).changed() {
                changed = true;
            }
            ui.horizontal(|ui| {
                ui.label("Then Field:");
                if ui.text_edit_singleline(then_field).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Else Field:");
                if ui.text_edit_singleline(else_field).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Blend {
            a,
            b,
            weight,
            output,
        } => {
            ui.horizontal(|ui| {
                ui.label("A:");
                if ui.text_edit_singleline(a).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("B:");
                if ui.text_edit_singleline(b).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Weight:");
                if ui.text_edit_singleline(weight).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                if ui.text_edit_singleline(output).changed() {
                    changed = true;
                }
            });
        }
        RuleConfig::Sync {
            phase_field,
            frequency,
            field,
            emit_amount,
            coupling,
            detection_threshold,
            on_fire,
        } => {
            ui.horizontal(|ui| {
                ui.label("Phase Field:");
                if ui.text_edit_singleline(phase_field).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Frequency:");
                if ui
                    .add(
                        egui::DragValue::new(frequency)
                            .speed(0.1)
                            .range(0.0..=100.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Field Index:");
                if ui.add(egui::DragValue::new(field)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Emit Amount:");
                if ui
                    .add(
                        egui::DragValue::new(emit_amount)
                            .speed(0.01)
                            .range(0.0..=10.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Coupling:");
                if ui
                    .add(egui::DragValue::new(coupling).speed(0.01).range(0.0..=10.0))
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Detection Threshold:");
                if ui
                    .add(
                        egui::DragValue::new(detection_threshold)
                            .speed(0.01)
                            .range(0.0..=10.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
            let mut has_on_fire = on_fire.is_some();
            if ui.checkbox(&mut has_on_fire, "On Fire WGSL").changed() {
                if has_on_fire && on_fire.is_none() {
                    *on_fire = Some("// Fire event code".into());
                } else if !has_on_fire {
                    *on_fire = None;
                }
                changed = true;
            }
            if let Some(code) = on_fire {
                ui.label("On Fire Code:");
                if ui
                    .add(
                        egui::TextEdit::multiline(code)
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    )
                    .changed()
                {
                    changed = true;
                }
            }
        }
        RuleConfig::Split {
            condition,
            offspring_count,
            offspring_type,
            resource_field,
            resource_cost,
            spread,
            speed_min,
            speed_max,
        } => {
            ui.label("Condition:");
            if ui
                .add(
                    egui::TextEdit::multiline(condition)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(2),
                )
                .changed()
            {
                changed = true;
            }
            ui.horizontal(|ui| {
                ui.label("Offspring Count:");
                if ui
                    .add(egui::DragValue::new(offspring_count).range(1..=10))
                    .changed()
                {
                    changed = true;
                }
            });
            let mut has_offspring_type = offspring_type.is_some();
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut has_offspring_type, "Override Type")
                    .changed()
                {
                    if has_offspring_type && offspring_type.is_none() {
                        *offspring_type = Some(0);
                    } else if !has_offspring_type {
                        *offspring_type = None;
                    }
                    changed = true;
                }
                if let Some(t) = offspring_type {
                    if ui.add(egui::DragValue::new(t)).changed() {
                        changed = true;
                    }
                }
            });
            let mut has_resource = resource_field.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut has_resource, "Resource Field").changed() {
                    if has_resource && resource_field.is_none() {
                        *resource_field = Some("energy".into());
                    } else if !has_resource {
                        *resource_field = None;
                    }
                    changed = true;
                }
                if let Some(f) = resource_field {
                    if ui.text_edit_singleline(f).changed() {
                        changed = true;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Resource Cost:");
                if ui
                    .add(
                        egui::DragValue::new(resource_cost)
                            .speed(0.01)
                            .range(0.0..=100.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Spread:");
                if ui
                    .add(
                        egui::DragValue::new(spread)
                            .speed(0.01)
                            .range(0.0..=std::f32::consts::PI),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Speed Min:");
                if ui
                    .add(
                        egui::DragValue::new(speed_min)
                            .speed(0.01)
                            .range(0.0..=10.0),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.label("Max:");
                if ui
                    .add(
                        egui::DragValue::new(speed_max)
                            .speed(0.01)
                            .range(0.0..=10.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
        }
        RuleConfig::OnCollisionDynamic {
            radius,
            response,
            params,
        } => {
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui
                    .add(egui::DragValue::new(radius).speed(0.01).range(0.0..=10.0))
                    .changed()
                {
                    changed = true;
                }
            });
            ui.label("Response (WGSL):");
            ui.label(
                egui::RichText::new("Access params via uniforms.rule_N_paramname")
                    .small()
                    .weak(),
            );
            if ui
                .add(
                    egui::TextEdit::multiline(response)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(4),
                )
                .changed()
            {
                changed = true;
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Dynamic Parameters:");
                if ui
                    .small_button("+")
                    .on_hover_text("Add parameter")
                    .clicked()
                {
                    let new_name = format!("param_{}", params.len());
                    params.push((new_name, UniformValueConfig::F32(1.0)));
                    changed = true;
                }
            });
            let mut to_remove = None;
            for (idx, (name, value)) in params.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let mut name_edit = name.clone();
                    if ui
                        .add(egui::TextEdit::singleline(&mut name_edit).desired_width(80.0))
                        .changed()
                    {
                        *name = name_edit;
                        changed = true;
                    }
                    ui.label("=");
                    match value {
                        UniformValueConfig::F32(v) => {
                            if ui.add(egui::DragValue::new(v).speed(0.01)).changed() {
                                changed = true;
                            }
                        }
                        UniformValueConfig::Vec2(v) => {
                            if ui
                                .add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("x:"))
                                .changed()
                            {
                                changed = true;
                            }
                            if ui
                                .add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("y:"))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                        UniformValueConfig::Vec3(v) => {
                            if ui
                                .add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("x:"))
                                .changed()
                            {
                                changed = true;
                            }
                            if ui
                                .add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("y:"))
                                .changed()
                            {
                                changed = true;
                            }
                            if ui
                                .add(egui::DragValue::new(&mut v[2]).speed(0.01).prefix("z:"))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                        UniformValueConfig::Vec4(v) => {
                            if ui
                                .add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("x:"))
                                .changed()
                            {
                                changed = true;
                            }
                            if ui
                                .add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("y:"))
                                .changed()
                            {
                                changed = true;
                            }
                            if ui
                                .add(egui::DragValue::new(&mut v[2]).speed(0.01).prefix("z:"))
                                .changed()
                            {
                                changed = true;
                            }
                            if ui
                                .add(egui::DragValue::new(&mut v[3]).speed(0.01).prefix("w:"))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    }
                    if ui.small_button("X").on_hover_text("Remove").clicked() {
                        to_remove = Some(idx);
                    }
                });
            }
            if let Some(idx) = to_remove {
                params.remove(idx);
                changed = true;
            }
        }
    }

    changed
}
