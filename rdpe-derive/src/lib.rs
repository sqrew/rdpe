//! Derive macros for the RDPE particle simulation engine.
//!
//! This crate provides two derive macros:
//!
//! - [`Particle`] - Generates GPU-compatible structs and WGSL code
//! - [`ParticleType`] - Creates type-safe enums for particle categories
//!
//! # Usage
//!
//! These macros are re-exported from the main `rdpe` crate. You don't need
//! to add this crate directly:
//!
//! ```ignore
//! use rdpe::prelude::*;
//!
//! #[derive(Particle, Clone)]
//! struct Ball {
//!     position: Vec3,
//!     velocity: Vec3,
//! }
//!
//! #[derive(ParticleType, Clone, Copy, PartialEq)]
//! enum Species {
//!     Prey,
//!     Predator,
//! }
//! ```
//!
//! # The Particle Macro
//!
//! `#[derive(Particle)]` transforms your Rust struct into a GPU-compatible
//! format. It generates:
//!
//! - A companion `{Name}Gpu` struct with proper alignment and padding
//! - A `WGSL_STRUCT` constant containing the WGSL struct definition
//! - `to_gpu()` method for converting Rust → GPU format
//!
//! ## Required Fields
//!
//! Every particle must have:
//! - `position: Vec3` - Particle position in 3D space
//! - `velocity: Vec3` - Particle velocity
//!
//! ## Optional Fields
//!
//! - `particle_type: u32` - For typed interactions (auto-added if missing)
//! - `#[color] color: Vec3` - Custom particle color
//! - Any `f32`, `u32`, `i32`, `Vec2`, `Vec3`, `Vec4` fields
//!
//! ## GPU Memory Layout
//!
//! The macro handles WGSL's strict alignment requirements:
//! - `Vec3` requires 16-byte alignment (even though it's only 12 bytes)
//! - Struct total size must be a multiple of 16 bytes
//! - Padding fields are automatically inserted
//!
//! # The ParticleType Macro
//!
//! `#[derive(ParticleType)]` enables type-safe particle categories for use
//! with typed rules like `Chase`, `Evade`, and `Convert`.
//!
//! It generates:
//! - `From<EnumName> for u32` - Convert enum to GPU-compatible integer
//! - `From<u32> for EnumName` - Convert back (defaults to first variant)
//! - `EnumName::count() -> u32` - Number of variants

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

/// Derive macro for particle type enums.
///
/// Creates type-safe particle categories that convert to/from `u32` for GPU usage.
/// Variants are assigned sequential IDs starting from 0.
///
/// # Generated Items
///
/// For an enum `Species`:
///
/// - `impl From<Species> for u32` - Convert variant to integer
/// - `impl From<u32> for Species` - Convert integer to variant (invalid values default to first variant)
/// - `Species::count() -> u32` - Returns number of variants
///
/// # Requirements
///
/// - Must be an enum (not a struct)
/// - All variants must be unit variants (no fields)
/// - Enum should also derive `Clone`, `Copy`, `PartialEq` for typical usage
///
/// # Example
///
/// ```ignore
/// #[derive(ParticleType, Clone, Copy, PartialEq)]
/// enum Species {
///     Prey,      // = 0
///     Predator,  // = 1
///     Plant,     // = 2
/// }
///
/// // Convert to u32 for rules
/// let prey_id: u32 = Species::Prey.into();  // 0
///
/// // Use with typed rules
/// Rule::Chase {
///     self_type: Species::Predator.into(),
///     target_type: Species::Prey.into(),
///     radius: 0.3,
///     strength: 2.0,
/// }
///
/// // Use in spawner
/// Creature {
///     position: pos,
///     velocity: Vec3::ZERO,
///     particle_type: Species::Prey.into(),
/// }
///
/// // Get variant count
/// let num_species = Species::count();  // 3
/// ```
///
/// # Panics
///
/// The macro panics at compile time if:
/// - Applied to a struct instead of an enum
/// - Any variant has fields (tuple or struct variants)
/// - Enum has zero variants
#[proc_macro_derive(ParticleType)]
pub fn derive_particle_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => panic!("ParticleType derive only supports enums"),
    };

    // Check that all variants are unit variants (no fields)
    for variant in variants.iter() {
        if !matches!(variant.fields, Fields::Unit) {
            panic!(
                "ParticleType enum variants must be unit variants (no fields). \
                 Found fields on variant '{}'",
                variant.ident
            );
        }
    }

    // Generate match arms for Into<u32>
    let into_arms: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
            let variant_name = &variant.ident;
            let idx = i as u32;
            quote! { #name::#variant_name => #idx }
        })
        .collect();

    // Generate match arms for From<u32>
    let from_arms: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
            let variant_name = &variant.ident;
            let idx = i as u32;
            quote! { #idx => #name::#variant_name }
        })
        .collect();

    let first_variant = &variants.first().expect("Enum must have at least one variant").ident;
    let variant_count = variants.len() as u32;

    let expanded = quote! {
        impl From<#name> for u32 {
            fn from(value: #name) -> u32 {
                match value {
                    #(#into_arms),*
                }
            }
        }

        impl From<u32> for #name {
            fn from(value: u32) -> #name {
                match value {
                    #(#from_arms,)*
                    _ => #name::#first_variant, // Default to first variant for invalid values
                }
            }
        }

        impl #name {
            /// Returns the number of variants in this particle type enum.
            pub const fn count() -> u32 {
                #variant_count
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for particle structs.
///
/// Transforms a Rust struct into a GPU-compatible particle type. Generates:
///
/// - A companion `{Name}Gpu` struct with `#[repr(C)]` and proper WGSL alignment
/// - Implementation of [`ParticleTrait`](rdpe::ParticleTrait)
/// - WGSL struct definition as a const string
///
/// # Required Fields
///
/// Every particle must have these fields:
///
/// | Field | Type | Purpose |
/// |-------|------|---------|
/// | `position` | `Vec3` | Particle location in 3D space |
/// | `velocity` | `Vec3` | Particle movement direction and speed |
///
/// # Optional Fields
///
/// | Field | Type | Purpose |
/// |-------|------|---------|
/// | `particle_type` | `u32` | Category for typed interactions (auto-added if missing) |
/// | `#[color] name` | `Vec3` | Custom particle color (RGB, 0.0-1.0) |
/// | *(any name)* | `f32`, `u32`, `i32`, `Vec2`, `Vec3`, `Vec4` | Custom data |
///
/// # Supported Types
///
/// | Rust Type | WGSL Type | Size | Alignment |
/// |-----------|-----------|------|-----------|
/// | `Vec3` | `vec3<f32>` | 12 bytes | 16 bytes |
/// | `Vec2` | `vec2<f32>` | 8 bytes | 8 bytes |
/// | `Vec4` | `vec4<f32>` | 16 bytes | 16 bytes |
/// | `f32` | `f32` | 4 bytes | 4 bytes |
/// | `u32` | `u32` | 4 bytes | 4 bytes |
/// | `i32` | `i32` | 4 bytes | 4 bytes |
///
/// # GPU Memory Layout
///
/// WGSL has strict alignment requirements that differ from Rust. This macro
/// automatically inserts padding fields to ensure correct GPU memory layout:
///
/// - `Vec3` requires 16-byte alignment (despite being 12 bytes)
/// - Arrays of structs require 16-byte stride
/// - Padding fields are named `_pad0`, `_pad1`, etc.
///
/// # The `#[color]` Attribute
///
/// Mark a `Vec3` field with `#[color]` to use it for particle rendering:
///
/// ```ignore
/// #[derive(Particle, Clone)]
/// struct Firework {
///     position: Vec3,
///     velocity: Vec3,
///     #[color]
///     color: Vec3,  // RGB values 0.0-1.0
/// }
/// ```
///
/// Without `#[color]`, particles are colored based on their position.
///
/// # Example
///
/// ```ignore
/// use rdpe::prelude::*;
///
/// // Minimal particle
/// #[derive(Particle, Clone)]
/// struct Basic {
///     position: Vec3,
///     velocity: Vec3,
/// }
///
/// // Full-featured particle
/// #[derive(Particle, Clone)]
/// struct Advanced {
///     position: Vec3,
///     velocity: Vec3,
///     #[color]
///     color: Vec3,
///     particle_type: u32,
///     energy: f32,
///     age: f32,
/// }
/// ```
///
/// # Generated Code
///
/// For a particle `Ball`, the macro generates:
///
/// ```ignore
/// // GPU-compatible struct
/// #[repr(C)]
/// #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// pub struct BallGpu {
///     pub position: [f32; 3],
///     pub _pad0: f32,  // Alignment padding
///     pub velocity: [f32; 3],
///     pub _pad1: f32,
///     pub particle_type: u32,
///     pub _pad2: [f32; 3],  // Struct size padding
/// }
///
/// impl ParticleTrait for Ball {
///     type Gpu = BallGpu;
///     const WGSL_STRUCT: &'static str = "...";
///     // ...
/// }
/// ```
///
/// # Panics
///
/// The macro panics at compile time if:
/// - Applied to an enum instead of a struct
/// - Struct uses tuple fields instead of named fields
/// - Any field has an unsupported type
#[proc_macro_derive(Particle, attributes(color))]
pub fn derive_particle(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let gpu_name = Ident::new(&format!("{}Gpu", name), Span::call_site());

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Particle derive only supports structs with named fields"),
        },
        _ => panic!("Particle derive only supports structs"),
    };

    let mut wgsl_fields = Vec::new();
    let mut gpu_struct_fields = Vec::new();
    let mut to_gpu_conversions = Vec::new();
    let mut field_offset = 0u32;
    let mut padding_count = 0u32;
    let mut color_field: Option<String> = None;
    let mut color_offset: Option<u32> = None;
    let mut has_particle_type = false;

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        if field_name == "particle_type" {
            has_particle_type = true;
        }
    }

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;
        let type_info = rust_type_info(field_type);

        // Check for #[color] attribute - need to track offset before padding
        let mut is_color_field = false;
        for attr in &field.attrs {
            if attr.path().is_ident("color") {
                color_field = Some(field_name_str.clone());
                is_color_field = true;
            }
        }

        // Add padding before field if needed for alignment
        let padding_needed = (type_info.align - (field_offset % type_info.align)) % type_info.align;
        if padding_needed > 0 {
            let pad_name = Ident::new(&format!("_pad{}", padding_count), Span::call_site());
            let pad_name_str = format!("_pad{}", padding_count);
            padding_count += 1;

            if padding_needed == 4 {
                wgsl_fields.push(format!("    {}: f32,", pad_name_str));
                gpu_struct_fields.push(quote! { #pad_name: f32 });
                to_gpu_conversions.push(quote! { #pad_name: 0.0 });
            } else {
                let count = (padding_needed / 4) as usize;
                wgsl_fields.push(format!("    {}: array<f32, {}>,", pad_name_str, count));
                gpu_struct_fields.push(quote! { #pad_name: [f32; #count] });
                to_gpu_conversions.push(quote! { #pad_name: [0.0; #count] });
            }
            field_offset += padding_needed;
        }

        // Record color field offset (after padding, before adding size)
        if is_color_field {
            color_offset = Some(field_offset);
        }

        // Add the actual field
        wgsl_fields.push(format!("    {}: {},", field_name_str, type_info.wgsl_type));

        let gpu_field_type = type_info.gpu_type;
        gpu_struct_fields.push(quote! { #field_name: #gpu_field_type });

        let conversion = generate_conversion(field_name, field_type);
        to_gpu_conversions.push(quote! { #field_name: #conversion });

        field_offset += type_info.size;
    }

    // Add particle_type field if user didn't provide one
    if !has_particle_type {
        // u32 has 4-byte alignment
        let padding_needed = (4 - (field_offset % 4)) % 4;
        if padding_needed > 0 {
            let pad_name = Ident::new(&format!("_pad{}", padding_count), Span::call_site());
            let pad_name_str = format!("_pad{}", padding_count);
            padding_count += 1;

            if padding_needed == 4 {
                wgsl_fields.push(format!("    {}: f32,", pad_name_str));
                gpu_struct_fields.push(quote! { #pad_name: f32 });
                to_gpu_conversions.push(quote! { #pad_name: 0.0 });
            } else {
                let count = (padding_needed / 4) as usize;
                wgsl_fields.push(format!("    {}: array<f32, {}>,", pad_name_str, count));
                gpu_struct_fields.push(quote! { #pad_name: [f32; #count] });
                to_gpu_conversions.push(quote! { #pad_name: [0.0; #count] });
            }
            field_offset += padding_needed;
        }

        wgsl_fields.push("    particle_type: u32,".to_string());
        gpu_struct_fields.push(quote! { particle_type: u32 });
        to_gpu_conversions.push(quote! { particle_type: 0 });
        field_offset += 4;
    }

    // Always inject lifecycle fields: age (f32), alive (u32), scale (f32)
    // These are always present for particle lifecycle management
    // age: time since spawn, alive: 0 = dead, 1 = alive, scale: particle size
    let alive_offset: u32;
    let scale_offset: u32;
    {
        // age: f32 (4-byte aligned)
        let padding_needed = (4 - (field_offset % 4)) % 4;
        if padding_needed > 0 {
            let pad_name = Ident::new(&format!("_pad{}", padding_count), Span::call_site());
            let pad_name_str = format!("_pad{}", padding_count);
            padding_count += 1;
            let count = (padding_needed / 4) as usize;
            if count == 1 {
                wgsl_fields.push(format!("    {}: f32,", pad_name_str));
                gpu_struct_fields.push(quote! { #pad_name: f32 });
                to_gpu_conversions.push(quote! { #pad_name: 0.0 });
            } else {
                wgsl_fields.push(format!("    {}: array<f32, {}>,", pad_name_str, count));
                gpu_struct_fields.push(quote! { #pad_name: [f32; #count] });
                to_gpu_conversions.push(quote! { #pad_name: [0.0; #count] });
            }
            field_offset += padding_needed;
        }

        wgsl_fields.push("    age: f32,".to_string());
        gpu_struct_fields.push(quote! { age: f32 });
        to_gpu_conversions.push(quote! { age: 0.0 });
        field_offset += 4;

        // alive: u32 (4-byte aligned, already aligned after f32)
        // Record offset before adding the field
        alive_offset = field_offset;
        wgsl_fields.push("    alive: u32,".to_string());
        gpu_struct_fields.push(quote! { alive: u32 });
        to_gpu_conversions.push(quote! { alive: 1u32 }); // Particles start alive
        field_offset += 4;

        // scale: f32 (4-byte aligned, already aligned after u32)
        // Record offset before adding the field
        scale_offset = field_offset;
        wgsl_fields.push("    scale: f32,".to_string());
        gpu_struct_fields.push(quote! { scale: f32 });
        to_gpu_conversions.push(quote! { scale: 1.0 }); // Default scale of 1.0
        field_offset += 4;
    }

    // Ensure struct size is multiple of 16 (vec4 alignment for GPU arrays)
    let final_padding = (16 - (field_offset % 16)) % 16;
    if final_padding > 0 {
        let pad_name = Ident::new(&format!("_pad{}", padding_count), Span::call_site());
        let pad_name_str = format!("_pad{}", padding_count);

        if final_padding == 4 {
            wgsl_fields.push(format!("    {}: f32,", pad_name_str));
            gpu_struct_fields.push(quote! { #pad_name: f32 });
            to_gpu_conversions.push(quote! { #pad_name: 0.0 });
        } else {
            let count = (final_padding / 4) as usize;
            wgsl_fields.push(format!("    {}: array<f32, {}>,", pad_name_str, count));
            gpu_struct_fields.push(quote! { #pad_name: [f32; #count] });
            to_gpu_conversions.push(quote! { #pad_name: [0.0; #count] });
        }
    }

    let wgsl_struct = format!("struct Particle {{\n{}\n}}", wgsl_fields.join("\n"));

    let color_field_expr = match color_field {
        Some(ref name) => quote! { Some(#name) },
        None => quote! { None },
    };

    let color_offset_expr = match color_offset {
        Some(offset) => quote! { Some(#offset) },
        None => quote! { None },
    };

    let expanded = quote! {
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        pub struct #gpu_name {
            #(pub #gpu_struct_fields),*
        }

        impl rdpe::ParticleTrait for #name {
            type Gpu = #gpu_name;

            const WGSL_STRUCT: &'static str = #wgsl_struct;
            const COLOR_FIELD: Option<&'static str> = #color_field_expr;
            const COLOR_OFFSET: Option<u32> = #color_offset_expr;
            const ALIVE_OFFSET: u32 = #alive_offset;
            const SCALE_OFFSET: u32 = #scale_offset;

            fn to_gpu(&self) -> Self::Gpu {
                #gpu_name {
                    #(#to_gpu_conversions),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Type metadata for GPU memory layout calculations.
struct TypeInfo {
    /// WGSL type name (e.g., "vec3<f32>")
    wgsl_type: &'static str,
    /// Rust type for the GPU struct (e.g., `[f32; 3]`)
    gpu_type: proc_macro2::TokenStream,
    /// Size in bytes
    size: u32,
    /// Required alignment in bytes
    align: u32,
}

/// Get type information for a Rust type.
///
/// Maps Rust types to their WGSL equivalents and alignment requirements.
fn rust_type_info(ty: &Type) -> TypeInfo {
    let type_str = quote!(#ty).to_string().replace(" ", "");

    match type_str.as_str() {
        "Vec3" | "glam::Vec3" => TypeInfo {
            wgsl_type: "vec3<f32>",
            gpu_type: quote! { [f32; 3] },
            size: 12,
            align: 16, // vec3 has 16-byte alignment in WGSL!
        },
        "Vec2" | "glam::Vec2" => TypeInfo {
            wgsl_type: "vec2<f32>",
            gpu_type: quote! { [f32; 2] },
            size: 8,
            align: 8,
        },
        "Vec4" | "glam::Vec4" => TypeInfo {
            wgsl_type: "vec4<f32>",
            gpu_type: quote! { [f32; 4] },
            size: 16,
            align: 16,
        },
        "f32" => TypeInfo {
            wgsl_type: "f32",
            gpu_type: quote! { f32 },
            size: 4,
            align: 4,
        },
        "u32" => TypeInfo {
            wgsl_type: "u32",
            gpu_type: quote! { u32 },
            size: 4,
            align: 4,
        },
        "i32" => TypeInfo {
            wgsl_type: "i32",
            gpu_type: quote! { i32 },
            size: 4,
            align: 4,
        },
        _ => panic!("Unsupported type in Particle struct: {}", type_str),
    }
}

/// Generate code to convert a field from Rust to GPU format.
///
/// Vector types need `.to_array()`, scalars are passed through.
fn generate_conversion(field_name: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string().replace(" ", "");

    match type_str.as_str() {
        "Vec3" | "glam::Vec3" | "Vec2" | "glam::Vec2" | "Vec4" | "glam::Vec4" => {
            quote! { self.#field_name.to_array() }
        }
        _ => {
            quote! { self.#field_name }
        }
    }
}
