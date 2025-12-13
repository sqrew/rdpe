//! Error types for RDPE.
//!
//! This module provides error types for GPU initialization, texture loading,
//! and other operations that can fail.

use std::fmt;

/// Errors that can occur during GPU initialization.
#[derive(Debug)]
pub enum GpuError {
    /// Failed to create a surface for rendering.
    SurfaceCreation(wgpu::CreateSurfaceError),
    /// No compatible GPU adapter found.
    NoAdapter,
    /// Failed to create GPU device.
    DeviceCreation(wgpu::RequestDeviceError),
    /// Failed to map buffer for reading.
    BufferMapping(String),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuError::SurfaceCreation(e) => write!(f, "Failed to create GPU surface: {}", e),
            GpuError::NoAdapter => write!(f, "No compatible GPU adapter found. Ensure your system has a GPU with WebGPU/Vulkan/Metal/DX12 support."),
            GpuError::DeviceCreation(e) => write!(f, "Failed to create GPU device: {}", e),
            GpuError::BufferMapping(msg) => write!(f, "Failed to map GPU buffer: {}", msg),
        }
    }
}

impl std::error::Error for GpuError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GpuError::SurfaceCreation(e) => Some(e),
            GpuError::DeviceCreation(e) => Some(e),
            _ => None,
        }
    }
}

impl From<wgpu::CreateSurfaceError> for GpuError {
    fn from(e: wgpu::CreateSurfaceError) -> Self {
        GpuError::SurfaceCreation(e)
    }
}

impl From<wgpu::RequestDeviceError> for GpuError {
    fn from(e: wgpu::RequestDeviceError) -> Self {
        GpuError::DeviceCreation(e)
    }
}

/// Errors that can occur during texture loading.
#[derive(Debug)]
pub enum TextureError {
    /// Failed to load image file.
    ImageLoad(image::ImageError),
    /// Failed to read file from disk.
    Io(std::io::Error),
}

impl fmt::Display for TextureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureError::ImageLoad(e) => write!(f, "Failed to load image: {}", e),
            TextureError::Io(e) => write!(f, "Failed to read texture file: {}", e),
        }
    }
}

impl std::error::Error for TextureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TextureError::ImageLoad(e) => Some(e),
            TextureError::Io(e) => Some(e),
        }
    }
}

impl From<image::ImageError> for TextureError {
    fn from(e: image::ImageError) -> Self {
        TextureError::ImageLoad(e)
    }
}

impl From<std::io::Error> for TextureError {
    fn from(e: std::io::Error) -> Self {
        TextureError::Io(e)
    }
}

/// Errors that can occur when running a simulation.
#[derive(Debug)]
pub enum SimulationError {
    /// Failed to create event loop.
    EventLoop(winit::error::EventLoopError),
    /// Failed to create window.
    Window(winit::error::OsError),
    /// GPU initialization failed.
    Gpu(GpuError),
    /// No spawner function provided.
    NoSpawner,
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimulationError::EventLoop(e) => write!(f, "Failed to create event loop: {}", e),
            SimulationError::Window(e) => write!(f, "Failed to create window: {}", e),
            SimulationError::Gpu(e) => write!(f, "GPU error: {}", e),
            SimulationError::NoSpawner => write!(f, "No spawner function provided. Use .with_spawner() to set one."),
        }
    }
}

impl std::error::Error for SimulationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SimulationError::EventLoop(e) => Some(e),
            SimulationError::Window(e) => Some(e),
            SimulationError::Gpu(e) => Some(e),
            SimulationError::NoSpawner => None,
        }
    }
}

impl From<winit::error::EventLoopError> for SimulationError {
    fn from(e: winit::error::EventLoopError) -> Self {
        SimulationError::EventLoop(e)
    }
}

impl From<winit::error::OsError> for SimulationError {
    fn from(e: winit::error::OsError) -> Self {
        SimulationError::Window(e)
    }
}

impl From<GpuError> for SimulationError {
    fn from(e: GpuError) -> Self {
        SimulationError::Gpu(e)
    }
}
