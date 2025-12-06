//! GPU infrastructure for sub-emitter system.
//!
//! This module handles the death buffer, atomic counters, and child spawning
//! compute pass for the sub-emitter system.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::sub_emitter::{SubEmitter, MAX_DEATH_EVENTS};

/// GPU representation of a death event.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DeathEventGpu {
    pub position: [f32; 3],
    pub parent_type: u32,
    pub velocity: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub _pad1: u32,
}

/// GPU state for sub-emitter system.
pub struct SubEmitterGpu {
    /// Buffer storing death events (kept for potential GPU readback).
    #[allow(dead_code)]
    pub death_buffer: wgpu::Buffer,
    /// Atomic counter for death event count.
    pub death_count_buffer: wgpu::Buffer,
    /// Atomic counter for child slot allocation.
    pub child_slot_buffer: wgpu::Buffer,
    /// Bind group for death recording (group 3).
    pub death_bind_group: wgpu::BindGroup,
    /// Bind group layout for death buffers.
    pub death_bind_group_layout: wgpu::BindGroupLayout,
    /// Child spawning compute pipeline.
    pub spawn_pipeline: wgpu::ComputePipeline,
    /// Bind group for spawn compute pass.
    pub spawn_bind_group: wgpu::BindGroup,
    /// Number of particles (kept for potential debug/stats).
    #[allow(dead_code)]
    pub num_particles: u32,
}

impl SubEmitterGpu {
    /// Create sub-emitter GPU infrastructure.
    pub fn new(
        device: &wgpu::Device,
        particle_buffer: &wgpu::Buffer,
        num_particles: u32,
        sub_emitters: &[SubEmitter],
        particle_wgsl_struct: &str,
    ) -> Self {
        // Create death buffer
        let death_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Death Buffer"),
            size: (MAX_DEATH_EVENTS as usize * std::mem::size_of::<DeathEventGpu>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create death count buffer (atomic u32)
        let death_count_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Death Count Buffer"),
            contents: &[0u8; 4],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create child slot buffer (atomic u32)
        let child_slot_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Child Slot Buffer"),
            contents: &[0u8; 4],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout for death buffers
        let death_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Death Buffer Bind Group Layout"),
                entries: &[
                    // Death buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Death count (atomic)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Child slot counter (atomic)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create death bind group
        let death_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Death Buffer Bind Group"),
            layout: &death_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: death_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: death_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: child_slot_buffer.as_entire_binding(),
                },
            ],
        });

        // Generate spawn shader
        let spawn_shader_src = generate_spawn_shader(particle_wgsl_struct, sub_emitters);

        let spawn_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sub-Emitter Spawn Shader"),
            source: wgpu::ShaderSource::Wgsl(spawn_shader_src.into()),
        });

        // Create spawn pipeline layout
        let spawn_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Spawn Bind Group Layout"),
                entries: &[
                    // Particle buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Death buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Death count
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Child slot counter (atomic for allocation)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let spawn_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Spawn Pipeline Layout"),
                bind_group_layouts: &[&spawn_bind_group_layout],
                push_constant_ranges: &[],
            });

        let spawn_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Sub-Emitter Spawn Pipeline"),
            layout: Some(&spawn_pipeline_layout),
            module: &spawn_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create spawn bind group
        let spawn_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Spawn Bind Group"),
            layout: &spawn_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: death_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: death_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: child_slot_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            death_buffer,
            death_count_buffer,
            child_slot_buffer,
            death_bind_group,
            death_bind_group_layout,
            spawn_pipeline,
            spawn_bind_group,
            num_particles,
        }
    }

    /// Clear death buffers before frame.
    pub fn clear_buffers(&self, queue: &wgpu::Queue) {
        // Clear death count
        queue.write_buffer(&self.death_count_buffer, 0, &[0u8; 4]);
        // Clear child slot counter
        queue.write_buffer(&self.child_slot_buffer, 0, &[0u8; 4]);
    }

    /// Run the child spawning compute pass.
    pub fn spawn_children(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Sub-Emitter Spawn Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.spawn_pipeline);
        compute_pass.set_bind_group(0, &self.spawn_bind_group, &[]);

        // Dispatch one workgroup per potential death event
        let workgroups = MAX_DEATH_EVENTS.div_ceil(256);
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }
}

/// Generate the child spawning compute shader.
fn generate_spawn_shader(particle_wgsl_struct: &str, sub_emitters: &[SubEmitter]) -> String {
    let mut spawn_code = String::new();

    for (i, se) in sub_emitters.iter().enumerate() {
        spawn_code.push_str(&se.child_spawning_wgsl(i));
    }

    format!(
        r#"
// Sub-emitter child spawning shader

{particle_struct}

struct DeathEvent {{
    position: vec3<f32>,
    parent_type: u32,
    velocity: vec3<f32>,
    _pad0: u32,
    color: vec3<f32>,
    _pad1: u32,
}};

struct CountBuffer {{
    count: u32,
}};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read> death_buffer: array<DeathEvent>;

@group(0) @binding(2)
var<storage, read> death_count_buf: CountBuffer;

@group(0) @binding(3)
var<storage, read_write> next_child_slot: atomic<u32>;

// Random functions
fn hash(n: u32) -> u32 {{
    var x = n;
    x = x ^ (x >> 17u);
    x = x * 0xed5ad4bbu;
    x = x ^ (x >> 11u);
    x = x * 0xac4c1b51u;
    x = x ^ (x >> 15u);
    x = x * 0x31848babu;
    x = x ^ (x >> 14u);
    return x;
}}

fn rand(seed: u32) -> f32 {{
    return f32(hash(seed)) / 4294967295.0;
}}

fn rand_sphere(seed: u32) -> vec3<f32> {{
    let v = vec3<f32>(
        rand(seed) * 2.0 - 1.0,
        rand(seed + 1u) * 2.0 - 1.0,
        rand(seed + 2u) * 2.0 - 1.0
    );
    let len = length(v);
    if len < 0.001 {{
        return vec3<f32>(0.0, 1.0, 0.0);
    }}
    return v / len;
}}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let death_idx = global_id.x;
    let total_deaths = death_count_buf.count;

    if death_idx >= total_deaths {{
        return;
    }}

    let death = death_buffer[death_idx];

    // Process sub-emitters
{spawn_code}
}}
"#,
        particle_struct = particle_wgsl_struct,
        spawn_code = spawn_code,
    )
}

/// Generate WGSL code for spawn event recording in main compute shader.
///
/// This handles both death-triggered and condition-triggered sub-emitters:
/// - `OnDeath`: Records when `was_alive == 1u && p.alive == 0u`
/// - `OnCondition`: Records when the custom WGSL condition is true
pub fn death_recording_wgsl(sub_emitters: &[SubEmitter]) -> String {
    use crate::sub_emitter::SpawnTrigger;

    if sub_emitters.is_empty() {
        return String::new();
    }

    let mut code = String::new();
    code.push_str("\n    // Sub-emitter spawn event recording\n");

    // Collect death-triggered emitters (group by parent type)
    let death_emitters: Vec<_> = sub_emitters
        .iter()
        .filter(|se| matches!(se.trigger, SpawnTrigger::OnDeath))
        .collect();

    if !death_emitters.is_empty() {
        let type_checks: Vec<String> = death_emitters
            .iter()
            .map(|se| format!("p.particle_type == {}u", se.parent_type))
            .collect();
        let type_condition = type_checks.join(" || ");

        code.push_str(&format!(
            r#"    // Death-triggered spawn recording
    if was_alive == 1u && p.alive == 0u && ({type_condition}) {{
        let spawn_idx = atomicAdd(&sub_emitter_death_count, 1u);
        if spawn_idx < {max_events}u {{
            sub_emitter_death_buffer[spawn_idx].position = p.position;
            sub_emitter_death_buffer[spawn_idx].velocity = p.velocity;
            sub_emitter_death_buffer[spawn_idx].color = p.color;
            sub_emitter_death_buffer[spawn_idx].parent_type = p.particle_type;
        }}
    }}
"#,
            type_condition = type_condition,
            max_events = MAX_DEATH_EVENTS,
        ));
    }

    // Handle condition-triggered emitters (each gets its own check)
    for (i, se) in sub_emitters.iter().enumerate() {
        if let SpawnTrigger::OnCondition(condition) = &se.trigger {
            code.push_str(&format!(
                r#"
    // Condition-triggered spawn recording (sub-emitter {i})
    // Condition: {condition}
    if p.particle_type == {parent_type}u && ({condition}) {{
        let spawn_idx = atomicAdd(&sub_emitter_death_count, 1u);
        if spawn_idx < {max_events}u {{
            sub_emitter_death_buffer[spawn_idx].position = p.position;
            sub_emitter_death_buffer[spawn_idx].velocity = p.velocity;
            sub_emitter_death_buffer[spawn_idx].color = p.color;
            sub_emitter_death_buffer[spawn_idx].parent_type = p.particle_type;
        }}
    }}
"#,
                i = i,
                condition = condition,
                parent_type = se.parent_type,
                max_events = MAX_DEATH_EVENTS,
            ));
        }
    }

    code
}

/// Generate WGSL bindings for death buffer (to include in main compute shader).
pub fn death_buffer_bindings_wgsl() -> &'static str {
    r#"
struct SubEmitterDeathEvent {
    position: vec3<f32>,
    parent_type: u32,
    velocity: vec3<f32>,
    _pad0: u32,
    color: vec3<f32>,
    _pad1: u32,
};

@group(3) @binding(0)
var<storage, read_write> sub_emitter_death_buffer: array<SubEmitterDeathEvent>;

@group(3) @binding(1)
var<storage, read_write> sub_emitter_death_count: atomic<u32>;

@group(3) @binding(2)
var<storage, read_write> sub_emitter_child_slot: atomic<u32>;
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sub_emitter::SubEmitter;

    #[test]
    fn test_print_death_recording_wgsl() {
        let sub_emitters = vec![
            SubEmitter::new(0, 1)  // ROCKET -> SPARK
                .count(30)
                .speed(0.5..2.0)
                .spread(std::f32::consts::TAU)
                .inherit_velocity(0.2)
                .child_lifetime(1.0)
        ];
        
        let death_recording = death_recording_wgsl(&sub_emitters);
        println!("\n=== Death Recording WGSL ===");
        println!("{}", death_recording);
        
        println!("\n=== Death Buffer Bindings WGSL ===");
        println!("{}", death_buffer_bindings_wgsl());
        
        // Verify it contains expected code
        assert!(death_recording.contains("was_alive == 1u"));
        assert!(death_recording.contains("p.alive == 0u"));
        assert!(death_recording.contains("p.particle_type == 0u"));
        assert!(death_recording.contains("atomicAdd(&sub_emitter_death_count"));
    }
    
    #[test]
    fn test_print_spawn_shader() {
        let sub_emitters = vec![
            SubEmitter::new(0, 1)
                .count(30)
                .speed(0.5..2.0)
                .spread(std::f32::consts::TAU)
                .inherit_velocity(0.2)
        ];
        
        let particle_struct = r#"struct Particle {
    position: vec3<f32>,
    _pad0: f32,
    velocity: vec3<f32>,
    _pad1: f32,
    color: vec3<f32>,
    particle_type: u32,
    age: f32,
    alive: u32,
    scale: f32,
    _pad2: f32,
}"#;
        
        let spawn_shader = generate_spawn_shader(particle_struct, &sub_emitters);
        println!("\n=== Spawn Shader WGSL ===");
        println!("{}", spawn_shader);
        
        // Verify expected content
        assert!(spawn_shader.contains("death.parent_type == 0u"));
        assert!(spawn_shader.contains("child.alive = 1u"));
        assert!(spawn_shader.contains("child.particle_type = 1u"));
    }
}
