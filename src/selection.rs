//! Particle selection utilities.
//!
//! This module provides helpers for working with particle selection,
//! particularly in egui UI callbacks.
//!
//! # Example
//!
//! ```ignore
//! use rdpe::selection::{selected_particle, selected_particle_data};
//!
//! #[derive(Particle, Clone)]
//! struct MyParticle {
//!     position: Vec3,
//!     velocity: Vec3,
//!     energy: f32,
//! }
//!
//! Simulation::<MyParticle>::new()
//!     .with_egui()
//!     .with_ui(|ctx| {
//!         egui::Window::new("Selection").show(ctx, |ui| {
//!             if let Some(particle) = selected_particle_data::<MyParticle>(ctx) {
//!                 ui.label(format!("Position: {:?}", particle.position));
//!                 ui.label(format!("Velocity: {:?}", particle.velocity));
//!                 ui.label(format!("Energy: {:.2}", particle.energy));
//!             } else {
//!                 ui.label("Click a particle to select it");
//!             }
//!         });
//!     })
//!     .run();
//! ```

use crate::ParticleTrait;

/// Internal wrapper for storing selection index in egui's data.
#[derive(Clone, Copy)]
pub struct SelectedParticle(pub Option<u32>);

/// Internal wrapper for storing selection data in egui's data.
#[derive(Clone)]
pub struct SelectedParticleData(pub Option<Vec<u8>>);

/// Internal wrapper for storing pending particle writes.
/// Contains (particle_index, gpu_bytes) when a particle has been edited.
#[derive(Clone)]
pub struct PendingParticleWrite(pub Option<(u32, Vec<u8>)>);

/// Get the currently selected particle index from an egui context.
///
/// Returns `Some(index)` if a particle is selected, `None` otherwise.
///
/// This works by reading from egui's temporary data storage, which is
/// populated by the simulation's render loop before calling your UI callback.
///
/// # Example
///
/// ```ignore
/// .with_ui(|ctx| {
///     if let Some(particle_idx) = rdpe::selection::selected_particle(ctx) {
///         // Show info about the selected particle
///     }
/// })
/// ```
#[cfg(feature = "egui")]
pub fn selected_particle(ctx: &egui::Context) -> Option<u32> {
    ctx.data(|d| d.get_temp::<SelectedParticle>(egui::Id::NULL))
        .and_then(|sp| sp.0)
}

/// Get the currently selected particle's data from an egui context.
///
/// Returns `Some(particle)` if a particle is selected and its data has been
/// read back from the GPU, `None` otherwise.
///
/// The particle data is read from the GPU one frame after selection, so this
/// may return `None` briefly after a new selection is made.
///
/// # Type Parameter
///
/// `P` must be the same particle type used in your simulation. The function
/// will convert the raw GPU bytes back into your particle struct.
///
/// # Example
///
/// ```ignore
/// .with_ui(|ctx| {
///     if let Some(particle) = selected_particle_data::<MyParticle>(ctx) {
///         ui.label(format!("Position: {:?}", particle.position));
///         ui.label(format!("Energy: {:.2}", particle.energy));
///     }
/// })
/// ```
#[cfg(feature = "egui")]
pub fn selected_particle_data<P: ParticleTrait>(ctx: &egui::Context) -> Option<P> {
    ctx.data(|d| d.get_temp::<SelectedParticleData>(egui::Id::NULL))
        .and_then(|spd| spd.0)
        .and_then(|bytes| {
            // Convert raw bytes to GPU struct, then to Rust struct
            if bytes.len() >= std::mem::size_of::<P::Gpu>() {
                let gpu_particle: &P::Gpu = bytemuck::from_bytes(&bytes[..std::mem::size_of::<P::Gpu>()]);
                Some(P::from_gpu(gpu_particle))
            } else {
                None
            }
        })
}

/// Queue a modified particle to be written back to the GPU.
///
/// Call this after editing a particle's fields to push the changes to the GPU.
/// The write will be applied on the next frame.
///
/// # Example
///
/// ```ignore
/// .with_ui(|ctx| {
///     if let Some(mut particle) = selected_particle_data::<MyParticle>(ctx) {
///         // Edit the particle
///         particle.velocity = Vec3::ZERO;
///
///         // Queue the write
///         if let Some(idx) = selected_particle(ctx) {
///             write_particle(ctx, idx, &particle);
///         }
///     }
/// })
/// ```
#[cfg(feature = "egui")]
pub fn write_particle<P: ParticleTrait>(ctx: &egui::Context, index: u32, particle: &P) {
    let gpu = particle.to_gpu();
    let bytes = bytemuck::bytes_of(&gpu).to_vec();
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::NULL, PendingParticleWrite(Some((index, bytes))));
    });
}
