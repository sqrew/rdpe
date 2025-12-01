use bytemuck::{Pod, Zeroable};

pub const SHADER_SOURCE: &str = include_str!("shader.wgsl");
pub const COMPUTE_SOURCE: &str = include_str!("compute.wgsl");

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Particle {
    pub position: [f32; 3],
    pub _pad0: f32,
    pub velocity: [f32; 3],
    pub _pad1: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub time: f32,
    pub delta_time: f32,
    pub _padding: [f32; 2],
}
