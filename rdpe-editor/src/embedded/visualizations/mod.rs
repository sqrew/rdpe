//! Visualization modules for the embedded simulation.
//!
//! This module contains various visualization systems that can be rendered
//! alongside the particle simulation.

mod grid;
mod connections;
mod wireframe;
mod trails;

pub(crate) use grid::GridVisualization;
pub(crate) use connections::ConnectionVisualization;
pub(crate) use wireframe::WireframeVisualization;
pub(crate) use trails::TrailVisualization;
