//! Rules editing panel

use crate::config::*;
use egui::Ui;

/// All available rule templates grouped by category
pub static RULE_TEMPLATES: &[(&str, &[(&str, fn() -> RuleConfig)])] = &[
    ("Forces", &[
        ("Gravity", || RuleConfig::Gravity(2.0)),
        ("Drag", || RuleConfig::Drag(0.5)),
        ("Acceleration", || RuleConfig::Acceleration { direction: [0.0, -1.0, 0.0] }),
    ]),
    ("Boundaries", &[
        ("Bounce Walls", || RuleConfig::BounceWalls),
        ("Wrap Walls", || RuleConfig::WrapWalls),
    ]),
    ("Point Forces", &[
        ("Attract To", || RuleConfig::AttractTo { point: [0.0, 0.0, 0.0], strength: 1.0 }),
        ("Repel From", || RuleConfig::RepelFrom { point: [0.0, 0.0, 0.0], strength: 1.0, radius: 0.5 }),
        ("Point Gravity", || RuleConfig::PointGravity { point: [0.0, 0.0, 0.0], strength: 2.0, softening: 0.05 }),
        ("Orbit", || RuleConfig::Orbit { center: [0.0, 0.0, 0.0], strength: 1.0 }),
        ("Spring", || RuleConfig::Spring { anchor: [0.0, 0.0, 0.0], stiffness: 1.0, damping: 0.1 }),
        ("Radial", || RuleConfig::Radial { point: [0.0, 0.0, 0.0], strength: 1.0, radius: 1.0, falloff: Falloff::InverseSquare }),
        ("Vortex", || RuleConfig::Vortex { center: [0.0, 0.0, 0.0], axis: [0.0, 1.0, 0.0], strength: 2.0 }),
        ("Pulse", || RuleConfig::Pulse { point: [0.0, 0.0, 0.0], strength: 1.0, frequency: 1.0, radius: 1.0 }),
    ]),
    ("Noise & Flow", &[
        ("Turbulence", || RuleConfig::Turbulence { scale: 1.0, strength: 0.5 }),
        ("Curl", || RuleConfig::Curl { scale: 2.0, strength: 1.0 }),
        ("Wind", || RuleConfig::Wind { direction: [1.0, 0.0, 0.0], strength: 1.0, turbulence: 0.2 }),
        ("Position Noise", || RuleConfig::PositionNoise { scale: 1.0, strength: 0.1, speed: 1.0 }),
    ]),
    ("Steering", &[
        ("Seek", || RuleConfig::Seek { target: [0.0, 0.0, 0.0], max_speed: 1.0, max_force: 0.5 }),
        ("Flee", || RuleConfig::Flee { target: [0.0, 0.0, 0.0], max_speed: 1.0, max_force: 0.5, panic_radius: 1.0 }),
        ("Arrive", || RuleConfig::Arrive { target: [0.0, 0.0, 0.0], max_speed: 1.0, max_force: 0.5, slowing_radius: 0.5 }),
        ("Wander", || RuleConfig::Wander { strength: 0.5, frequency: 1.0 }),
    ]),
    ("Flocking", &[
        ("Separate", || RuleConfig::Separate { radius: 0.1, strength: 2.0 }),
        ("Cohere", || RuleConfig::Cohere { radius: 0.3, strength: 1.0 }),
        ("Align", || RuleConfig::Align { radius: 0.2, strength: 1.5 }),
        ("Flock", || RuleConfig::Flock { radius: 0.2, separation: 2.0, cohesion: 1.0, alignment: 1.5 }),
        ("Avoid", || RuleConfig::Avoid { radius: 0.1, strength: 3.0 }),
    ]),
    ("Physics", &[
        ("Collide", || RuleConfig::Collide { radius: 0.05, restitution: 0.8 }),
        ("N-Body Gravity", || RuleConfig::NBodyGravity { strength: 0.5, softening: 0.05, radius: 1.0 }),
        ("Lennard-Jones", || RuleConfig::LennardJones { epsilon: 0.5, sigma: 0.05, cutoff: 0.15 }),
        ("Viscosity", || RuleConfig::Viscosity { radius: 0.1, strength: 0.5 }),
        ("Pressure", || RuleConfig::Pressure { radius: 0.1, strength: 2.0, target_density: 10.0 }),
        ("Surface Tension", || RuleConfig::SurfaceTension { radius: 0.1, strength: 1.0, threshold: 5.0 }),
        ("Magnetism", || RuleConfig::Magnetism { radius: 0.2, strength: 1.0, same_repel: true }),
    ]),
    ("Constraints", &[
        ("Speed Limit", || RuleConfig::SpeedLimit { min: 0.0, max: 2.0 }),
        ("Buoyancy", || RuleConfig::Buoyancy { surface_y: 0.0, density: 0.5 }),
        ("Friction", || RuleConfig::Friction { ground_y: -1.0, strength: 0.8, threshold: 0.05 }),
    ]),
    ("Lifecycle", &[
        ("Age", || RuleConfig::Age),
        ("Lifetime", || RuleConfig::Lifetime(5.0)),
        ("Fade Out", || RuleConfig::FadeOut(3.0)),
        ("Shrink Out", || RuleConfig::ShrinkOut(3.0)),
        ("Color Over Life", || RuleConfig::ColorOverLife { start: [1.0, 1.0, 0.0], end: [1.0, 0.0, 0.0], duration: 3.0 }),
        ("Color By Speed", || RuleConfig::ColorBySpeed { slow_color: [0.0, 0.0, 1.0], fast_color: [1.0, 0.0, 0.0], max_speed: 2.0 }),
        ("Color By Age", || RuleConfig::ColorByAge { young_color: [1.0, 1.0, 1.0], old_color: [0.5, 0.5, 0.5], max_age: 5.0 }),
        ("Scale By Speed", || RuleConfig::ScaleBySpeed { min_scale: 0.5, max_scale: 2.0, max_speed: 2.0 }),
    ]),
    ("Typed", &[
        ("Chase", || RuleConfig::Chase { self_type: 1, target_type: 0, radius: 0.5, strength: 2.0 }),
        ("Evade", || RuleConfig::Evade { self_type: 0, threat_type: 1, radius: 0.3, strength: 3.0 }),
        ("Convert", || RuleConfig::Convert { from_type: 0, trigger_type: 1, to_type: 1, radius: 0.1, probability: 0.5 }),
    ]),
    ("Events", &[
        ("Shockwave", || RuleConfig::Shockwave { origin: [0.0, 0.0, 0.0], speed: 2.0, width: 0.2, strength: 1.0, repeat: 3.0 }),
        ("Oscillate", || RuleConfig::Oscillate { axis: [0.0, 1.0, 0.0], amplitude: 0.1, frequency: 2.0, spatial_scale: 1.0 }),
        ("Respawn Below", || RuleConfig::RespawnBelow { threshold_y: -1.0, spawn_y: 1.0, reset_velocity: true }),
    ]),
    ("Conditional", &[
        ("Maybe", || RuleConfig::Maybe { probability: 0.5, action: "p.velocity.y += 0.1;".into() }),
        ("Trigger", || RuleConfig::Trigger { condition: "p.age > 1.0".into(), action: "p.color = vec3(1.0, 0.0, 0.0);".into() }),
    ]),
    ("Custom", &[
        ("Custom WGSL", || RuleConfig::Custom { code: "// Your WGSL code here\np.velocity.y += 0.01;".into() }),
        ("Custom Dynamic", || RuleConfig::CustomDynamic {
            code: "// Custom code with editable params\np.velocity.y += uniforms.rule_0_strength * sin(uniforms.time);".into(),
            params: vec![("strength".into(), 1.0)],
        }),
        ("Neighbor Custom", || RuleConfig::NeighborCustom { code: "// Applied for each neighbor\nlet diff = n.position - p.position;".into() }),
        ("Neighbor Custom Dynamic", || RuleConfig::NeighborCustomDynamic {
            code: "// Neighbor code with editable params\nif neighbor_dist < uniforms.rule_0_radius {\n    p.velocity += neighbor_dir * uniforms.rule_0_force;\n}".into(),
            params: vec![("radius".into(), 0.2), ("force".into(), 0.5)],
        }),
        ("On Collision", || RuleConfig::OnCollision { radius: 0.1, response: "p.color = vec3(1.0, 0.0, 0.0);".into() }),
    ]),
    ("Event Hooks", &[
        ("On Condition", || RuleConfig::OnCondition { condition: "p.age > 1.0".into(), action: "p.color = vec3(1.0, 0.0, 0.0);".into() }),
        ("On Death", || RuleConfig::OnDeath { action: "// particle died".into() }),
        ("On Interval", || RuleConfig::OnInterval { interval: 1.0, action: "p.color = vec3(1.0, 1.0, 0.0);".into() }),
        ("On Spawn", || RuleConfig::OnSpawn { action: "// particle spawned".into() }),
    ]),
    ("Growth & Decay", &[
        ("Grow", || RuleConfig::Grow { rate: 0.5, min: 0.1, max: 2.0 }),
        ("Decay", || RuleConfig::Decay { field: "scale".into(), rate: 0.5 }),
        ("Die", || RuleConfig::Die { condition: "p.age > 5.0".into() }),
        ("DLA", || RuleConfig::DLA { seed_type: 0, mobile_type: 1, stick_radius: 0.1, diffusion_strength: 0.5 }),
    ]),
    ("Fields", &[
        ("Copy Field", || RuleConfig::CopyField { from: "age".into(), to: "scale".into() }),
        ("Current", || RuleConfig::Current { field: "flow".into(), strength: 1.0 }),
    ]),
    ("Math", &[
        ("Lerp", || RuleConfig::Lerp { field: "scale".into(), target: 1.0, rate: 1.0 }),
        ("Clamp", || RuleConfig::Clamp { field: "scale".into(), min: 0.1, max: 2.0 }),
        ("Remap", || RuleConfig::Remap { field: "age".into(), in_min: 0.0, in_max: 5.0, out_min: 1.0, out_max: 0.0 }),
        ("Quantize", || RuleConfig::Quantize { field: "scale".into(), step: 0.25 }),
        ("Noise", || RuleConfig::Noise { field: "scale".into(), amplitude: 0.1, frequency: 2.0 }),
    ]),
];

pub fn render_rules_panel(ui: &mut Ui, rules: &mut Vec<RuleConfig>) -> bool {
    let mut changed = false;
    let mut remove_idx = None;
    let mut move_up_idx = None;
    let mut move_down_idx = None;

    ui.heading("Rules");
    ui.separator();

    // Add rule dropdown
    egui::ComboBox::from_label("Add Rule")
        .selected_text("Select...")
        .show_ui(ui, |ui| {
            for (category, templates) in RULE_TEMPLATES {
                ui.separator();
                ui.label(*category);
                for (name, factory) in *templates {
                    if ui.selectable_label(false, *name).clicked() {
                        rules.push(factory());
                        changed = true;
                    }
                }
            }
        });

    ui.separator();

    // List existing rules
    let rules_len = rules.len();
    for (idx, rule) in rules.iter_mut().enumerate() {
        let id = ui.make_persistent_id(format!("rule_{}", idx));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{}.", idx + 1));
                    ui.strong(rule.name());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("X").clicked() {
                            remove_idx = Some(idx);
                        }
                        if idx < rules_len - 1 && ui.small_button("v").clicked() {
                            move_down_idx = Some(idx);
                        }
                        if idx > 0 && ui.small_button("^").clicked() {
                            move_up_idx = Some(idx);
                        }
                    });
                });
            })
            .body(|ui| {
                changed |= render_rule_params(ui, rule);
            });
    }

    // Handle removals and reordering
    if let Some(idx) = remove_idx {
        rules.remove(idx);
        changed = true;
    }
    if let Some(idx) = move_up_idx {
        rules.swap(idx, idx - 1);
        changed = true;
    }
    if let Some(idx) = move_down_idx {
        rules.swap(idx, idx + 1);
        changed = true;
    }

    changed
}

fn render_rule_params(ui: &mut Ui, rule: &mut RuleConfig) -> bool {
    let mut changed = false;

    match rule {
        // === Basic Forces ===
        RuleConfig::Gravity(g) => {
            changed |= ui.add(egui::Slider::new(g, -10.0..=10.0).text("Strength")).changed();
        }
        RuleConfig::Drag(d) => {
            changed |= ui.add(egui::Slider::new(d, 0.0..=2.0).text("Drag")).changed();
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
            changed |= ui.add(egui::Slider::new(strength, -5.0..=5.0).text("Strength")).changed();
        }
        RuleConfig::RepelFrom { point, strength, radius } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(radius, 0.01..=2.0).text("Radius")).changed();
        }
        RuleConfig::PointGravity { point, strength, softening } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui.add(egui::Slider::new(strength, -10.0..=10.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(softening, 0.001..=0.5).text("Softening")).changed();
        }
        RuleConfig::Orbit { center, strength } => {
            changed |= render_vec3(ui, "Center", center);
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
        }
        RuleConfig::Spring { anchor, stiffness, damping } => {
            changed |= render_vec3(ui, "Anchor", anchor);
            changed |= ui.add(egui::Slider::new(stiffness, 0.0..=10.0).text("Stiffness")).changed();
            changed |= ui.add(egui::Slider::new(damping, 0.0..=2.0).text("Damping")).changed();
        }
        RuleConfig::Radial { point, strength, radius, falloff } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui.add(egui::Slider::new(strength, -10.0..=10.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(radius, 0.1..=5.0).text("Radius")).changed();
            changed |= render_falloff(ui, falloff);
        }
        RuleConfig::Vortex { center, axis, strength } => {
            changed |= render_vec3(ui, "Center", center);
            changed |= render_vec3(ui, "Axis", axis);
            changed |= ui.add(egui::Slider::new(strength, -10.0..=10.0).text("Strength")).changed();
        }
        RuleConfig::Pulse { point, strength, frequency, radius } => {
            changed |= render_vec3(ui, "Point", point);
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency")).changed();
            changed |= ui.add(egui::Slider::new(radius, 0.1..=5.0).text("Radius")).changed();
        }

        // === Noise & Flow ===
        RuleConfig::Turbulence { scale, strength } => {
            changed |= ui.add(egui::Slider::new(scale, 0.1..=10.0).text("Scale")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
        }
        RuleConfig::Curl { scale, strength } => {
            changed |= ui.add(egui::Slider::new(scale, 0.1..=10.0).text("Scale")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
        }
        RuleConfig::Wind { direction, strength, turbulence } => {
            changed |= render_vec3(ui, "Direction", direction);
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(turbulence, 0.0..=1.0).text("Turbulence")).changed();
        }
        RuleConfig::PositionNoise { scale, strength, speed } => {
            changed |= ui.add(egui::Slider::new(scale, 0.1..=10.0).text("Scale")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=1.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(speed, 0.0..=5.0).text("Speed")).changed();
        }

        // === Steering ===
        RuleConfig::Seek { target, max_speed, max_force } => {
            changed |= render_vec3(ui, "Target", target);
            changed |= ui.add(egui::Slider::new(max_speed, 0.1..=5.0).text("Max Speed")).changed();
            changed |= ui.add(egui::Slider::new(max_force, 0.1..=5.0).text("Max Force")).changed();
        }
        RuleConfig::Flee { target, max_speed, max_force, panic_radius } => {
            changed |= render_vec3(ui, "Target", target);
            changed |= ui.add(egui::Slider::new(max_speed, 0.1..=5.0).text("Max Speed")).changed();
            changed |= ui.add(egui::Slider::new(max_force, 0.1..=5.0).text("Max Force")).changed();
            changed |= ui.add(egui::Slider::new(panic_radius, 0.1..=5.0).text("Panic Radius")).changed();
        }
        RuleConfig::Arrive { target, max_speed, max_force, slowing_radius } => {
            changed |= render_vec3(ui, "Target", target);
            changed |= ui.add(egui::Slider::new(max_speed, 0.1..=5.0).text("Max Speed")).changed();
            changed |= ui.add(egui::Slider::new(max_force, 0.1..=5.0).text("Max Force")).changed();
            changed |= ui.add(egui::Slider::new(slowing_radius, 0.1..=5.0).text("Slowing Radius")).changed();
        }
        RuleConfig::Wander { strength, frequency } => {
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency")).changed();
        }

        // === Flocking ===
        RuleConfig::Separate { radius, strength } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=1.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength")).changed();
        }
        RuleConfig::Cohere { radius, strength } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=1.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
        }
        RuleConfig::Align { radius, strength } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=1.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
        }
        RuleConfig::Flock { radius, separation, cohesion, alignment } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=1.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(separation, 0.0..=5.0).text("Separation")).changed();
            changed |= ui.add(egui::Slider::new(cohesion, 0.0..=5.0).text("Cohesion")).changed();
            changed |= ui.add(egui::Slider::new(alignment, 0.0..=5.0).text("Alignment")).changed();
        }
        RuleConfig::Avoid { radius, strength } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=1.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength")).changed();
        }

        // === Physics ===
        RuleConfig::Collide { radius, restitution } => {
            changed |= ui.add(egui::Slider::new(radius, 0.001..=0.5).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(restitution, 0.0..=1.0).text("Restitution")).changed();
        }
        RuleConfig::NBodyGravity { strength, softening, radius } => {
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(softening, 0.001..=0.5).text("Softening")).changed();
            changed |= ui.add(egui::Slider::new(radius, 0.1..=5.0).text("Radius")).changed();
        }
        RuleConfig::LennardJones { epsilon, sigma, cutoff } => {
            changed |= ui.add(egui::Slider::new(epsilon, 0.0..=2.0).text("Epsilon")).changed();
            changed |= ui.add(egui::Slider::new(sigma, 0.01..=0.5).text("Sigma")).changed();
            changed |= ui.add(egui::Slider::new(cutoff, 0.01..=1.0).text("Cutoff")).changed();
        }
        RuleConfig::Viscosity { radius, strength } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=0.5).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=2.0).text("Strength")).changed();
        }
        RuleConfig::Pressure { radius, strength, target_density } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=0.5).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(target_density, 1.0..=50.0).text("Target Density")).changed();
        }
        RuleConfig::SurfaceTension { radius, strength, threshold } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=0.5).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(threshold, 1.0..=20.0).text("Threshold")).changed();
        }
        RuleConfig::Magnetism { radius, strength, same_repel } => {
            changed |= ui.add(egui::Slider::new(radius, 0.01..=1.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
            changed |= ui.checkbox(same_repel, "Same Polarity Repels").changed();
        }

        // === Constraints ===
        RuleConfig::SpeedLimit { min, max } => {
            changed |= ui.add(egui::Slider::new(min, 0.0..=5.0).text("Min")).changed();
            changed |= ui.add(egui::Slider::new(max, 0.0..=10.0).text("Max")).changed();
        }
        RuleConfig::Buoyancy { surface_y, density } => {
            changed |= ui.add(egui::Slider::new(surface_y, -2.0..=2.0).text("Surface Y")).changed();
            changed |= ui.add(egui::Slider::new(density, 0.0..=2.0).text("Density")).changed();
        }
        RuleConfig::Friction { ground_y, strength, threshold } => {
            changed |= ui.add(egui::Slider::new(ground_y, -2.0..=2.0).text("Ground Y")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=1.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(threshold, 0.0..=0.2).text("Threshold")).changed();
        }

        // === Lifecycle ===
        RuleConfig::Age => {
            ui.label("Increments particle age each frame");
        }
        RuleConfig::Lifetime(t) => {
            changed |= ui.add(egui::Slider::new(t, 0.1..=30.0).text("Lifetime")).changed();
        }
        RuleConfig::FadeOut(t) => {
            changed |= ui.add(egui::Slider::new(t, 0.1..=30.0).text("Duration")).changed();
        }
        RuleConfig::ShrinkOut(t) => {
            changed |= ui.add(egui::Slider::new(t, 0.1..=30.0).text("Duration")).changed();
        }
        RuleConfig::ColorOverLife { start, end, duration } => {
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
            changed |= ui.add(egui::Slider::new(duration, 0.1..=30.0).text("Duration")).changed();
        }
        RuleConfig::ColorBySpeed { slow_color, fast_color, max_speed } => {
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
            changed |= ui.add(egui::Slider::new(max_speed, 0.1..=10.0).text("Max Speed")).changed();
        }
        RuleConfig::ColorByAge { young_color, old_color, max_age } => {
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
            changed |= ui.add(egui::Slider::new(max_age, 0.1..=30.0).text("Max Age")).changed();
        }
        RuleConfig::ScaleBySpeed { min_scale, max_scale, max_speed } => {
            changed |= ui.add(egui::Slider::new(min_scale, 0.1..=2.0).text("Min Scale")).changed();
            changed |= ui.add(egui::Slider::new(max_scale, 0.1..=5.0).text("Max Scale")).changed();
            changed |= ui.add(egui::Slider::new(max_speed, 0.1..=10.0).text("Max Speed")).changed();
        }

        // === Typed Interactions ===
        RuleConfig::Chase { self_type, target_type, radius, strength } => {
            changed |= ui.add(egui::Slider::new(self_type, 0..=7).text("Self Type")).changed();
            changed |= ui.add(egui::Slider::new(target_type, 0..=7).text("Target Type")).changed();
            changed |= ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength")).changed();
        }
        RuleConfig::Evade { self_type, threat_type, radius, strength } => {
            changed |= ui.add(egui::Slider::new(self_type, 0..=7).text("Self Type")).changed();
            changed |= ui.add(egui::Slider::new(threat_type, 0..=7).text("Threat Type")).changed();
            changed |= ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=10.0).text("Strength")).changed();
        }
        RuleConfig::Convert { from_type, trigger_type, to_type, radius, probability } => {
            changed |= ui.add(egui::Slider::new(from_type, 0..=7).text("From Type")).changed();
            changed |= ui.add(egui::Slider::new(trigger_type, 0..=7).text("Trigger Type")).changed();
            changed |= ui.add(egui::Slider::new(to_type, 0..=7).text("To Type")).changed();
            changed |= ui.add(egui::Slider::new(radius, 0.1..=2.0).text("Radius")).changed();
            changed |= ui.add(egui::Slider::new(probability, 0.0..=1.0).text("Probability")).changed();
        }

        // === Events ===
        RuleConfig::Shockwave { origin, speed, width, strength, repeat } => {
            changed |= render_vec3(ui, "Origin", origin);
            changed |= ui.add(egui::Slider::new(speed, 0.1..=10.0).text("Speed")).changed();
            changed |= ui.add(egui::Slider::new(width, 0.01..=1.0).text("Width")).changed();
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
            changed |= ui.add(egui::Slider::new(repeat, 0.1..=10.0).text("Repeat")).changed();
        }
        RuleConfig::Oscillate { axis, amplitude, frequency, spatial_scale } => {
            changed |= render_vec3(ui, "Axis", axis);
            changed |= ui.add(egui::Slider::new(amplitude, 0.0..=1.0).text("Amplitude")).changed();
            changed |= ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency")).changed();
            changed |= ui.add(egui::Slider::new(spatial_scale, 0.1..=10.0).text("Spatial Scale")).changed();
        }
        RuleConfig::RespawnBelow { threshold_y, spawn_y, reset_velocity } => {
            changed |= ui.add(egui::Slider::new(threshold_y, -5.0..=0.0).text("Threshold Y")).changed();
            changed |= ui.add(egui::Slider::new(spawn_y, 0.0..=5.0).text("Spawn Y")).changed();
            changed |= ui.checkbox(reset_velocity, "Reset Velocity").changed();
        }

        // === Conditional ===
        RuleConfig::Maybe { probability, action } => {
            changed |= ui.add(egui::Slider::new(probability, 0.0..=1.0).text("Probability")).changed();
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
            changed |= ui.add(egui::Slider::new(radius, 0.01..=0.5).text("Radius")).changed();
            ui.label("Response (WGSL):");
            if ui.text_edit_multiline(response).changed() {
                changed = true;
            }
        }
        RuleConfig::CustomDynamic { code, params } => {
            ui.label("WGSL Code:");
            ui.label(egui::RichText::new("Access params via uniforms.rule_N_paramname").small().weak());
            if ui.text_edit_multiline(code).changed() {
                changed = true;
            }
            ui.separator();

            // Parameters with add/remove
            ui.horizontal(|ui| {
                ui.label("Parameters:");
                if ui.small_button("+").on_hover_text("Add parameter").clicked() {
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
                    if ui.add(egui::TextEdit::singleline(&mut name_edit).desired_width(80.0)).changed() {
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
            ui.label(egui::RichText::new("Available: neighbor_dist, neighbor_dir, neighbor_pos, neighbor_vel, other").small().weak());
            ui.label(egui::RichText::new("Access params via uniforms.rule_N_paramname").small().weak());
            if ui.text_edit_multiline(code).changed() {
                changed = true;
            }
            ui.separator();

            // Parameters with add/remove
            ui.horizontal(|ui| {
                ui.label("Parameters:");
                if ui.small_button("+").on_hover_text("Add parameter").clicked() {
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
                    if ui.add(egui::TextEdit::singleline(&mut name_edit).desired_width(80.0)).changed() {
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
            changed |= ui.add(egui::Slider::new(interval, 0.01..=10.0).text("Interval (s)")).changed();
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
            changed |= ui.add(egui::Slider::new(rate, -2.0..=2.0).text("Rate")).changed();
            changed |= ui.add(egui::Slider::new(min, 0.0..=1.0).text("Min Scale")).changed();
            changed |= ui.add(egui::Slider::new(max, 0.1..=5.0).text("Max Scale")).changed();
        }
        RuleConfig::Decay { field, rate } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui.add(egui::Slider::new(rate, 0.0..=5.0).text("Rate")).changed();
        }
        RuleConfig::Die { condition } => {
            ui.label("Death Condition (WGSL):");
            if ui.text_edit_singleline(condition).changed() {
                changed = true;
            }
        }
        RuleConfig::DLA { seed_type, mobile_type, stick_radius, diffusion_strength } => {
            changed |= ui.add(egui::Slider::new(seed_type, 0..=7).text("Seed Type")).changed();
            changed |= ui.add(egui::Slider::new(mobile_type, 0..=7).text("Mobile Type")).changed();
            changed |= ui.add(egui::Slider::new(stick_radius, 0.01..=0.5).text("Stick Radius")).changed();
            changed |= ui.add(egui::Slider::new(diffusion_strength, 0.0..=2.0).text("Diffusion")).changed();
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
            changed |= ui.add(egui::Slider::new(strength, 0.0..=5.0).text("Strength")).changed();
        }

        // === Math / Signal ===
        RuleConfig::Lerp { field, target, rate } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui.add(egui::Slider::new(target, -10.0..=10.0).text("Target")).changed();
            changed |= ui.add(egui::Slider::new(rate, 0.0..=10.0).text("Rate")).changed();
        }
        RuleConfig::Clamp { field, min, max } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui.add(egui::Slider::new(min, -10.0..=10.0).text("Min")).changed();
            changed |= ui.add(egui::Slider::new(max, -10.0..=10.0).text("Max")).changed();
        }
        RuleConfig::Remap { field, in_min, in_max, out_min, out_max } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui.add(egui::Slider::new(in_min, -10.0..=10.0).text("In Min")).changed();
            changed |= ui.add(egui::Slider::new(in_max, -10.0..=10.0).text("In Max")).changed();
            changed |= ui.add(egui::Slider::new(out_min, -10.0..=10.0).text("Out Min")).changed();
            changed |= ui.add(egui::Slider::new(out_max, -10.0..=10.0).text("Out Max")).changed();
        }
        RuleConfig::Quantize { field, step } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui.add(egui::Slider::new(step, 0.01..=1.0).text("Step Size")).changed();
        }
        RuleConfig::Noise { field, amplitude, frequency } => {
            ui.horizontal(|ui| {
                ui.label("Field:");
                if ui.text_edit_singleline(field).changed() {
                    changed = true;
                }
            });
            changed |= ui.add(egui::Slider::new(amplitude, 0.0..=2.0).text("Amplitude")).changed();
            changed |= ui.add(egui::Slider::new(frequency, 0.1..=10.0).text("Frequency")).changed();
        }
    }

    changed
}

fn render_vec3(ui: &mut Ui, label: &str, v: &mut [f32; 3]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(format!("{}:", label));
        changed |= ui.add(egui::DragValue::new(&mut v[0]).speed(0.01).prefix("x:")).changed();
        changed |= ui.add(egui::DragValue::new(&mut v[1]).speed(0.01).prefix("y:")).changed();
        changed |= ui.add(egui::DragValue::new(&mut v[2]).speed(0.01).prefix("z:")).changed();
    });
    changed
}

fn render_falloff(ui: &mut Ui, falloff: &mut Falloff) -> bool {
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
