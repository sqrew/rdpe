//! Shader validation using naga.
//!
//! This module validates WGSL shaders before they're compiled by wgpu,
//! providing better error messages and preventing crashes.

use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};

/// Shader validation error with helpful context.
#[derive(Debug, Clone)]
pub struct ShaderError {
    pub message: String,
    pub stage: &'static str,
}

impl std::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} shader error: {}", self.stage, self.message)
    }
}

/// Validate a WGSL compute shader.
pub fn validate_compute_shader(source: &str) -> Result<(), ShaderError> {
    validate_wgsl(source, "Compute")
}

/// Validate a WGSL render shader (vertex + fragment).
pub fn validate_render_shader(source: &str) -> Result<(), ShaderError> {
    validate_wgsl(source, "Render")
}

/// Validate WGSL source code.
fn validate_wgsl(source: &str, stage: &'static str) -> Result<(), ShaderError> {
    // Parse the WGSL
    let module = match wgsl::parse_str(source) {
        Ok(module) => module,
        Err(err) => {
            // Format the parse error nicely
            let message = err.emit_to_string(source);
            return Err(ShaderError {
                message,
                stage,
            });
        }
    };

    // Validate the module
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    if let Err(err) = validator.validate(&module) {
        let message = format!("{}", err);
        return Err(ShaderError {
            message,
            stage,
        });
    }

    Ok(())
}

/// Validate both compute and render shaders, returning all errors.
pub fn validate_shaders(
    compute_src: &str,
    render_src: &str,
) -> Result<(), Vec<ShaderError>> {
    let mut errors = Vec::new();

    if let Err(e) = validate_compute_shader(compute_src) {
        errors.push(e);
    }

    if let Err(e) = validate_render_shader(render_src) {
        errors.push(e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
