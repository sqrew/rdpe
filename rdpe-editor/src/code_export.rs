//! Code export functionality - generates Rust code from SimConfig.

use crate::config::*;

/// Generate Rust code that creates a simulation matching the given config.
pub fn generate_code(config: &SimConfig) -> String {
    let mut code = String::new();

    // Header
    code.push_str("use rdpe::prelude::*;\n\n");
    code.push_str("fn main() {\n");
    code.push_str(&format!("    Simulation::<MetaParticle>::new({:?})\n", config.name));

    // Basic settings
    code.push_str(&format!("        .with_particle_count({})\n", config.particle_count));
    code.push_str(&format!("        .with_bounds({:.2})\n", config.bounds));
    code.push_str(&format!("        .with_particle_size({:.4})\n", config.particle_size));

    // Spatial config (if needed)
    if config.needs_spatial() {
        code.push_str(&format!(
            "        .with_spatial_config({:.2}, {})\n",
            config.spatial_cell_size, config.spatial_resolution
        ));
    }

    // Spawner closure
    code.push_str(&spawner_code(&config.spawn));

    // Rules
    for rule in &config.rules {
        code.push_str(&format!("        .with_rule({})\n", rule_code(rule)));
    }

    // Fields
    for field in &config.fields {
        code.push_str(&format!("        .with_field({:?}, {})\n", field.name, field_code(field)));
    }

    // Custom uniforms
    for (name, value) in &config.custom_uniforms {
        code.push_str(&format!("        .with_uniform({:?}, {})\n", name, uniform_value_code(value)));
    }

    // Custom shaders
    if !config.custom_shaders.vertex_code.is_empty() {
        code.push_str(&format!(
            "        .with_vertex_shader(r#\"{}\"#)\n",
            config.custom_shaders.vertex_code
        ));
    }
    if !config.custom_shaders.fragment_code.is_empty() {
        code.push_str(&format!(
            "        .with_fragment_shader(r#\"{}\"#)\n",
            config.custom_shaders.fragment_code
        ));
    }

    // Visuals closure (if any non-default)
    let visuals_code = visuals_closure_code(&config.visuals);
    if !visuals_code.is_empty() {
        code.push_str(&visuals_code);
    }

    // Run
    code.push_str("        .run();\n");
    code.push_str("}\n");

    code
}

/// Generate the spawner closure code
fn spawner_code(spawn: &SpawnConfig) -> String {
    let position_code = position_spawn_code(&spawn.shape);
    let velocity_code = velocity_spawn_code(&spawn.velocity);
    let color_code = color_spawn_code(&spawn.color_mode);

    format!(
        r#"        .with_spawner(|ctx| {{
            let position = {};
            let velocity = {};
            let color = {};
            MetaParticle {{
                position,
                velocity,
                color,
                ..Default::default()
            }}
        }})
"#,
        position_code, velocity_code, color_code
    )
}

fn position_spawn_code(shape: &SpawnShape) -> String {
    match shape {
        SpawnShape::Cube { size } => format!("ctx.random_in_cube({:.2})", size),
        SpawnShape::Sphere { radius } => format!("ctx.random_in_sphere({:.2})", radius),
        SpawnShape::Shell { inner, outer } => {
            // Shell: random on sphere between inner and outer radius
            format!("ctx.random_on_sphere(ctx.random_range({:.2}, {:.2}))", inner, outer)
        }
        SpawnShape::Ring { radius, thickness } => {
            format!("ctx.random_on_ring({:.2}) + Vec3::new(0.0, ctx.random_range(-{:.2}, {:.2}), 0.0)",
                radius, thickness / 2.0, thickness / 2.0)
        }
        SpawnShape::Point => "Vec3::ZERO".to_string(),
        SpawnShape::Line { length } => {
            format!("Vec3::new(ctx.random_range(-{:.2}, {:.2}), 0.0, 0.0)", length / 2.0, length / 2.0)
        }
        SpawnShape::Plane { width, depth } => {
            format!("Vec3::new(ctx.random_range(-{:.2}, {:.2}), 0.0, ctx.random_range(-{:.2}, {:.2}))",
                width / 2.0, width / 2.0, depth / 2.0, depth / 2.0)
        }
    }
}

fn velocity_spawn_code(vel: &InitialVelocity) -> String {
    match vel {
        InitialVelocity::Zero => "Vec3::ZERO".to_string(),
        InitialVelocity::RandomDirection { speed } => format!("ctx.random_direction() * {:.2}", speed),
        InitialVelocity::Outward { speed } => format!("ctx.outward_velocity(position, {:.2})", speed),
        InitialVelocity::Inward { speed } => format!("-ctx.outward_velocity(position, {:.2})", speed),
        InitialVelocity::Swirl { speed } => format!("ctx.tangent_velocity(position, {:.2})", speed),
        InitialVelocity::Directional { direction, speed } => {
            format!("Vec3::new({:.2}, {:.2}, {:.2}).normalize() * {:.2}",
                direction[0], direction[1], direction[2], speed)
        }
    }
}

fn color_spawn_code(mode: &ColorMode) -> String {
    match mode {
        ColorMode::Uniform { r, g, b } => format!("Vec3::new({:.2}, {:.2}, {:.2})", r, g, b),
        ColorMode::RandomHue { saturation, value } => {
            format!("ctx.random_hue({:.2}, {:.2})", saturation, value)
        }
        ColorMode::ByPosition => "position.abs().normalize()".to_string(),
        ColorMode::ByVelocity => "velocity.abs().normalize()".to_string(),
        ColorMode::Gradient { start, end } => {
            format!("Vec3::new({:.2}, {:.2}, {:.2}).lerp(Vec3::new({:.2}, {:.2}, {:.2}), ctx.progress())",
                start[0], start[1], start[2], end[0], end[1], end[2])
        }
    }
}

/// Generate with_visuals closure if needed
fn visuals_closure_code(visuals: &VisualsConfig) -> String {
    let default = VisualsConfig::default();
    let mut settings = Vec::new();

    if visuals.blend_mode != default.blend_mode {
        settings.push(format!("v.blend_mode({});", blend_mode_code(&visuals.blend_mode)));
    }
    if visuals.shape != default.shape {
        settings.push(format!("v.shape({});", shape_code(&visuals.shape)));
    }
    if visuals.trail_length > 0 {
        settings.push(format!("v.trails({});", visuals.trail_length));
    }
    if visuals.connections_enabled {
        settings.push(format!("v.connections({:.2});", visuals.connections_radius));
        let default_color = [0.5, 0.7, 1.0];
        if visuals.connections_color != default_color {
            settings.push(format!("v.connections_color(Vec3::new({:.2}, {:.2}, {:.2}));",
                visuals.connections_color[0], visuals.connections_color[1], visuals.connections_color[2]));
        }
    }
    if visuals.velocity_stretch {
        settings.push(format!("v.velocity_stretch({:.2});", visuals.velocity_stretch_factor));
    }
    if visuals.background_color != default.background_color {
        settings.push(format!("v.background(Vec3::new({:.2}, {:.2}, {:.2}));",
            visuals.background_color[0], visuals.background_color[1], visuals.background_color[2]));
    }

    if settings.is_empty() {
        String::new()
    } else {
        format!(
            "        .with_visuals(|v| {{\n            {}\n        }})\n",
            settings.join("\n            ")
        )
    }
}

fn vec3_code(v: &[f32; 3]) -> String {
    format!("Vec3::new({:.3}, {:.3}, {:.3})", v[0], v[1], v[2])
}

fn falloff_code(f: &Falloff) -> String {
    match f {
        Falloff::Constant => "Falloff::Constant",
        Falloff::Linear => "Falloff::Linear",
        Falloff::Inverse => "Falloff::Inverse",
        Falloff::InverseSquare => "Falloff::InverseSquare",
        Falloff::Smooth => "Falloff::Smooth",
    }.to_string()
}

fn rule_code(rule: &RuleConfig) -> String {
    match rule {
        // Basic Forces
        RuleConfig::Gravity(g) => format!("Rule::Gravity({:.2})", g),
        RuleConfig::Drag(d) => format!("Rule::Drag({:.3})", d),
        RuleConfig::Acceleration { direction } => {
            format!("Rule::Acceleration {{ direction: {} }}", vec3_code(direction))
        }

        // Boundaries
        RuleConfig::BounceWalls => "Rule::BounceWalls".to_string(),
        RuleConfig::WrapWalls => "Rule::WrapWalls".to_string(),

        // Point Forces
        RuleConfig::AttractTo { point, strength } => {
            format!("Rule::AttractTo {{ point: {}, strength: {:.2} }}", vec3_code(point), strength)
        }
        RuleConfig::RepelFrom { point, strength, radius } => {
            format!("Rule::RepelFrom {{ point: {}, strength: {:.2}, radius: {:.2} }}", vec3_code(point), strength, radius)
        }
        RuleConfig::PointGravity { point, strength, softening } => {
            format!("Rule::PointGravity {{ point: {}, strength: {:.2}, softening: {:.3} }}", vec3_code(point), strength, softening)
        }
        RuleConfig::Orbit { center, strength } => {
            format!("Rule::Orbit {{ center: {}, strength: {:.2} }}", vec3_code(center), strength)
        }
        RuleConfig::Spring { anchor, stiffness, damping } => {
            format!("Rule::Spring {{ anchor: {}, stiffness: {:.2}, damping: {:.3} }}", vec3_code(anchor), stiffness, damping)
        }
        RuleConfig::Radial { point, strength, radius, falloff } => {
            format!("Rule::Radial {{ point: {}, strength: {:.2}, radius: {:.2}, falloff: {} }}",
                vec3_code(point), strength, radius, falloff_code(falloff))
        }
        RuleConfig::Vortex { center, axis, strength } => {
            format!("Rule::Vortex {{ center: {}, axis: {}, strength: {:.2} }}", vec3_code(center), vec3_code(axis), strength)
        }
        RuleConfig::Pulse { point, strength, frequency, radius } => {
            format!("Rule::Pulse {{ point: {}, strength: {:.2}, frequency: {:.2}, radius: {:.2} }}",
                vec3_code(point), strength, frequency, radius)
        }

        // Noise & Flow
        RuleConfig::Turbulence { scale, strength } => {
            format!("Rule::Turbulence {{ scale: {:.2}, strength: {:.3} }}", scale, strength)
        }
        RuleConfig::Curl { scale, strength } => {
            format!("Rule::Curl {{ scale: {:.2}, strength: {:.3} }}", scale, strength)
        }
        RuleConfig::Wind { direction, strength, turbulence } => {
            format!("Rule::Wind {{ direction: {}, strength: {:.2}, turbulence: {:.2} }}", vec3_code(direction), strength, turbulence)
        }
        RuleConfig::PositionNoise { scale, strength, speed } => {
            format!("Rule::PositionNoise {{ scale: {:.2}, strength: {:.3}, speed: {:.2} }}", scale, strength, speed)
        }

        // Steering
        RuleConfig::Seek { target, max_speed, max_force } => {
            format!("Rule::Seek {{ target: {}, max_speed: {:.2}, max_force: {:.3} }}", vec3_code(target), max_speed, max_force)
        }
        RuleConfig::Flee { target, max_speed, max_force, panic_radius } => {
            format!("Rule::Flee {{ target: {}, max_speed: {:.2}, max_force: {:.3}, panic_radius: {:.2} }}",
                vec3_code(target), max_speed, max_force, panic_radius)
        }
        RuleConfig::Arrive { target, max_speed, max_force, slowing_radius } => {
            format!("Rule::Arrive {{ target: {}, max_speed: {:.2}, max_force: {:.3}, slowing_radius: {:.2} }}",
                vec3_code(target), max_speed, max_force, slowing_radius)
        }
        RuleConfig::Wander { strength, frequency } => {
            format!("Rule::Wander {{ strength: {:.3}, frequency: {:.2} }}", strength, frequency)
        }

        // Boids
        RuleConfig::Separate { radius, strength } => {
            format!("Rule::Separate {{ radius: {:.2}, strength: {:.3} }}", radius, strength)
        }
        RuleConfig::Cohere { radius, strength } => {
            format!("Rule::Cohere {{ radius: {:.2}, strength: {:.3} }}", radius, strength)
        }
        RuleConfig::Align { radius, strength } => {
            format!("Rule::Align {{ radius: {:.2}, strength: {:.3} }}", radius, strength)
        }
        RuleConfig::Flock { radius, separation, cohesion, alignment } => {
            format!("Rule::Flock {{ radius: {:.2}, separation: {:.3}, cohesion: {:.3}, alignment: {:.3} }}",
                radius, separation, cohesion, alignment)
        }
        RuleConfig::Avoid { radius, strength } => {
            format!("Rule::Avoid {{ radius: {:.2}, strength: {:.3} }}", radius, strength)
        }

        // Physics
        RuleConfig::Collide { radius, restitution } => {
            format!("Rule::Collide {{ radius: {:.3}, restitution: {:.2} }}", radius, restitution)
        }
        RuleConfig::NBodyGravity { strength, softening, radius } => {
            format!("Rule::NBodyGravity {{ strength: {:.3}, softening: {:.3}, radius: {:.2} }}", strength, softening, radius)
        }
        RuleConfig::LennardJones { epsilon, sigma, cutoff } => {
            format!("Rule::LennardJones {{ epsilon: {:.4}, sigma: {:.3}, cutoff: {:.2} }}", epsilon, sigma, cutoff)
        }
        RuleConfig::Viscosity { radius, strength } => {
            format!("Rule::Viscosity {{ radius: {:.2}, strength: {:.3} }}", radius, strength)
        }
        RuleConfig::Pressure { radius, strength, target_density } => {
            format!("Rule::Pressure {{ radius: {:.2}, strength: {:.3}, target_density: {:.2} }}", radius, strength, target_density)
        }
        RuleConfig::SurfaceTension { radius, strength, threshold } => {
            format!("Rule::SurfaceTension {{ radius: {:.2}, strength: {:.3}, threshold: {:.2} }}", radius, strength, threshold)
        }
        RuleConfig::Magnetism { radius, strength, same_repel } => {
            format!("Rule::Magnetism {{ radius: {:.2}, strength: {:.3}, same_repel: {} }}", radius, strength, same_repel)
        }

        // Constraints
        RuleConfig::SpeedLimit { min, max } => {
            format!("Rule::SpeedLimit {{ min: {:.3}, max: {:.2} }}", min, max)
        }
        RuleConfig::Buoyancy { surface_y, density } => {
            format!("Rule::Buoyancy {{ surface_y: {:.2}, density: {:.2} }}", surface_y, density)
        }
        RuleConfig::Friction { ground_y, strength, threshold } => {
            format!("Rule::Friction {{ ground_y: {:.2}, strength: {:.3}, threshold: {:.3} }}", ground_y, strength, threshold)
        }

        // Lifecycle
        RuleConfig::Age => "Rule::Age".to_string(),
        RuleConfig::Lifetime(t) => format!("Rule::Lifetime({:.2})", t),
        RuleConfig::FadeOut(t) => format!("Rule::FadeOut({:.2})", t),
        RuleConfig::ShrinkOut(t) => format!("Rule::ShrinkOut({:.2})", t),
        RuleConfig::ColorOverLife { start, end, duration } => {
            format!("Rule::ColorOverLife {{ start: {}, end: {}, duration: {:.2} }}",
                vec3_code(start), vec3_code(end), duration)
        }
        RuleConfig::ColorBySpeed { slow_color, fast_color, max_speed } => {
            format!("Rule::ColorBySpeed {{ slow_color: {}, fast_color: {}, max_speed: {:.2} }}",
                vec3_code(slow_color), vec3_code(fast_color), max_speed)
        }
        RuleConfig::ColorByAge { young_color, old_color, max_age } => {
            format!("Rule::ColorByAge {{ young_color: {}, old_color: {}, max_age: {:.2} }}",
                vec3_code(young_color), vec3_code(old_color), max_age)
        }
        RuleConfig::ScaleBySpeed { min_scale, max_scale, max_speed } => {
            format!("Rule::ScaleBySpeed {{ min_scale: {:.2}, max_scale: {:.2}, max_speed: {:.2} }}",
                min_scale, max_scale, max_speed)
        }

        // Typed Interactions
        RuleConfig::Chase { self_type, target_type, radius, strength } => {
            format!("Rule::Chase {{ self_type: {}, target_type: {}, radius: {:.2}, strength: {:.3} }}",
                self_type, target_type, radius, strength)
        }
        RuleConfig::Evade { self_type, threat_type, radius, strength } => {
            format!("Rule::Evade {{ self_type: {}, threat_type: {}, radius: {:.2}, strength: {:.3} }}",
                self_type, threat_type, radius, strength)
        }
        RuleConfig::Convert { from_type, trigger_type, to_type, radius, probability } => {
            format!("Rule::Convert {{ from_type: {}, trigger_type: {}, to_type: {}, radius: {:.2}, probability: {:.3} }}",
                from_type, trigger_type, to_type, radius, probability)
        }

        // Events
        RuleConfig::Shockwave { origin, speed, width, strength, repeat } => {
            format!("Rule::Shockwave {{ origin: {}, speed: {:.2}, width: {:.2}, strength: {:.3}, repeat: {:.2} }}",
                vec3_code(origin), speed, width, strength, repeat)
        }
        RuleConfig::Oscillate { axis, amplitude, frequency, spatial_scale } => {
            format!("Rule::Oscillate {{ axis: {}, amplitude: {:.3}, frequency: {:.2}, spatial_scale: {:.2} }}",
                vec3_code(axis), amplitude, frequency, spatial_scale)
        }
        RuleConfig::RespawnBelow { threshold_y, spawn_y, reset_velocity } => {
            format!("Rule::RespawnBelow {{ threshold_y: {:.2}, spawn_y: {:.2}, reset_velocity: {} }}",
                threshold_y, spawn_y, reset_velocity)
        }

        // Conditional
        RuleConfig::Maybe { probability, action } => {
            format!("Rule::Maybe {{ probability: {:.3}, action: r#\"{}\"#.into() }}", probability, action)
        }
        RuleConfig::Trigger { condition, action } => {
            format!("Rule::Trigger {{ condition: r#\"{}\"#.into(), action: r#\"{}\"#.into() }}", condition, action)
        }

        // Custom
        RuleConfig::Custom { code } => {
            format!("Rule::Custom(r#\"{}\"#.into())", code)
        }
        RuleConfig::NeighborCustom { code } => {
            format!("Rule::NeighborCustom(r#\"{}\"#.into())", code)
        }
        RuleConfig::OnCollision { radius, response } => {
            format!("Rule::OnCollision {{ radius: {:.3}, response: r#\"{}\"#.into() }}", radius, response)
        }
        RuleConfig::CustomDynamic { code, params } => {
            let params_str: Vec<String> = params.iter()
                .map(|(name, val)| format!("({:?}, {:.3})", name, val))
                .collect();
            format!("Rule::CustomDynamic {{ code: r#\"{}\"#.into(), params: vec![{}] }}", code, params_str.join(", "))
        }
        RuleConfig::NeighborCustomDynamic { code, params } => {
            let params_str: Vec<String> = params.iter()
                .map(|(name, val)| format!("({:?}, {:.3})", name, val))
                .collect();
            format!("Rule::NeighborCustomDynamic {{ code: r#\"{}\"#.into(), params: vec![{}] }}", code, params_str.join(", "))
        }

        // Event Hooks
        RuleConfig::OnCondition { condition, action } => {
            format!("Rule::OnCondition {{ condition: r#\"{}\"#.into(), action: r#\"{}\"#.into() }}", condition, action)
        }
        RuleConfig::OnDeath { action } => {
            format!("Rule::OnDeath {{ action: r#\"{}\"#.into() }}", action)
        }
        RuleConfig::OnInterval { interval, action } => {
            format!("Rule::OnInterval {{ interval: {:.3}, action: r#\"{}\"#.into() }}", interval, action)
        }
        RuleConfig::OnSpawn { action } => {
            format!("Rule::OnSpawn {{ action: r#\"{}\"#.into() }}", action)
        }

        // Growth & Decay
        RuleConfig::Grow { rate, min, max } => {
            format!("Rule::Grow {{ rate: {:.4}, min: {:.2}, max: {:.2} }}", rate, min, max)
        }
        RuleConfig::Decay { field, rate } => {
            format!("Rule::Decay {{ field: {:?}.into(), rate: {:.4} }}", field, rate)
        }
        RuleConfig::Die { condition } => {
            format!("Rule::Die {{ condition: r#\"{}\"#.into() }}", condition)
        }
        RuleConfig::DLA { seed_type, mobile_type, stick_radius, diffusion_strength } => {
            format!("Rule::DLA {{ seed_type: {}, mobile_type: {}, stick_radius: {:.3}, diffusion_strength: {:.3} }}",
                seed_type, mobile_type, stick_radius, diffusion_strength)
        }

        // Field Operations
        RuleConfig::CopyField { from, to } => {
            format!("Rule::CopyField {{ from: {:?}.into(), to: {:?}.into() }}", from, to)
        }
        RuleConfig::Current { field, strength } => {
            format!("Rule::Current {{ field: {:?}.into(), strength: {:.3} }}", field, strength)
        }

        // Math / Signal
        RuleConfig::Lerp { field, target, rate } => {
            format!("Rule::Lerp {{ field: {:?}.into(), target: {:.3}, rate: {:.3} }}", field, target, rate)
        }
        RuleConfig::Clamp { field, min, max } => {
            format!("Rule::Clamp {{ field: {:?}.into(), min: {:.3}, max: {:.3} }}", field, min, max)
        }
        RuleConfig::Remap { field, in_min, in_max, out_min, out_max } => {
            format!("Rule::Remap {{ field: {:?}.into(), in_min: {:.3}, in_max: {:.3}, out_min: {:.3}, out_max: {:.3} }}",
                field, in_min, in_max, out_min, out_max)
        }
        RuleConfig::Quantize { field, step } => {
            format!("Rule::Quantize {{ field: {:?}.into(), step: {:.3} }}", field, step)
        }
        RuleConfig::Noise { field, amplitude, frequency } => {
            format!("Rule::Noise {{ field: {:?}.into(), amplitude: {:.3}, frequency: {:.3} }}", field, amplitude, frequency)
        }

        // Springs
        RuleConfig::ChainSprings { stiffness, damping, rest_length, max_stretch } => {
            if let Some(max_s) = max_stretch {
                format!("Rule::ChainSprings {{ stiffness: {:.2}, damping: {:.3}, rest_length: {:.4}, max_stretch: Some({:.2}) }}",
                    stiffness, damping, rest_length, max_s)
            } else {
                format!("Rule::ChainSprings {{ stiffness: {:.2}, damping: {:.3}, rest_length: {:.4}, max_stretch: None }}",
                    stiffness, damping, rest_length)
            }
        }
        RuleConfig::RadialSprings { hub_stiffness, ring_stiffness, damping, hub_length, ring_length } => {
            format!("Rule::RadialSprings {{ hub_stiffness: {:.2}, ring_stiffness: {:.2}, damping: {:.3}, hub_length: {:.4}, ring_length: {:.4} }}",
                hub_stiffness, ring_stiffness, damping, hub_length, ring_length)
        }
        RuleConfig::BondSprings { bonds, stiffness, damping, rest_length, max_stretch } => {
            let bonds_str: Vec<String> = bonds.iter().map(|b| format!("{:?}", b)).collect();
            if let Some(ms) = max_stretch {
                format!("Rule::BondSprings {{ bonds: vec![{}], stiffness: {:.2}, damping: {:.3}, rest_length: {:.4}, max_stretch: Some({:.2}) }}",
                    bonds_str.join(", "), stiffness, damping, rest_length, ms)
            } else {
                format!("Rule::BondSprings {{ bonds: vec![{}], stiffness: {:.2}, damping: {:.3}, rest_length: {:.4}, max_stretch: None }}",
                    bonds_str.join(", "), stiffness, damping, rest_length)
            }
        }

        // State Machine
        RuleConfig::State { field, transitions } => {
            let trans_str: Vec<String> = transitions.iter()
                .map(|(from, to, cond)| format!("({}, {}, r#\"{}\"#.into())", from, to, cond))
                .collect();
            format!("Rule::State {{ field: {:?}.into(), transitions: vec![{}] }}", field, trans_str.join(", "))
        }
        RuleConfig::Agent { state_field, prev_state_field, state_timer_field, states } => {
            let states_str: Vec<String> = states.iter().map(|s| {
                let mut parts = vec![format!("AgentState::new({})", s.id)];
                if let Some(name) = &s.name {
                    parts.push(format!(".named({:?})", name));
                }
                if let Some(code) = &s.on_enter {
                    parts.push(format!(".on_enter(r#\"{}\"#)", code));
                }
                if let Some(code) = &s.on_update {
                    parts.push(format!(".on_update(r#\"{}\"#)", code));
                }
                if let Some(code) = &s.on_exit {
                    parts.push(format!(".on_exit(r#\"{}\"#)", code));
                }
                for t in &s.transitions {
                    if t.priority != 0 {
                        parts.push(format!(".transition_priority({}, r#\"{}\"#, {})", t.to, t.condition, t.priority));
                    } else {
                        parts.push(format!(".transition({}, r#\"{}\"#)", t.to, t.condition));
                    }
                }
                parts.join("")
            }).collect();
            let timer_str = match state_timer_field {
                Some(f) => format!("Some({:?}.into())", f),
                None => "None".into(),
            };
            format!("Rule::Agent {{ state_field: {:?}.into(), prev_state_field: {:?}.into(), state_timer_field: {}, states: vec![{}] }}",
                state_field, prev_state_field, timer_str, states_str.join(", "))
        }

        // Conditional (simplified)
        RuleConfig::Switch { condition, then_code, else_code } => {
            if let Some(else_c) = else_code {
                format!("Rule::Custom(r#\"if ({}) {{\n    {}\n}} else {{\n    {}\n}}\"#.into())",
                    condition, then_code, else_c)
            } else {
                format!("Rule::Custom(r#\"if ({}) {{\n    {}\n}}\"#.into())", condition, then_code)
            }
        }
        RuleConfig::TypedNeighbor { self_type, other_type, radius, code } => {
            let type_check = match (self_type, other_type) {
                (Some(st), Some(ot)) => format!("if p.particle_type != {}u || other.particle_type != {}u {{ continue; }}\\n", st, ot),
                (Some(st), None) => format!("if p.particle_type != {}u {{ continue; }}\\n", st),
                (None, Some(ot)) => format!("if other.particle_type != {}u {{ continue; }}\\n", ot),
                (None, None) => String::new(),
            };
            format!("Rule::NeighborCustom(r#\"{}if neighbor_dist < {} && neighbor_dist > 0.001 {{\n    {}\n}}\"#.into())",
                type_check, radius, code)
        }

        // Advanced Physics
        RuleConfig::DensityBuoyancy { density_field, medium_density, strength } => {
            format!("Rule::DensityBuoyancy {{ density_field: {:?}.into(), medium_density: {:.3}, strength: {:.3} }}",
                density_field, medium_density, strength)
        }
        RuleConfig::Diffuse { field, rate, radius } => {
            format!("Rule::Diffuse {{ field: {:?}.into(), rate: {:.3}, radius: {:.3} }}", field, rate, radius)
        }
        RuleConfig::Mass { field } => {
            format!("Rule::Mass {{ field: {:?}.into() }}", field)
        }
        RuleConfig::Refractory { trigger, charge, active_threshold, depletion_rate, regen_rate } => {
            format!("Rule::Refractory {{ trigger: {:?}.into(), charge: {:?}.into(), active_threshold: {:.3}, depletion_rate: {:.3}, regen_rate: {:.3} }}",
                trigger, charge, active_threshold, depletion_rate, regen_rate)
        }

        // Math / Signal
        RuleConfig::Smooth { field, target, rate } => {
            format!("Rule::Smooth {{ field: {:?}.into(), target: {:.3}, rate: {:.3} }}", field, target, rate)
        }
        RuleConfig::Modulo { field, min, max } => {
            format!("Rule::Modulo {{ field: {:?}.into(), min: {:.3}, max: {:.3} }}", field, min, max)
        }
        RuleConfig::Copy { from, to, scale, offset } => {
            format!("Rule::Copy {{ from: {:?}.into(), to: {:?}.into(), scale: {:.3}, offset: {:.3} }}", from, to, scale, offset)
        }
        RuleConfig::Threshold { input_field, output_field, threshold, above, below } => {
            format!("Rule::Threshold {{ input_field: {:?}.into(), output_field: {:?}.into(), threshold: {:.3}, above: {:.3}, below: {:.3} }}",
                input_field, output_field, threshold, above, below)
        }
        RuleConfig::Gate { condition, action } => {
            format!("Rule::Gate {{ condition: r#\"{}\"#.into(), action: r#\"{}\"#.into() }}", condition, action)
        }
        RuleConfig::Tween { field, from, to, duration, timer_field } => {
            format!("Rule::Tween {{ field: {:?}.into(), from: {:.3}, to: {:.3}, duration: {:.3}, timer_field: {:?}.into() }}",
                field, from, to, duration, timer_field)
        }
        RuleConfig::Periodic { interval, phase_field, action } => {
            let phase = match phase_field {
                Some(f) => format!("Some({:?}.into())", f),
                None => "None".to_string(),
            };
            format!("Rule::Periodic {{ interval: {:.3}, phase_field: {}, action: r#\"{}\"#.into() }}", interval, phase, action)
        }

        // Field Interactions
        RuleConfig::Deposit { field_index, source, amount } => {
            format!("Rule::Deposit {{ field_index: {}, source: {:?}.into(), amount: {:.3} }}", field_index, source, amount)
        }
        RuleConfig::Sense { field_index, target } => {
            format!("Rule::Sense {{ field_index: {}, target: {:?}.into() }}", field_index, target)
        }
        RuleConfig::Consume { field_index, target, rate } => {
            format!("Rule::Consume {{ field_index: {}, target: {:?}.into(), rate: {:.3} }}", field_index, target, rate)
        }
        RuleConfig::Gradient { field, strength, ascending } => {
            format!("Rule::Gradient {{ field: {}, strength: {:.3}, ascending: {} }}", field, strength, ascending)
        }

        // Neighbor Field Operations
        RuleConfig::Accumulate { source, target, radius, operation, falloff } => {
            let falloff_str = match falloff {
                Some(f) => format!("Some({})", falloff_code(f)),
                None => "None".to_string(),
            };
            format!("Rule::Accumulate {{ source: {:?}.into(), target: {:?}.into(), radius: {:.3}, operation: {:?}.into(), falloff: {} }}",
                source, target, radius, operation, falloff_str)
        }
        RuleConfig::Signal { source, target, radius, strength, falloff } => {
            let falloff_str = match falloff {
                Some(f) => format!("Some({})", falloff_code(f)),
                None => "None".to_string(),
            };
            format!("Rule::Signal {{ source: {:?}.into(), target: {:?}.into(), radius: {:.3}, strength: {:.3}, falloff: {} }}",
                source, target, radius, strength, falloff_str)
        }
        RuleConfig::Absorb { target_type, radius, source_field, target_field } => {
            let type_str = match target_type {
                Some(t) => format!("Some({})", t),
                None => "None".to_string(),
            };
            format!("Rule::Absorb {{ target_type: {}, radius: {:.3}, source_field: {:?}.into(), target_field: {:?}.into() }}",
                type_str, radius, source_field, target_field)
        }

        // Logic Gates
        RuleConfig::And { a, b, output } => {
            format!("Rule::And {{ a: {:?}.into(), b: {:?}.into(), output: {:?}.into() }}", a, b, output)
        }
        RuleConfig::Or { a, b, output } => {
            format!("Rule::Or {{ a: {:?}.into(), b: {:?}.into(), output: {:?}.into() }}", a, b, output)
        }
        RuleConfig::Not { input, output, max } => {
            format!("Rule::Not {{ input: {:?}.into(), output: {:?}.into(), max: {:.3} }}", input, output, max)
        }
        RuleConfig::Xor { a, b, output } => {
            format!("Rule::Xor {{ a: {:?}.into(), b: {:?}.into(), output: {:?}.into() }}", a, b, output)
        }
        RuleConfig::Hysteresis { input, output, low_threshold, high_threshold, on_value, off_value } => {
            format!("Rule::Hysteresis {{ input: {:?}.into(), output: {:?}.into(), low_threshold: {:.3}, high_threshold: {:.3}, on_value: {:.3}, off_value: {:.3} }}",
                input, output, low_threshold, high_threshold, on_value, off_value)
        }
        RuleConfig::Latch { output, set_condition, reset_condition, set_value, reset_value } => {
            format!("Rule::Latch {{ output: {:?}.into(), set_condition: r#\"{}\"#.into(), reset_condition: r#\"{}\"#.into(), set_value: {:.3}, reset_value: {:.3} }}",
                output, set_condition, reset_condition, set_value, reset_value)
        }
        RuleConfig::Edge { input, prev_field, output, threshold, rising, falling } => {
            format!("Rule::Edge {{ input: {:?}.into(), prev_field: {:?}.into(), output: {:?}.into(), threshold: {:.3}, rising: {}, falling: {} }}",
                input, prev_field, output, threshold, rising, falling)
        }
        RuleConfig::Select { condition, then_field, else_field, output } => {
            format!("Rule::Select {{ condition: r#\"{}\"#.into(), then_field: {:?}.into(), else_field: {:?}.into(), output: {:?}.into() }}",
                condition, then_field, else_field, output)
        }
        RuleConfig::Blend { a, b, weight, output } => {
            format!("Rule::Blend {{ a: {:?}.into(), b: {:?}.into(), weight: {:?}.into(), output: {:?}.into() }}", a, b, weight, output)
        }
        RuleConfig::Sync { phase_field, frequency, field, emit_amount, coupling, detection_threshold, on_fire } => {
            let on_fire_str = match on_fire {
                Some(code) => format!("Some(r#\"{}\"#.into())", code),
                None => "None".into(),
            };
            format!("Rule::Sync {{ phase_field: {:?}.into(), frequency: {:.4}, field: {}, emit_amount: {:.4}, coupling: {:.4}, detection_threshold: {:.4}, on_fire: {} }}",
                phase_field, frequency, field, emit_amount, coupling, detection_threshold, on_fire_str)
        }
        RuleConfig::Split { condition, offspring_count, offspring_type, resource_field, resource_cost, spread, speed_min, speed_max } => {
            let offspring_type_str = match offspring_type {
                Some(t) => format!("Some({})", t),
                None => "None".into(),
            };
            let resource_field_str = match resource_field {
                Some(f) => format!("Some({:?}.into())", f),
                None => "None".into(),
            };
            format!("Rule::Split {{ condition: r#\"{}\"#.into(), offspring_count: {}, offspring_type: {}, resource_field: {}, resource_cost: {:.4}, spread: {:.4}, speed_min: {:.4}, speed_max: {:.4} }}",
                condition, offspring_count, offspring_type_str, resource_field_str, resource_cost, spread, speed_min, speed_max)
        }
        RuleConfig::OnCollisionDynamic { radius, response, params } => {
            let params_code: Vec<String> = params.iter().map(|(k, v)| {
                format!("({:?}.into(), UniformValue::{})", k, match v {
                    UniformValueConfig::F32(f) => format!("F32({:.4})", f),
                    UniformValueConfig::Vec2(arr) => format!("Vec2(Vec2::new({:.4}, {:.4}))", arr[0], arr[1]),
                    UniformValueConfig::Vec3(arr) => format!("Vec3(Vec3::new({:.4}, {:.4}, {:.4}))", arr[0], arr[1], arr[2]),
                    UniformValueConfig::Vec4(arr) => format!("Vec4(Vec4::new({:.4}, {:.4}, {:.4}, {:.4}))", arr[0], arr[1], arr[2], arr[3]),
                })
            }).collect();
            format!("Rule::OnCollisionDynamic {{ radius: {:.4}, response: r#\"{}\"#.into(), params: vec![{}] }}",
                radius, response, params_code.join(", "))
        }
    }
}

fn field_code(field: &FieldConfigEntry) -> String {
    let field_type = match field.field_type {
        FieldTypeConfig::Scalar => format!("FieldConfig::new({})", field.resolution),
        FieldTypeConfig::Vector => format!("FieldConfig::new_vector({})", field.resolution),
    };

    format!(
        "{}\n            .with_extent({:.2})\n            .with_decay({:.3})\n            .with_blur({:.3})\n            .with_blur_iterations({})",
        field_type, field.extent, field.decay, field.blur, field.blur_iterations
    )
}

fn uniform_value_code(value: &UniformValueConfig) -> String {
    match value {
        UniformValueConfig::F32(v) => format!("{:.4}", v),
        UniformValueConfig::Vec2(v) => format!("Vec2::new({:.4}, {:.4})", v[0], v[1]),
        UniformValueConfig::Vec3(v) => format!("Vec3::new({:.4}, {:.4}, {:.4})", v[0], v[1], v[2]),
        UniformValueConfig::Vec4(v) => format!("Vec4::new({:.4}, {:.4}, {:.4}, {:.4})", v[0], v[1], v[2], v[3]),
    }
}

fn blend_mode_code(mode: &BlendModeConfig) -> String {
    match mode {
        BlendModeConfig::Additive => "BlendMode::Additive",
        BlendModeConfig::Alpha => "BlendMode::Alpha",
        BlendModeConfig::Multiply => "BlendMode::Multiply",
    }.to_string()
}

fn shape_code(shape: &ParticleShapeConfig) -> String {
    match shape {
        ParticleShapeConfig::Circle => "ParticleShape::Circle",
        ParticleShapeConfig::CircleHard => "ParticleShape::CircleHard",
        ParticleShapeConfig::Square => "ParticleShape::Square",
        ParticleShapeConfig::Ring => "ParticleShape::Ring",
        ParticleShapeConfig::Triangle => "ParticleShape::Triangle",
        ParticleShapeConfig::Star => "ParticleShape::Star",
        ParticleShapeConfig::Hexagon => "ParticleShape::Hexagon",
        ParticleShapeConfig::Diamond => "ParticleShape::Diamond",
        ParticleShapeConfig::Point => "ParticleShape::Point",
    }.to_string()
}

