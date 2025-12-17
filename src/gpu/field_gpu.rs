//! GPU resources and compute pipelines for 3D spatial fields.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::field::{FieldConfig, FieldRegistry};

/// Parameters for a single field, uploaded to GPU.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FieldParamsGpu {
    pub resolution: u32,
    pub total_cells: u32,
    pub extent: f32,
    pub decay: f32,
    pub blur: f32,
    /// Field type: 0 = scalar, 1 = vector
    pub field_type: u32,
    pub _pad: [f32; 2],
}

/// Time uniforms for custom field update shaders.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FieldTimeUniforms {
    pub time: f32,
    pub delta_time: f32,
}

/// GPU state for a single 3D field.
pub struct SingleFieldGpu {
    /// Atomic write buffer - particles deposit here
    pub write_buffer: wgpu::Buffer,
    /// Read buffer A - particles sample from here
    pub read_buffer_a: wgpu::Buffer,
    /// Read buffer B - for double buffering during blur
    pub read_buffer_b: wgpu::Buffer,
    /// Which buffer is currently the "read" buffer (false = A, true = B)
    pub read_is_b: bool,
    /// Field configuration
    pub config: FieldConfig,
    /// Field index (for future multi-field support)
    #[allow(dead_code)]
    pub index: usize,
    /// Custom update pipeline (if custom_update code is provided)
    pub custom_pipeline: Option<wgpu::ComputePipeline>,
    /// Custom update bind group layout
    pub custom_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl SingleFieldGpu {
    pub fn new(device: &wgpu::Device, config: &FieldConfig, index: usize) -> Self {
        let total_cells = config.total_cells() as usize;
        // Vector fields need 3x the storage (one f32 per component)
        let components = config.field_type.components() as usize;
        let buffer_elements = total_cells * components;

        // Write buffer: atomic i32 for parallel particle deposits
        let write_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Field {} Write Buffer", index)),
            size: (buffer_elements * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Read buffers: f32 for particle sampling (double-buffered for blur)
        let read_buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Field {} Read Buffer A", index)),
            size: (buffer_elements * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let read_buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Field {} Read Buffer B", index)),
            size: (buffer_elements * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create custom update pipeline if custom code is provided
        let (custom_pipeline, custom_bind_group_layout) = if let Some(ref code) = config.custom_update {
            let (pipeline, layout) = create_custom_update_pipeline(device, code, config.is_vector());
            (Some(pipeline), Some(layout))
        } else {
            (None, None)
        };

        Self {
            write_buffer,
            read_buffer_a,
            read_buffer_b,
            read_is_b: false,
            config: config.clone(),
            index,
            custom_pipeline,
            custom_bind_group_layout,
        }
    }

    /// Get the current read buffer
    pub fn current_read_buffer(&self) -> &wgpu::Buffer {
        if self.read_is_b {
            &self.read_buffer_b
        } else {
            &self.read_buffer_a
        }
    }

    /// Get the buffer to write blur output to
    pub fn blur_target_buffer(&self) -> &wgpu::Buffer {
        if self.read_is_b {
            &self.read_buffer_a
        } else {
            &self.read_buffer_b
        }
    }

    /// Swap read buffers after blur
    pub fn swap_buffers(&mut self) {
        self.read_is_b = !self.read_is_b;
    }

    /// Read back field data from GPU to CPU.
    ///
    /// Returns the field data as a Vec of f32 values.
    /// For scalar fields: one value per cell.
    /// For vector fields: three values (x, y, z) per cell.
    ///
    /// This is a blocking operation and should be used sparingly.
    pub fn read_field_data(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<f32> {
        let total_cells = self.config.total_cells() as usize;
        let components = self.config.field_type.components() as usize;
        let buffer_elements = total_cells * components;
        let buffer_size = (buffer_elements * 4) as u64;

        // Create staging buffer for readback
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Field Readback Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy from current read buffer to staging
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Field Readback Encoder"),
        });
        encoder.copy_buffer_to_buffer(
            self.current_read_buffer(),
            0,
            &staging_buffer,
            0,
            buffer_size,
        );
        queue.submit(std::iter::once(encoder.finish()));

        // Map and read the staging buffer
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();

        result
    }

    /// Sample the field at a world position.
    ///
    /// `data` should be the result of `read_field_data()`.
    /// Returns the interpolated value at the given position.
    pub fn sample_at(&self, data: &[f32], pos: glam::Vec3) -> glam::Vec3 {
        let res = self.config.resolution as f32;
        let extent = self.config.world_extent;
        let components = self.config.field_type.components() as usize;

        // Convert world position to grid coordinates
        let normalized = (pos + glam::Vec3::splat(extent)) / (2.0 * extent);
        let grid_pos = normalized * res;

        // Clamp to valid range
        let x = grid_pos.x.clamp(0.0, res - 1.001);
        let y = grid_pos.y.clamp(0.0, res - 1.001);
        let z = grid_pos.z.clamp(0.0, res - 1.001);

        // Get integer indices
        let ix = x as usize;
        let iy = y as usize;
        let iz = z as usize;

        let res_u = self.config.resolution as usize;

        // Helper to get value at grid index
        let get_value = |gx: usize, gy: usize, gz: usize| -> glam::Vec3 {
            let idx = (gz * res_u * res_u + gy * res_u + gx) * components;
            if components == 3 && idx + 2 < data.len() {
                glam::Vec3::new(data[idx], data[idx + 1], data[idx + 2])
            } else if components == 1 && idx < data.len() {
                // Scalar field - return as (value, 0, 0)
                glam::Vec3::new(data[idx], 0.0, 0.0)
            } else {
                glam::Vec3::ZERO
            }
        };

        // Simple nearest-neighbor sampling (could be improved to trilinear)
        get_value(ix, iy, iz)
    }
}

/// GPU state for all fields in a simulation.
pub struct FieldSystemGpu {
    /// Individual field GPU states
    pub fields: Vec<SingleFieldGpu>,
    /// Parameters buffer (storage buffer with array of FieldParams)
    pub params_buffer: wgpu::Buffer,
    /// Number of fields
    pub field_count: usize,
    /// Merge pipeline (atomic writes → float field)
    pub merge_pipeline: wgpu::ComputePipeline,
    pub merge_bind_group_layout: wgpu::BindGroupLayout,
    /// Blur/decay pipeline
    pub blur_decay_pipeline: wgpu::ComputePipeline,
    pub blur_decay_bind_group_layout: wgpu::BindGroupLayout,
    /// Clear pipeline (reset atomic buffers to zero)
    pub clear_pipeline: wgpu::ComputePipeline,
    pub clear_bind_group_layout: wgpu::BindGroupLayout,
}

impl FieldSystemGpu {
    pub fn new(device: &wgpu::Device, registry: &FieldRegistry) -> Self {
        let field_count = registry.fields.len();

        // Create GPU state for each field
        let fields: Vec<_> = registry
            .fields
            .iter()
            .enumerate()
            .map(|(i, (_, config))| SingleFieldGpu::new(device, config, i))
            .collect();

        // Create params buffer as storage buffer (array of FieldParams)
        let params: Vec<FieldParamsGpu> = registry
            .fields
            .iter()
            .map(|(_, config)| FieldParamsGpu {
                resolution: config.resolution,
                total_cells: config.total_cells(),
                extent: config.world_extent,
                decay: config.decay,
                blur: config.blur,
                field_type: if config.is_vector() { 1 } else { 0 },
                _pad: [0.0; 2],
            })
            .collect();

        let params_buffer = if params.is_empty() {
            // Create a dummy buffer if no fields
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Field Params Buffer (empty)"),
                size: 32,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Field Params Buffer"),
                contents: bytemuck::cast_slice(&params),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };

        // Create pipelines
        let (merge_pipeline, merge_bind_group_layout) = create_merge_pipeline(device);
        let (blur_decay_pipeline, blur_decay_bind_group_layout) = create_blur_decay_pipeline(device);
        let (clear_pipeline, clear_bind_group_layout) = create_clear_pipeline(device);

        Self {
            fields,
            params_buffer,
            field_count,
            merge_pipeline,
            merge_bind_group_layout,
            blur_decay_pipeline,
            blur_decay_bind_group_layout,
            clear_pipeline,
            clear_bind_group_layout,
        }
    }

    /// Run field processing: merge deposits, blur/decay or custom update, clear write buffer
    ///
    /// If a field has custom_update code, it replaces the blur/decay step with the custom shader.
    pub fn process(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        _queue: &wgpu::Queue,
        time: f32,
        delta_time: f32,
    ) {
        // Create time uniforms buffer (shared across all fields)
        let time_uniforms = FieldTimeUniforms { time, delta_time };
        let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Field Time Uniforms"),
            contents: bytemuck::bytes_of(&time_uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        for field in &mut self.fields {
            let total_cells = field.config.total_cells();
            let components = field.config.field_type.components();
            let buffer_elements = total_cells * components;
            // Workgroups for merge/clear (process buffer elements)
            let element_workgroups = buffer_elements.div_ceil(256);
            // Workgroups for blur/custom (process cells, loop over components internally)
            let cell_workgroups = total_cells.div_ceil(256);

            // Create params for this field
            let params = FieldParamsGpu {
                resolution: field.config.resolution,
                total_cells: field.config.total_cells(),
                extent: field.config.world_extent,
                decay: field.config.decay,
                blur: field.config.blur,
                field_type: if field.config.is_vector() { 1 } else { 0 },
                _pad: [0.0; 2],
            };
            let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Field Process Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            // Step 1: Merge atomic writes into read buffer
            let merge_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Field Merge Bind Group"),
                layout: &self.merge_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: field.write_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: field.current_read_buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Field Merge Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.merge_pipeline);
                pass.set_bind_group(0, &merge_bind_group, &[]);
                pass.dispatch_workgroups(element_workgroups, 1, 1);
            }

            // Step 2: Update field - either custom shader or default blur/decay
            if let (Some(ref pipeline), Some(ref layout)) = (&field.custom_pipeline, &field.custom_bind_group_layout) {
                // Custom update shader (replaces blur/decay)
                let custom_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Field Custom Update Bind Group"),
                    layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: field.current_read_buffer().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: field.blur_target_buffer().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: params_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: time_buffer.as_entire_binding(),
                        },
                    ],
                });

                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Field Custom Update Pass"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &custom_bind_group, &[]);
                    pass.dispatch_workgroups(cell_workgroups, 1, 1);
                }

                field.swap_buffers();
            } else {
                // Default blur and decay (if enabled)
                for _ in 0..field.config.blur_iterations {
                    if field.config.blur > 0.0 || field.config.decay < 1.0 {
                        let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("Field Blur Bind Group"),
                            layout: &self.blur_decay_bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: field.current_read_buffer().as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: field.blur_target_buffer().as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: params_buffer.as_entire_binding(),
                                },
                            ],
                        });

                        {
                            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                label: Some("Field Blur/Decay Pass"),
                                timestamp_writes: None,
                            });
                            pass.set_pipeline(&self.blur_decay_pipeline);
                            pass.set_bind_group(0, &blur_bind_group, &[]);
                            pass.dispatch_workgroups(cell_workgroups, 1, 1);
                        }

                        field.swap_buffers();
                    }
                }
            }

            // Step 3: Clear write buffer for next frame
            let clear_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Field Clear Bind Group"),
                layout: &self.clear_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: field.write_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Field Clear Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.clear_pipeline);
                pass.set_bind_group(0, &clear_bind_group, &[]);
                pass.dispatch_workgroups(element_workgroups, 1, 1);
            }
        }
    }

    /// Create bind group for particle compute shader access
    pub fn create_particle_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> Option<wgpu::BindGroup> {
        if self.fields.is_empty() {
            return None;
        }

        // Build entries for all fields: 2 bindings per field (write + read) + 1 params buffer
        let mut entries = Vec::new();
        let mut binding = 0u32;

        for field in &self.fields {
            // Write buffer for this field
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: field.write_buffer.as_entire_binding(),
            });
            binding += 1;

            // Read buffer for this field
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: field.current_read_buffer().as_entire_binding(),
            });
            binding += 1;
        }

        // Params buffer (storage array of all field params)
        entries.push(wgpu::BindGroupEntry {
            binding,
            resource: self.params_buffer.as_entire_binding(),
        });

        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Field Particle Bind Group"),
            layout,
            entries: &entries,
        }))
    }
}

fn create_merge_pipeline(
    device: &wgpu::Device,
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Field Merge Shader"),
        source: wgpu::ShaderSource::Wgsl(MERGE_SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Field Merge Bind Group Layout"),
        entries: &[
            // Write buffer (atomic i32)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Read buffer (f32, read_write for merging)
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
            // Params
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Field Merge Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Field Merge Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    (pipeline, bind_group_layout)
}

fn create_blur_decay_pipeline(
    device: &wgpu::Device,
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Field Blur/Decay Shader"),
        source: wgpu::ShaderSource::Wgsl(BLUR_DECAY_SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Field Blur Bind Group Layout"),
        entries: &[
            // Source buffer
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Destination buffer
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
            // Params
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Field Blur Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Field Blur/Decay Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    (pipeline, bind_group_layout)
}

fn create_clear_pipeline(
    device: &wgpu::Device,
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Field Clear Shader"),
        source: wgpu::ShaderSource::Wgsl(CLEAR_SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Field Clear Bind Group Layout"),
        entries: &[
            // Write buffer to clear
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
            // Params
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Field Clear Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Field Clear Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    (pipeline, bind_group_layout)
}

/// Create a custom field update pipeline with user-provided WGSL code.
fn create_custom_update_pipeline(
    device: &wgpu::Device,
    user_code: &str,
    is_vector: bool,
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    let shader_src = generate_custom_update_shader(user_code, is_vector);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Custom Field Update Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    // Same layout as blur/decay: src (read), dst (write), params (uniform), time_uniforms (uniform)
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Custom Field Update Bind Group Layout"),
        entries: &[
            // Source buffer
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Destination buffer
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
            // Field params
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Time uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Custom Field Update Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Custom Field Update Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    (pipeline, bind_group_layout)
}

/// Generate the full WGSL shader for custom field updates.
fn generate_custom_update_shader(user_code: &str, is_vector: bool) -> String {
    let value_type = if is_vector { "vec3<f32>" } else { "f32" };
    let read_neighbor_fn = if is_vector {
        r#"fn read_neighbor(dx: i32, dy: i32, dz: i32) -> vec3<f32> {
    let nx = clamp(i32(pos.x) + dx, 0, i32(params.resolution) - 1);
    let ny = clamp(i32(pos.y) + dy, 0, i32(params.resolution) - 1);
    let nz = clamp(i32(pos.z) + dz, 0, i32(params.resolution) - 1);
    let idx = u32(nx) + u32(ny) * params.resolution + u32(nz) * params.resolution * params.resolution;
    return vec3<f32>(src[idx * 3u], src[idx * 3u + 1u], src[idx * 3u + 2u]);
}"#
    } else {
        r#"fn read_neighbor(dx: i32, dy: i32, dz: i32) -> f32 {
    let nx = clamp(i32(pos.x) + dx, 0, i32(params.resolution) - 1);
    let ny = clamp(i32(pos.y) + dy, 0, i32(params.resolution) - 1);
    let nz = clamp(i32(pos.z) + dz, 0, i32(params.resolution) - 1);
    let idx = u32(nx) + u32(ny) * params.resolution + u32(nz) * params.resolution * params.resolution;
    return src[idx];
}"#
    };

    let read_value = if is_vector {
        "let value = vec3<f32>(src[cell_idx * 3u], src[cell_idx * 3u + 1u], src[cell_idx * 3u + 2u]);"
    } else {
        "let value = src[cell_idx];"
    };

    let write_value = if is_vector {
        r#"dst[cell_idx * 3u] = new_value.x;
    dst[cell_idx * 3u + 1u] = new_value.y;
    dst[cell_idx * 3u + 2u] = new_value.z;"#
    } else {
        "dst[cell_idx] = new_value;"
    };

    format!(
        r#"// Custom Field Update Shader

struct Params {{
    resolution: u32,
    total_cells: u32,
    extent: f32,
    decay: f32,
    blur: f32,
    field_type: u32,
    _pad1: f32,
    _pad2: f32,
}};

struct TimeUniforms {{
    time: f32,
    delta_time: f32,
}};

@group(0) @binding(0)
var<storage, read> src: array<f32>;

@group(0) @binding(1)
var<storage, read_write> dst: array<f32>;

@group(0) @binding(2)
var<uniform> params: Params;

@group(0) @binding(3)
var<uniform> uniforms: TimeUniforms;

// Global position for read_neighbor to access
var<private> pos: vec3<u32>;

fn idx_3d(x: u32, y: u32, z: u32) -> u32 {{
    return x + y * params.resolution + z * params.resolution * params.resolution;
}}

fn idx_to_3d(idx: u32) -> vec3<u32> {{
    let res = params.resolution;
    let z = idx / (res * res);
    let remainder = idx % (res * res);
    let y = remainder / res;
    let x = remainder % res;
    return vec3<u32>(x, y, z);
}}

fn cell_to_world(cell_pos: vec3<u32>) -> vec3<f32> {{
    let res = f32(params.resolution);
    let normalized = (vec3<f32>(cell_pos) + 0.5) / res;
    return (normalized * 2.0 - 1.0) * params.extent;
}}

{read_neighbor_fn}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let cell_idx = global_id.x;
    if cell_idx >= params.total_cells {{
        return;
    }}

    pos = idx_to_3d(cell_idx);
    let world_pos = cell_to_world(pos);

    // Read current value
    {read_value}

    // Initialize new_value (user code should modify this)
    var new_value: {value_type} = value;

    // --- User code begins ---
{user_code}
    // --- User code ends ---

    // Write result
    {write_value}
}}
"#,
        read_neighbor_fn = read_neighbor_fn,
        read_value = read_value,
        write_value = write_value,
        value_type = value_type,
        user_code = user_code,
    )
}

/// Shader to merge atomic writes into the float field
const MERGE_SHADER: &str = r#"
struct Params {
    resolution: u32,
    total_cells: u32,
    extent: f32,
    decay: f32,
    blur: f32,
    field_type: u32,  // 0 = scalar, 1 = vector
    _pad1: f32,
    _pad2: f32,
};

const FIELD_SCALE: f32 = 65536.0;

@group(0) @binding(0)
var<storage, read> write_buffer: array<i32>;

@group(0) @binding(1)
var<storage, read_write> read_buffer: array<f32>;

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    // Buffer size is total_cells * components (1 for scalar, 3 for vector)
    let components = select(1u, 3u, params.field_type == 1u);
    let buffer_size = params.total_cells * components;
    if idx >= buffer_size {
        return;
    }

    // Read atomic value and convert from fixed-point
    let deposited = f32(write_buffer[idx]) / FIELD_SCALE;

    // Add to existing field value
    read_buffer[idx] = read_buffer[idx] + deposited;
}
"#;

/// Shader for blur and decay
const BLUR_DECAY_SHADER: &str = r#"
struct Params {
    resolution: u32,
    total_cells: u32,
    extent: f32,
    decay: f32,
    blur: f32,
    field_type: u32,  // 0 = scalar, 1 = vector
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0)
var<storage, read> src: array<f32>;

@group(0) @binding(1)
var<storage, read_write> dst: array<f32>;

@group(0) @binding(2)
var<uniform> params: Params;

fn idx_3d(x: u32, y: u32, z: u32) -> u32 {
    return x + y * params.resolution + z * params.resolution * params.resolution;
}

fn idx_to_3d(idx: u32) -> vec3<u32> {
    let res = params.resolution;
    let z = idx / (res * res);
    let remainder = idx % (res * res);
    let y = remainder / res;
    let x = remainder % res;
    return vec3<u32>(x, y, z);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell_idx = global_id.x;
    if cell_idx >= params.total_cells {
        return;
    }

    let pos = idx_to_3d(cell_idx);
    let res = params.resolution;
    let components = select(1u, 3u, params.field_type == 1u);

    // Process all components for this cell
    for (var c = 0u; c < components; c = c + 1u) {
        let idx = cell_idx * components + c;

        // Sample center and 6 neighbors for simple 3D blur
        var sum = src[idx];
        var count = 1.0;

        // Only blur if blur > 0
        if params.blur > 0.0 {
            // X neighbors
            if pos.x > 0u {
                sum += src[idx_3d(pos.x - 1u, pos.y, pos.z) * components + c] * params.blur;
                count += params.blur;
            }
            if pos.x < res - 1u {
                sum += src[idx_3d(pos.x + 1u, pos.y, pos.z) * components + c] * params.blur;
                count += params.blur;
            }

            // Y neighbors
            if pos.y > 0u {
                sum += src[idx_3d(pos.x, pos.y - 1u, pos.z) * components + c] * params.blur;
                count += params.blur;
            }
            if pos.y < res - 1u {
                sum += src[idx_3d(pos.x, pos.y + 1u, pos.z) * components + c] * params.blur;
                count += params.blur;
            }

            // Z neighbors
            if pos.z > 0u {
                sum += src[idx_3d(pos.x, pos.y, pos.z - 1u) * components + c] * params.blur;
                count += params.blur;
            }
            if pos.z < res - 1u {
                sum += src[idx_3d(pos.x, pos.y, pos.z + 1u) * components + c] * params.blur;
                count += params.blur;
            }
        }

        // Average and apply decay
        dst[idx] = (sum / count) * params.decay;
    }
}
"#;

/// Shader to clear atomic write buffer
const CLEAR_SHADER: &str = r#"
struct Params {
    resolution: u32,
    total_cells: u32,
    extent: f32,
    decay: f32,
    blur: f32,
    field_type: u32,  // 0 = scalar, 1 = vector
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0)
var<storage, read_write> write_buffer: array<atomic<i32>>;

@group(0) @binding(1)
var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    // Buffer size is total_cells * components (1 for scalar, 3 for vector)
    let components = select(1u, 3u, params.field_type == 1u);
    let buffer_size = params.total_cells * components;
    if idx >= buffer_size {
        return;
    }

    atomicStore(&write_buffer[idx], 0);
}
"#;

/// Create bind group layout for particle shader field access
///
/// Layout: 2 bindings per field (write + read) + 1 params storage buffer
pub fn create_particle_field_bind_group_layout(
    device: &wgpu::Device,
    field_count: usize,
) -> wgpu::BindGroupLayout {
    let mut entries = Vec::new();
    let mut binding = 0u32;

    // Create entries for each field: write buffer + read buffer
    for _ in 0..field_count {
        // Write buffer (atomic for deposits)
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        binding += 1;

        // Read buffer (f32 for sampling)
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        binding += 1;
    }

    // Params storage buffer (array of FieldParams)
    entries.push(wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    });

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Particle Field Bind Group Layout"),
        entries: &entries,
    })
}
