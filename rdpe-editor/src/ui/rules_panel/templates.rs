//! Rule templates for the rules panel

use crate::config::*;

/// All available rule templates grouped by category
pub static RULE_TEMPLATES: &[(&str, &[(&str, fn() -> RuleConfig)])] = &[
    (
        "Forces",
        &[
            ("Gravity", || RuleConfig::Gravity(2.0)),
            ("Drag", || RuleConfig::Drag(0.5)),
            ("Acceleration", || RuleConfig::Acceleration {
                direction: [0.0, -1.0, 0.0],
            }),
        ],
    ),
    (
        "Boundaries",
        &[
            ("Bounce Walls", || RuleConfig::BounceWalls),
            ("Wrap Walls", || RuleConfig::WrapWalls),
        ],
    ),
    (
        "Point Forces",
        &[
            ("Attract To", || RuleConfig::AttractTo {
                point: [0.0, 0.0, 0.0],
                strength: 1.0,
            }),
            ("Repel From", || RuleConfig::RepelFrom {
                point: [0.0, 0.0, 0.0],
                strength: 1.0,
                radius: 0.5,
            }),
            ("Point Gravity", || RuleConfig::PointGravity {
                point: [0.0, 0.0, 0.0],
                strength: 2.0,
                softening: 0.05,
            }),
            ("Orbit", || RuleConfig::Orbit {
                center: [0.0, 0.0, 0.0],
                strength: 1.0,
            }),
            ("Spring", || RuleConfig::Spring {
                anchor: [0.0, 0.0, 0.0],
                stiffness: 1.0,
                damping: 0.1,
            }),
            ("Radial", || RuleConfig::Radial {
                point: [0.0, 0.0, 0.0],
                strength: 1.0,
                radius: 1.0,
                falloff: Falloff::InverseSquare,
            }),
            ("Vortex", || RuleConfig::Vortex {
                center: [0.0, 0.0, 0.0],
                axis: [0.0, 1.0, 0.0],
                strength: 2.0,
            }),
            ("Pulse", || RuleConfig::Pulse {
                point: [0.0, 0.0, 0.0],
                strength: 1.0,
                frequency: 1.0,
                radius: 1.0,
            }),
        ],
    ),
    (
        "Noise & Flow",
        &[
            ("Turbulence", || RuleConfig::Turbulence {
                scale: 1.0,
                strength: 0.5,
            }),
            ("Curl", || RuleConfig::Curl {
                scale: 2.0,
                strength: 1.0,
            }),
            ("Wind", || RuleConfig::Wind {
                direction: [1.0, 0.0, 0.0],
                strength: 1.0,
                turbulence: 0.2,
            }),
            ("Position Noise", || RuleConfig::PositionNoise {
                scale: 1.0,
                strength: 0.1,
                speed: 1.0,
            }),
        ],
    ),
    (
        "Steering",
        &[
            ("Seek", || RuleConfig::Seek {
                target: [0.0, 0.0, 0.0],
                max_speed: 1.0,
                max_force: 0.5,
            }),
            ("Flee", || RuleConfig::Flee {
                target: [0.0, 0.0, 0.0],
                max_speed: 1.0,
                max_force: 0.5,
                panic_radius: 1.0,
            }),
            ("Arrive", || RuleConfig::Arrive {
                target: [0.0, 0.0, 0.0],
                max_speed: 1.0,
                max_force: 0.5,
                slowing_radius: 0.5,
            }),
            ("Wander", || RuleConfig::Wander {
                strength: 0.5,
                frequency: 1.0,
            }),
        ],
    ),
    (
        "Flocking",
        &[
            ("Separate", || RuleConfig::Separate {
                radius: 0.1,
                strength: 2.0,
            }),
            ("Cohere", || RuleConfig::Cohere {
                radius: 0.3,
                strength: 1.0,
            }),
            ("Align", || RuleConfig::Align {
                radius: 0.2,
                strength: 1.5,
            }),
            ("Flock", || RuleConfig::Flock {
                radius: 0.2,
                separation: 2.0,
                cohesion: 1.0,
                alignment: 1.5,
            }),
            ("Avoid", || RuleConfig::Avoid {
                radius: 0.1,
                strength: 3.0,
            }),
        ],
    ),
    (
        "Physics",
        &[
            ("Collide", || RuleConfig::Collide {
                radius: 0.05,
                restitution: 0.8,
            }),
            ("N-Body Gravity", || RuleConfig::NBodyGravity {
                strength: 0.5,
                softening: 0.05,
                radius: 1.0,
            }),
            ("Lennard-Jones", || RuleConfig::LennardJones {
                epsilon: 0.5,
                sigma: 0.05,
                cutoff: 0.15,
            }),
            ("Viscosity", || RuleConfig::Viscosity {
                radius: 0.1,
                strength: 0.5,
            }),
            ("Pressure", || RuleConfig::Pressure {
                radius: 0.1,
                strength: 2.0,
                target_density: 10.0,
            }),
            ("Surface Tension", || RuleConfig::SurfaceTension {
                radius: 0.1,
                strength: 1.0,
                threshold: 5.0,
            }),
            ("Magnetism", || RuleConfig::Magnetism {
                radius: 0.2,
                strength: 1.0,
                same_repel: true,
            }),
        ],
    ),
    (
        "Constraints",
        &[
            ("Speed Limit", || RuleConfig::SpeedLimit {
                min: 0.0,
                max: 2.0,
            }),
            ("Buoyancy", || RuleConfig::Buoyancy {
                surface_y: 0.0,
                density: 0.5,
            }),
            ("Friction", || RuleConfig::Friction {
                ground_y: -1.0,
                strength: 0.8,
                threshold: 0.05,
            }),
        ],
    ),
    (
        "Lifecycle",
        &[
            ("Age", || RuleConfig::Age),
            ("Lifetime", || RuleConfig::Lifetime(5.0)),
            ("Fade Out", || RuleConfig::FadeOut(3.0)),
            ("Shrink Out", || RuleConfig::ShrinkOut(3.0)),
            ("Color Over Life", || RuleConfig::ColorOverLife {
                start: [1.0, 1.0, 0.0],
                end: [1.0, 0.0, 0.0],
                duration: 3.0,
            }),
            ("Color By Speed", || RuleConfig::ColorBySpeed {
                slow_color: [0.0, 0.0, 1.0],
                fast_color: [1.0, 0.0, 0.0],
                max_speed: 2.0,
            }),
            ("Color By Age", || RuleConfig::ColorByAge {
                young_color: [1.0, 1.0, 1.0],
                old_color: [0.5, 0.5, 0.5],
                max_age: 5.0,
            }),
            ("Scale By Speed", || RuleConfig::ScaleBySpeed {
                min_scale: 0.5,
                max_scale: 2.0,
                max_speed: 2.0,
            }),
            ("Sync", || RuleConfig::Sync {
                phase_field: "phase".into(),
                frequency: 1.0,
                field: 0,
                emit_amount: 1.0,
                coupling: 0.5,
                detection_threshold: 0.5,
                on_fire: None,
            }),
            ("Split", || RuleConfig::Split {
                condition: "p.energy > 2.0".into(),
                offspring_count: 2,
                offspring_type: None,
                resource_field: Some("energy".into()),
                resource_cost: 1.0,
                spread: 0.5,
                speed_min: 0.1,
                speed_max: 0.5,
            }),
        ],
    ),
    (
        "Typed",
        &[
            ("Chase", || RuleConfig::Chase {
                self_type: 1,
                target_type: 0,
                radius: 0.5,
                strength: 2.0,
            }),
            ("Evade", || RuleConfig::Evade {
                self_type: 0,
                threat_type: 1,
                radius: 0.3,
                strength: 3.0,
            }),
            ("Convert", || RuleConfig::Convert {
                from_type: 0,
                trigger_type: 1,
                to_type: 1,
                radius: 0.1,
                probability: 0.5,
            }),
        ],
    ),
    (
        "Events",
        &[
            ("Shockwave", || RuleConfig::Shockwave {
                origin: [0.0, 0.0, 0.0],
                speed: 2.0,
                width: 0.2,
                strength: 1.0,
                repeat: 3.0,
            }),
            ("Oscillate", || RuleConfig::Oscillate {
                axis: [0.0, 1.0, 0.0],
                amplitude: 0.1,
                frequency: 2.0,
                spatial_scale: 1.0,
            }),
            ("Respawn Below", || RuleConfig::RespawnBelow {
                threshold_y: -1.0,
                spawn_y: 1.0,
                reset_velocity: true,
            }),
        ],
    ),
    (
        "Conditional",
        &[
            ("Maybe", || RuleConfig::Maybe {
                probability: 0.5,
                action: "p.velocity.y += 0.1;".into(),
            }),
            ("Trigger", || RuleConfig::Trigger {
                condition: "p.age > 1.0".into(),
                action: "p.color = vec3(1.0, 0.0, 0.0);".into(),
            }),
        ],
    ),
    (
        "Custom",
        &[
            ("Custom WGSL", || RuleConfig::Custom {
                code: "// Your WGSL code here\np.velocity.y += 0.01;".into(),
            }),
            ("Custom Dynamic", || {
                RuleConfig::CustomDynamic {
            code: "// Custom code with editable params\np.velocity.y += uniforms.rule_0_strength * sin(uniforms.time);".into(),
            params: vec![("strength".into(), 1.0)],
        }
            }),
            ("Neighbor Custom", || RuleConfig::NeighborCustom {
                code: "// Applied for each neighbor\nlet diff = n.position - p.position;".into(),
            }),
            ("Neighbor Custom Dynamic", || {
                RuleConfig::NeighborCustomDynamic {
            code: "// Neighbor code with editable params\nif neighbor_dist < uniforms.rule_0_radius {\n    p.velocity += neighbor_dir * uniforms.rule_0_force;\n}".into(),
            params: vec![("radius".into(), 0.2), ("force".into(), 0.5)],
        }
            }),
            ("On Collision", || RuleConfig::OnCollision {
                radius: 0.1,
                response: "p.color = vec3(1.0, 0.0, 0.0);".into(),
            }),
            ("On Collision Dynamic", || {
                RuleConfig::OnCollisionDynamic {
            radius: 0.1,
            response: "// Collision response with editable params\np.velocity = -p.velocity * uniforms.rule_0_bounce;".into(),
            params: vec![("bounce".into(), UniformValueConfig::F32(0.8))],
        }
            }),
        ],
    ),
    (
        "Event Hooks",
        &[
            ("On Condition", || RuleConfig::OnCondition {
                condition: "p.age > 1.0".into(),
                action: "p.color = vec3(1.0, 0.0, 0.0);".into(),
            }),
            ("On Death", || RuleConfig::OnDeath {
                action: "// particle died".into(),
            }),
            ("On Interval", || RuleConfig::OnInterval {
                interval: 1.0,
                action: "p.color = vec3(1.0, 1.0, 0.0);".into(),
            }),
            ("On Spawn", || RuleConfig::OnSpawn {
                action: "// particle spawned".into(),
            }),
        ],
    ),
    (
        "Growth & Decay",
        &[
            ("Grow", || RuleConfig::Grow {
                rate: 0.5,
                min: 0.1,
                max: 2.0,
            }),
            ("Decay", || RuleConfig::Decay {
                field: "scale".into(),
                rate: 0.5,
            }),
            ("Die", || RuleConfig::Die {
                condition: "p.age > 5.0".into(),
            }),
            ("DLA", || RuleConfig::DLA {
                seed_type: 0,
                mobile_type: 1,
                stick_radius: 0.1,
                diffusion_strength: 0.5,
            }),
            ("Refractory", || RuleConfig::Refractory {
                trigger: "signal".into(),
                charge: "energy".into(),
                active_threshold: 0.5,
                depletion_rate: 2.0,
                regen_rate: 0.5,
            }),
        ],
    ),
    (
        "Springs",
        &[
            ("Chain Springs", || RuleConfig::ChainSprings {
                stiffness: 500.0,
                damping: 10.0,
                rest_length: 0.02,
                max_stretch: Some(1.5),
            }),
            ("Radial Springs", || RuleConfig::RadialSprings {
                hub_stiffness: 200.0,
                ring_stiffness: 100.0,
                damping: 5.0,
                hub_length: 0.3,
                ring_length: 0.1,
            }),
            ("Bond Springs", || RuleConfig::BondSprings {
                bonds: vec!["bond0".into()],
                stiffness: 500.0,
                damping: 10.0,
                rest_length: 0.05,
                max_stretch: Some(1.5),
            }),
        ],
    ),
    (
        "State Machine",
        &[
            ("State", || RuleConfig::State {
                field: "state".into(),
                transitions: vec![(0, 1, "p.age > 1.0".into())],
            }),
            ("Agent", || RuleConfig::Agent {
                state_field: "state".into(),
                prev_state_field: "prev_state".into(),
                state_timer_field: Some("state_timer".into()),
                states: vec![
                    AgentStateConfig {
                        id: 0,
                        name: Some("idle".into()),
                        on_enter: None,
                        on_update: None,
                        on_exit: None,
                        transitions: vec![TransitionConfig {
                            to: 1,
                            condition: "p.energy > 0.8".into(),
                            priority: 0,
                        }],
                    },
                    AgentStateConfig {
                        id: 1,
                        name: Some("active".into()),
                        on_enter: None,
                        on_update: None,
                        on_exit: None,
                        transitions: vec![TransitionConfig {
                            to: 0,
                            condition: "p.energy < 0.2".into(),
                            priority: 0,
                        }],
                    },
                ],
            }),
        ],
    ),
    (
        "Conditional",
        &[
            ("Switch", || RuleConfig::Switch {
                condition: "p.particle_type == 0u".into(),
                then_code: "p.velocity.y += 0.1;".into(),
                else_code: None,
            }),
            ("Typed Neighbor", || RuleConfig::TypedNeighbor {
                self_type: Some(0),
                other_type: Some(1),
                radius: 0.2,
                code: "p.velocity += neighbor_dir * 0.5;".into(),
            }),
        ],
    ),
    (
        "Fields",
        &[
            ("Copy Field", || RuleConfig::CopyField {
                from: "age".into(),
                to: "scale".into(),
            }),
            ("Current", || RuleConfig::Current {
                field: "flow".into(),
                strength: 1.0,
            }),
            ("Deposit", || RuleConfig::Deposit {
                field_index: 0,
                source: "energy".into(),
                amount: 1.0,
            }),
            ("Sense", || RuleConfig::Sense {
                field_index: 0,
                target: "sensed".into(),
            }),
            ("Consume", || RuleConfig::Consume {
                field_index: 0,
                target: "consumed".into(),
                rate: 0.5,
            }),
            ("Gradient", || RuleConfig::Gradient {
                field: 0,
                strength: 1.0,
                ascending: true,
            }),
        ],
    ),
    (
        "Neighbor Fields",
        &[
            ("Accumulate", || RuleConfig::Accumulate {
                source: "energy".into(),
                target: "total".into(),
                radius: 0.2,
                operation: "sum".into(),
                falloff: Some(Falloff::Linear),
            }),
            ("Signal", || RuleConfig::Signal {
                source: "signal".into(),
                target: "received".into(),
                radius: 0.3,
                strength: 1.0,
                falloff: Some(Falloff::InverseSquare),
            }),
            ("Absorb", || RuleConfig::Absorb {
                target_type: None,
                radius: 0.1,
                source_field: "energy".into(),
                target_field: "absorbed".into(),
            }),
            ("Density Buoyancy", || RuleConfig::DensityBuoyancy {
                density_field: "density".into(),
                medium_density: 1.0,
                strength: 5.0,
            }),
            ("Diffuse", || RuleConfig::Diffuse {
                field: "heat".into(),
                rate: 0.5,
                radius: 0.1,
            }),
            ("Mass", || RuleConfig::Mass {
                field: "mass".into(),
            }),
        ],
    ),
    (
        "Math",
        &[
            ("Lerp", || RuleConfig::Lerp {
                field: "scale".into(),
                target: 1.0,
                rate: 1.0,
            }),
            ("Clamp", || RuleConfig::Clamp {
                field: "scale".into(),
                min: 0.1,
                max: 2.0,
            }),
            ("Remap", || RuleConfig::Remap {
                field: "age".into(),
                in_min: 0.0,
                in_max: 5.0,
                out_min: 1.0,
                out_max: 0.0,
            }),
            ("Quantize", || RuleConfig::Quantize {
                field: "scale".into(),
                step: 0.25,
            }),
            ("Noise", || RuleConfig::Noise {
                field: "scale".into(),
                amplitude: 0.1,
                frequency: 2.0,
            }),
            ("Smooth", || RuleConfig::Smooth {
                field: "value".into(),
                target: 1.0,
                rate: 2.0,
            }),
            ("Modulo", || RuleConfig::Modulo {
                field: "phase".into(),
                min: 0.0,
                max: std::f32::consts::TAU,
            }),
            ("Copy", || RuleConfig::Copy {
                from: "source".into(),
                to: "dest".into(),
                scale: 1.0,
                offset: 0.0,
            }),
            ("Threshold", || RuleConfig::Threshold {
                input_field: "value".into(),
                output_field: "binary".into(),
                threshold: 0.5,
                above: 1.0,
                below: 0.0,
            }),
            ("Gate", || RuleConfig::Gate {
                condition: "p.age > 1.0".into(),
                action: "p.scale = 2.0;".into(),
            }),
            ("Tween", || RuleConfig::Tween {
                field: "scale".into(),
                from: 0.0,
                to: 1.0,
                duration: 2.0,
                timer_field: "age".into(),
            }),
            ("Periodic", || RuleConfig::Periodic {
                interval: 1.0,
                phase_field: None,
                action: "p.color = vec3(1.0, 0.0, 0.0);".into(),
            }),
        ],
    ),
    (
        "Logic",
        &[
            ("And", || RuleConfig::And {
                a: "input1".into(),
                b: "input2".into(),
                output: "result".into(),
            }),
            ("Or", || RuleConfig::Or {
                a: "input1".into(),
                b: "input2".into(),
                output: "result".into(),
            }),
            ("Not", || RuleConfig::Not {
                input: "value".into(),
                output: "inverted".into(),
                max: 1.0,
            }),
            ("Xor", || RuleConfig::Xor {
                a: "input1".into(),
                b: "input2".into(),
                output: "result".into(),
            }),
            ("Hysteresis", || RuleConfig::Hysteresis {
                input: "value".into(),
                output: "state".into(),
                low_threshold: 0.3,
                high_threshold: 0.7,
                on_value: 1.0,
                off_value: 0.0,
            }),
            ("Latch", || RuleConfig::Latch {
                output: "latched".into(),
                set_condition: "p.trigger > 0.5".into(),
                reset_condition: "p.reset > 0.5".into(),
                set_value: 1.0,
                reset_value: 0.0,
            }),
            ("Edge", || RuleConfig::Edge {
                input: "signal".into(),
                prev_field: "prev_signal".into(),
                output: "pulse".into(),
                threshold: 0.5,
                rising: true,
                falling: false,
            }),
            ("Select", || RuleConfig::Select {
                condition: "p.flag > 0.5".into(),
                then_field: "value_a".into(),
                else_field: "value_b".into(),
                output: "selected".into(),
            }),
            ("Blend", || RuleConfig::Blend {
                a: "color1".into(),
                b: "color2".into(),
                weight: "mix".into(),
                output: "blended".into(),
            }),
        ],
    ),
];
