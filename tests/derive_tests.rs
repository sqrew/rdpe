//! Integration tests for derive macros.
//!
//! These tests verify that the `#[derive(Particle)]` and `#[derive(ParticleType)]`
//! macros generate correct code by actually using the derived implementations.

use glam::{Vec2, Vec3, Vec4};
use rdpe::{Particle, ParticleTrait, ParticleType};

// ============================================================================
// ParticleType Derive Tests
// ============================================================================

#[derive(ParticleType, Clone, Copy, PartialEq, Debug)]
enum Species {
    Prey,
    Predator,
    Plant,
}

#[test]
fn test_particle_type_into_u32() {
    let prey: u32 = Species::Prey.into();
    let predator: u32 = Species::Predator.into();
    let plant: u32 = Species::Plant.into();

    assert_eq!(prey, 0);
    assert_eq!(predator, 1);
    assert_eq!(plant, 2);
}

#[test]
fn test_particle_type_from_u32() {
    let prey: Species = 0u32.into();
    let predator: Species = 1u32.into();
    let plant: Species = 2u32.into();

    assert_eq!(prey, Species::Prey);
    assert_eq!(predator, Species::Predator);
    assert_eq!(plant, Species::Plant);
}

#[test]
fn test_particle_type_invalid_u32_defaults_to_first() {
    let invalid: Species = 99u32.into();
    assert_eq!(invalid, Species::Prey); // Should default to first variant
}

#[test]
fn test_particle_type_count() {
    assert_eq!(Species::count(), 3);
}

#[derive(ParticleType, Clone, Copy, PartialEq, Debug)]
enum SingleVariant {
    Only,
}

#[test]
fn test_single_variant_particle_type() {
    assert_eq!(SingleVariant::count(), 1);
    let only: u32 = SingleVariant::Only.into();
    assert_eq!(only, 0);
    let back: SingleVariant = 0u32.into();
    assert_eq!(back, SingleVariant::Only);
}

#[derive(ParticleType, Clone, Copy, PartialEq, Debug)]
enum ManyVariants {
    A, B, C, D, E, F, G, H,
}

#[test]
fn test_many_variants_particle_type() {
    assert_eq!(ManyVariants::count(), 8);
    assert_eq!(u32::from(ManyVariants::A), 0);
    assert_eq!(u32::from(ManyVariants::H), 7);
}

// ============================================================================
// Particle Derive Tests - Basic Structs
// ============================================================================

#[derive(Particle, Clone)]
struct MinimalParticle {
    position: Vec3,
    velocity: Vec3,
}

#[test]
fn test_minimal_particle_wgsl_struct() {
    let wgsl = MinimalParticle::WGSL_STRUCT;

    // Should contain required fields
    assert!(wgsl.contains("position: vec3<f32>"));
    assert!(wgsl.contains("velocity: vec3<f32>"));

    // Should have auto-injected particle_type
    assert!(wgsl.contains("particle_type: u32"));

    // Should have lifecycle fields
    assert!(wgsl.contains("age: f32"));
    assert!(wgsl.contains("alive: u32"));
    assert!(wgsl.contains("scale: f32"));
}

#[test]
fn test_minimal_particle_to_gpu() {
    let p = MinimalParticle {
        position: Vec3::new(1.0, 2.0, 3.0),
        velocity: Vec3::new(4.0, 5.0, 6.0),
    };

    let gpu = p.to_gpu();

    assert_eq!(gpu.position, [1.0, 2.0, 3.0]);
    assert_eq!(gpu.velocity, [4.0, 5.0, 6.0]);
    assert_eq!(gpu.particle_type, 0); // Auto-injected default
    assert_eq!(gpu.alive, 1); // Particles start alive
    assert!((gpu.scale - 1.0).abs() < 0.001); // Default scale
}

#[test]
fn test_minimal_particle_no_color() {
    assert!(MinimalParticle::COLOR_FIELD.is_none());
    assert!(MinimalParticle::COLOR_OFFSET.is_none());
}

// ============================================================================
// Particle Derive Tests - With Color
// ============================================================================

#[derive(Particle, Clone)]
struct ColoredParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
}

#[test]
fn test_colored_particle_wgsl_struct() {
    let wgsl = ColoredParticle::WGSL_STRUCT;

    assert!(wgsl.contains("position: vec3<f32>"));
    assert!(wgsl.contains("velocity: vec3<f32>"));
    assert!(wgsl.contains("color: vec3<f32>"));
}

#[test]
fn test_colored_particle_color_field() {
    assert_eq!(ColoredParticle::COLOR_FIELD, Some("color"));
    assert!(ColoredParticle::COLOR_OFFSET.is_some());
}

#[test]
fn test_colored_particle_to_gpu() {
    let p = ColoredParticle {
        position: Vec3::new(1.0, 2.0, 3.0),
        velocity: Vec3::new(4.0, 5.0, 6.0),
        color: Vec3::new(0.5, 0.7, 0.9),
    };

    let gpu = p.to_gpu();

    assert_eq!(gpu.position, [1.0, 2.0, 3.0]);
    assert_eq!(gpu.velocity, [4.0, 5.0, 6.0]);
    assert_eq!(gpu.color, [0.5, 0.7, 0.9]);
}

// ============================================================================
// Particle Derive Tests - With Explicit particle_type
// ============================================================================

#[derive(Particle, Clone)]
struct TypedParticle {
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
}

#[test]
fn test_typed_particle_wgsl_struct() {
    let wgsl = TypedParticle::WGSL_STRUCT;

    assert!(wgsl.contains("particle_type: u32"));
    // Should not have duplicate particle_type
    assert_eq!(wgsl.matches("particle_type").count(), 1);
}

#[test]
fn test_typed_particle_to_gpu() {
    let p = TypedParticle {
        position: Vec3::ZERO,
        velocity: Vec3::ZERO,
        particle_type: 42,
    };

    let gpu = p.to_gpu();
    assert_eq!(gpu.particle_type, 42);
}

// ============================================================================
// Particle Derive Tests - Complex with Multiple Field Types
// ============================================================================

#[derive(Particle, Clone)]
struct ComplexParticle {
    position: Vec3,
    velocity: Vec3,
    #[color]
    color: Vec3,
    particle_type: u32,
    energy: f32,
    mass: f32,
    temperature: f32,
}

#[test]
fn test_complex_particle_wgsl_struct() {
    let wgsl = ComplexParticle::WGSL_STRUCT;

    assert!(wgsl.contains("position: vec3<f32>"));
    assert!(wgsl.contains("velocity: vec3<f32>"));
    assert!(wgsl.contains("color: vec3<f32>"));
    assert!(wgsl.contains("particle_type: u32"));
    assert!(wgsl.contains("energy: f32"));
    assert!(wgsl.contains("mass: f32"));
    assert!(wgsl.contains("temperature: f32"));
}

#[test]
fn test_complex_particle_to_gpu() {
    let p = ComplexParticle {
        position: Vec3::new(1.0, 2.0, 3.0),
        velocity: Vec3::new(4.0, 5.0, 6.0),
        color: Vec3::new(0.1, 0.2, 0.3),
        particle_type: 7,
        energy: 100.0,
        mass: 1.5,
        temperature: 300.0,
    };

    let gpu = p.to_gpu();

    assert_eq!(gpu.position, [1.0, 2.0, 3.0]);
    assert_eq!(gpu.velocity, [4.0, 5.0, 6.0]);
    assert_eq!(gpu.color, [0.1, 0.2, 0.3]);
    assert_eq!(gpu.particle_type, 7);
    assert!((gpu.energy - 100.0).abs() < 0.001);
    assert!((gpu.mass - 1.5).abs() < 0.001);
    assert!((gpu.temperature - 300.0).abs() < 0.001);
}

// ============================================================================
// Particle Derive Tests - Vec2 and Vec4 Fields
// ============================================================================

#[derive(Particle, Clone)]
struct VectorParticle {
    position: Vec3,
    velocity: Vec3,
    uv: Vec2,
    extra: Vec4,
}

#[test]
fn test_vector_particle_wgsl_struct() {
    let wgsl = VectorParticle::WGSL_STRUCT;

    assert!(wgsl.contains("vec3<f32>"));
    assert!(wgsl.contains("vec2<f32>"));
    assert!(wgsl.contains("vec4<f32>"));
}

#[test]
fn test_vector_particle_to_gpu() {
    let p = VectorParticle {
        position: Vec3::new(1.0, 2.0, 3.0),
        velocity: Vec3::new(4.0, 5.0, 6.0),
        uv: Vec2::new(0.5, 0.5),
        extra: Vec4::new(1.0, 2.0, 3.0, 4.0),
    };

    let gpu = p.to_gpu();

    assert_eq!(gpu.position, [1.0, 2.0, 3.0]);
    assert_eq!(gpu.velocity, [4.0, 5.0, 6.0]);
    assert_eq!(gpu.uv, [0.5, 0.5]);
    assert_eq!(gpu.extra, [1.0, 2.0, 3.0, 4.0]);
}

// ============================================================================
// Particle Derive Tests - Integer Fields
// ============================================================================

#[derive(Particle, Clone)]
struct IntParticle {
    position: Vec3,
    velocity: Vec3,
    count: u32,
    offset: i32,
}

#[test]
fn test_int_particle_wgsl_struct() {
    let wgsl = IntParticle::WGSL_STRUCT;

    assert!(wgsl.contains("count: u32"));
    assert!(wgsl.contains("offset: i32"));
}

#[test]
fn test_int_particle_to_gpu() {
    let p = IntParticle {
        position: Vec3::ZERO,
        velocity: Vec3::ZERO,
        count: 42,
        offset: -10,
    };

    let gpu = p.to_gpu();

    assert_eq!(gpu.count, 42);
    assert_eq!(gpu.offset, -10);
}

// ============================================================================
// Particle Derive Tests - GPU Struct Alignment
// ============================================================================

#[test]
fn test_gpu_struct_size_multiple_of_16() {
    // GPU structs must have size that's a multiple of 16 for array usage
    assert_eq!(std::mem::size_of::<MinimalParticleGpu>() % 16, 0);
    assert_eq!(std::mem::size_of::<ColoredParticleGpu>() % 16, 0);
    assert_eq!(std::mem::size_of::<ComplexParticleGpu>() % 16, 0);
    assert_eq!(std::mem::size_of::<VectorParticleGpu>() % 16, 0);
    assert_eq!(std::mem::size_of::<IntParticleGpu>() % 16, 0);
}

#[test]
fn test_gpu_struct_is_pod() {
    // All GPU structs should be Pod (plain old data)
    fn assert_pod<T: bytemuck::Pod>() {}

    assert_pod::<MinimalParticleGpu>();
    assert_pod::<ColoredParticleGpu>();
    assert_pod::<ComplexParticleGpu>();
    assert_pod::<VectorParticleGpu>();
    assert_pod::<IntParticleGpu>();
}

#[test]
fn test_gpu_struct_is_zeroable() {
    // All GPU structs should be Zeroable
    fn assert_zeroable<T: bytemuck::Zeroable>() {}

    assert_zeroable::<MinimalParticleGpu>();
    assert_zeroable::<ColoredParticleGpu>();
    assert_zeroable::<ComplexParticleGpu>();
    assert_zeroable::<VectorParticleGpu>();
    assert_zeroable::<IntParticleGpu>();
}

// ============================================================================
// Particle Derive Tests - Lifecycle Fields
// ============================================================================

#[test]
fn test_alive_offset_valid() {
    // ALIVE_OFFSET should be within struct bounds
    assert!(MinimalParticle::ALIVE_OFFSET < std::mem::size_of::<MinimalParticleGpu>() as u32);
    assert!(ColoredParticle::ALIVE_OFFSET < std::mem::size_of::<ColoredParticleGpu>() as u32);
}

#[test]
fn test_scale_offset_valid() {
    // SCALE_OFFSET should be within struct bounds
    assert!(MinimalParticle::SCALE_OFFSET < std::mem::size_of::<MinimalParticleGpu>() as u32);
    assert!(ColoredParticle::SCALE_OFFSET < std::mem::size_of::<ColoredParticleGpu>() as u32);
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn test_scale_offset_after_alive_offset() {
    // Scale should come after alive in memory layout
    // These are compile-time-known, but we keep them as runtime asserts
    // to catch derive macro bugs
    assert!(MinimalParticle::SCALE_OFFSET > MinimalParticle::ALIVE_OFFSET);
    assert!(ColoredParticle::SCALE_OFFSET > ColoredParticle::ALIVE_OFFSET);
}

// ============================================================================
// WGSL Validation Tests
// ============================================================================

/// Validates that generated WGSL struct is syntactically valid
fn validate_wgsl_struct(wgsl_struct: &str) -> Result<(), String> {
    // Wrap in a minimal shader for naga validation
    let shader = format!(
        r#"
{wgsl_struct}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    var p = particles[idx];
    p.position = p.position + p.velocity;
    particles[idx] = p;
}}
"#
    );

    let module = naga::front::wgsl::parse_str(&shader)
        .map_err(|e| format!("WGSL parse error: {:?}", e))?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .map_err(|e| format!("WGSL validation error: {:?}", e))?;

    Ok(())
}

#[test]
fn test_minimal_particle_wgsl_validates() {
    validate_wgsl_struct(MinimalParticle::WGSL_STRUCT)
        .expect("MinimalParticle WGSL should be valid");
}

#[test]
fn test_colored_particle_wgsl_validates() {
    validate_wgsl_struct(ColoredParticle::WGSL_STRUCT)
        .expect("ColoredParticle WGSL should be valid");
}

#[test]
fn test_complex_particle_wgsl_validates() {
    validate_wgsl_struct(ComplexParticle::WGSL_STRUCT)
        .expect("ComplexParticle WGSL should be valid");
}

#[test]
fn test_vector_particle_wgsl_validates() {
    validate_wgsl_struct(VectorParticle::WGSL_STRUCT)
        .expect("VectorParticle WGSL should be valid");
}

#[test]
fn test_int_particle_wgsl_validates() {
    validate_wgsl_struct(IntParticle::WGSL_STRUCT)
        .expect("IntParticle WGSL should be valid");
}
