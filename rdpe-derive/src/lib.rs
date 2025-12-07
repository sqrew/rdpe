//! Derive macros for the RDPE particle simulation engine.
//!
//! This crate provides three derive macros:
//!
//! - [`Particle`] - Generates GPU-compatible structs and WGSL code
//! - [`ParticleType`] - Creates type-safe enums for particle categories
//! - [`MultiParticle`] - Combines multiple Particle types into one simulation
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
    let mut from_gpu_conversions = Vec::new();
    let mut inspect_field_entries = Vec::new();
    let mut editable_field_widgets = Vec::new();
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

        let reverse_conversion = generate_reverse_conversion(field_name, field_type);
        from_gpu_conversions.push(quote! { #field_name: #reverse_conversion });

        // Generate inspect field entry with nice formatting
        let inspect_format = generate_inspect_format(field_name, field_type);
        inspect_field_entries.push(quote! { (#field_name_str, #inspect_format) });

        // Generate editable widget for this field
        let editable_widget = generate_editable_widget(field_name, &field_name_str, field_type);
        editable_field_widgets.push(editable_widget);

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

            fn from_gpu(gpu: &Self::Gpu) -> Self {
                Self {
                    #(#from_gpu_conversions),*
                }
            }

            fn inspect_fields(&self) -> Vec<(&'static str, String)> {
                vec![
                    #(#inspect_field_entries),*
                ]
            }

            #[cfg(feature = "egui")]
            fn render_editable_fields(&mut self, ui: &mut egui::Ui) -> bool {
                let mut modified = false;
                egui::Grid::new("editable_fields")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        #(#editable_field_widgets)*
                    });
                modified
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

/// Generate code to convert a field from GPU format back to Rust.
///
/// Vector types need `from_array()`, scalars are passed through.
fn generate_reverse_conversion(field_name: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string().replace(" ", "");

    match type_str.as_str() {
        "Vec3" | "glam::Vec3" => {
            quote! { glam::Vec3::from_array(gpu.#field_name) }
        }
        "Vec2" | "glam::Vec2" => {
            quote! { glam::Vec2::from_array(gpu.#field_name) }
        }
        "Vec4" | "glam::Vec4" => {
            quote! { glam::Vec4::from_array(gpu.#field_name) }
        }
        _ => {
            quote! { gpu.#field_name }
        }
    }
}

/// Generate code to format a field for inspection display.
///
/// Produces human-readable formatted strings for the inspector panel.
fn generate_inspect_format(field_name: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string().replace(" ", "");

    match type_str.as_str() {
        "Vec3" | "glam::Vec3" => {
            quote! {
                format!("({:.3}, {:.3}, {:.3})", self.#field_name.x, self.#field_name.y, self.#field_name.z)
            }
        }
        "Vec2" | "glam::Vec2" => {
            quote! {
                format!("({:.3}, {:.3})", self.#field_name.x, self.#field_name.y)
            }
        }
        "Vec4" | "glam::Vec4" => {
            quote! {
                format!("({:.3}, {:.3}, {:.3}, {:.3})", self.#field_name.x, self.#field_name.y, self.#field_name.z, self.#field_name.w)
            }
        }
        "f32" => {
            quote! { format!("{:.3}", self.#field_name) }
        }
        "u32" | "i32" => {
            quote! { format!("{}", self.#field_name) }
        }
        _ => {
            quote! { format!("{:?}", self.#field_name) }
        }
    }
}

/// Generate editable UI widget code for a field.
///
/// Produces egui widget code for editing particle fields in the inspector.
fn generate_editable_widget(field_name: &Ident, field_name_str: &str, ty: &Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string().replace(" ", "");

    match type_str.as_str() {
        "Vec3" | "glam::Vec3" => {
            quote! {
                ui.label(#field_name_str);
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut self.#field_name.x).speed(0.01).prefix("x: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.y).speed(0.01).prefix("y: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.z).speed(0.01).prefix("z: ")).changed() {
                        modified = true;
                    }
                });
                ui.end_row();
            }
        }
        "Vec2" | "glam::Vec2" => {
            quote! {
                ui.label(#field_name_str);
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut self.#field_name.x).speed(0.01).prefix("x: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.y).speed(0.01).prefix("y: ")).changed() {
                        modified = true;
                    }
                });
                ui.end_row();
            }
        }
        "Vec4" | "glam::Vec4" => {
            quote! {
                ui.label(#field_name_str);
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut self.#field_name.x).speed(0.01).prefix("x: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.y).speed(0.01).prefix("y: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.z).speed(0.01).prefix("z: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.w).speed(0.01).prefix("w: ")).changed() {
                        modified = true;
                    }
                });
                ui.end_row();
            }
        }
        "f32" => {
            quote! {
                ui.label(#field_name_str);
                if ui.add(egui::DragValue::new(&mut self.#field_name).speed(0.01)).changed() {
                    modified = true;
                }
                ui.end_row();
            }
        }
        "u32" => {
            quote! {
                ui.label(#field_name_str);
                let mut val = self.#field_name as i64;
                if ui.add(egui::DragValue::new(&mut val).speed(1.0)).changed() {
                    self.#field_name = val.max(0) as u32;
                    modified = true;
                }
                ui.end_row();
            }
        }
        "i32" => {
            quote! {
                ui.label(#field_name_str);
                if ui.add(egui::DragValue::new(&mut self.#field_name).speed(1.0)).changed() {
                    modified = true;
                }
                ui.end_row();
            }
        }
        _ => {
            // For unknown types, just show as read-only
            quote! {
                ui.label(#field_name_str);
                ui.label(format!("{:?}", self.#field_name));
                ui.end_row();
            }
        }
    }
}

/// Generate editable UI widget code for a field (from string type).
///
/// Used by MultiParticle derive where we have type as string.
fn generate_editable_widget_from_string(field_name: &Ident, field_name_str: &str, type_str: &str) -> proc_macro2::TokenStream {
    match type_str {
        "Vec3" => {
            quote! {
                ui.label(#field_name_str);
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut self.#field_name.x).speed(0.01).prefix("x: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.y).speed(0.01).prefix("y: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.z).speed(0.01).prefix("z: ")).changed() {
                        modified = true;
                    }
                });
                ui.end_row();
            }
        }
        "Vec2" => {
            quote! {
                ui.label(#field_name_str);
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut self.#field_name.x).speed(0.01).prefix("x: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.y).speed(0.01).prefix("y: ")).changed() {
                        modified = true;
                    }
                });
                ui.end_row();
            }
        }
        "Vec4" => {
            quote! {
                ui.label(#field_name_str);
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut self.#field_name.x).speed(0.01).prefix("x: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.y).speed(0.01).prefix("y: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.z).speed(0.01).prefix("z: ")).changed() {
                        modified = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.#field_name.w).speed(0.01).prefix("w: ")).changed() {
                        modified = true;
                    }
                });
                ui.end_row();
            }
        }
        "f32" => {
            quote! {
                ui.label(#field_name_str);
                if ui.add(egui::DragValue::new(&mut self.#field_name).speed(0.01)).changed() {
                    modified = true;
                }
                ui.end_row();
            }
        }
        "u32" => {
            quote! {
                ui.label(#field_name_str);
                let mut val = self.#field_name as i64;
                if ui.add(egui::DragValue::new(&mut val).speed(1.0)).changed() {
                    self.#field_name = val.max(0) as u32;
                    modified = true;
                }
                ui.end_row();
            }
        }
        "i32" => {
            quote! {
                ui.label(#field_name_str);
                if ui.add(egui::DragValue::new(&mut self.#field_name).speed(1.0)).changed() {
                    modified = true;
                }
                ui.end_row();
            }
        }
        _ => {
            quote! {
                ui.label(#field_name_str);
                ui.label(format!("{:?}", self.#field_name));
                ui.end_row();
            }
        }
    }
}

/// Derive macro for multi-particle enums with inline struct definitions.
///
/// Generates standalone particle structs AND a unified enum, enabling both
/// single-type and heterogeneous particle simulations from one definition.
///
/// # Overview
///
/// `MultiParticle` lets you define multiple particle types inline as struct-like
/// enum variants. The macro generates:
///
/// 1. **Standalone structs** - Each variant becomes its own struct implementing `ParticleTrait`
/// 2. **Unified enum** - The enum implements `ParticleTrait` with all fields combined
/// 3. **Rust type constants** - `EnumName::VARIANT` constants for use in typed rules
/// 4. **WGSL helpers** - Type constants and helper functions for shaders
///
/// # Example
///
/// ```ignore
/// use rdpe::prelude::*;
///
/// #[derive(MultiParticle, Clone)]
/// enum Creature {
///     Boid {
///         position: Vec3,
///         velocity: Vec3,
///         flock_id: u32,
///     },
///     Predator {
///         position: Vec3,
///         velocity: Vec3,
///         hunger: f32,
///         target_id: u32,
///     },
/// }
///
/// // All three work:
/// Simulation::<Creature>::new()  // Mixed simulation
/// Simulation::<Boid>::new()      // Boid-only simulation (uses generated struct)
/// Simulation::<Predator>::new()  // Predator-only simulation (uses generated struct)
///
/// // Creating particles with clean struct-like syntax:
/// Creature::Boid { position: Vec3::ZERO, velocity: Vec3::ZERO, flock_id: 0 }
/// Creature::Predator { position: Vec3::ZERO, velocity: Vec3::ZERO, hunger: 1.0, target_id: 0 }
///
/// // Using type constants in rules:
/// Rule::Chase {
///     self_type: Creature::PREDATOR,
///     target_type: Creature::BOID,
///     radius: 0.5,
///     strength: 3.0,
/// }
/// ```
///
/// # Requirements
///
/// - The enum must also derive `Clone` (for `ParticleTrait` bounds)
/// - Each variant must use struct-like syntax with named fields
/// - Each variant must have `position: Vec3` and `velocity: Vec3` fields
/// - Use `#[color]` attribute on a `Vec3` field for custom particle color
///
/// # Generated Code
///
/// For an enum `Creature { Boid { ... }, Predator { ... } }`, the macro generates:
///
/// ```ignore
/// // Standalone structs (separate types for single-particle simulations)
/// struct Boid { position: Vec3, velocity: Vec3, flock_id: u32 }
/// impl ParticleTrait for Boid { ... }
///
/// struct Predator { position: Vec3, velocity: Vec3, hunger: f32, target_id: u32 }
/// impl ParticleTrait for Predator { ... }
///
/// // ParticleTrait on the original enum (for mixed simulations)
/// impl ParticleTrait for Creature { ... }  // Unified GPU struct with all fields
/// ```
///
/// # WGSL Usage
///
/// In custom rules, use the generated constants and helpers:
///
/// ```wgsl
/// // Type constants
/// const BOID: u32 = 0u;
/// const PREDATOR: u32 = 1u;
///
/// // Helper functions
/// fn is_boid(p: Particle) -> bool { return p.particle_type == 0u; }
/// fn is_predator(p: Particle) -> bool { return p.particle_type == 1u; }
///
/// // Usage in custom rules
/// if is_predator(p) {
///     p.hunger -= uniforms.delta_time * 0.1;
/// }
/// ```
#[proc_macro_derive(MultiParticle, attributes(color))]
pub fn derive_multi_particle(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = &input.ident;
    let enum_gpu_name = Ident::new(&format!("{}Gpu", enum_name), Span::call_site());
    let visibility = &input.vis;

    // Parse as enum with struct-like variants
    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => panic!("MultiParticle derive only supports enums"),
    };

    // Collect variant info: (name, fields as Vec<(name, type_string, is_color)>)
    let mut variant_info: Vec<(Ident, Vec<(Ident, String, bool)>)> = Vec::new();

    for variant in variants.iter() {
        let variant_name = variant.ident.clone();

        let fields = match &variant.fields {
            Fields::Named(named) => {
                named.named.iter().map(|f| {
                    let field_name = f.ident.clone().unwrap();
                    let ty = &f.ty;
                    let type_str = quote!(#ty).to_string().replace(" ", "");
                    let is_color = f.attrs.iter().any(|a| a.path().is_ident("color"));
                    (field_name, type_str, is_color)
                }).collect::<Vec<_>>()
            }
            _ => panic!(
                "MultiParticle variant '{}' must have named fields (struct-like syntax)",
                variant_name
            ),
        };

        // Validate required fields
        let has_position = fields.iter().any(|(n, t, _)| n == "position" && (t == "Vec3" || t == "glam::Vec3"));
        let has_velocity = fields.iter().any(|(n, t, _)| n == "velocity" && (t == "Vec3" || t == "glam::Vec3"));

        if !has_position {
            panic!("MultiParticle variant '{}' must have 'position: Vec3' field", variant_name);
        }
        if !has_velocity {
            panic!("MultiParticle variant '{}' must have 'velocity: Vec3' field", variant_name);
        }

        variant_info.push((variant_name, fields));
    }

    // ========================================
    // Generate standalone structs for each variant
    // ========================================
    let mut standalone_structs = Vec::new();

    for (variant_name, fields) in &variant_info {
        let struct_gpu_name = Ident::new(&format!("{}Gpu", variant_name), Span::call_site());

        // Build struct fields
        let struct_fields: Vec<_> = fields.iter().map(|(name, type_str, _)| {
            let ty = rust_type_from_string(type_str);
            quote! { pub #name: #ty }
        }).collect();

        // Build GPU struct using the generate_particle_gpu_struct helper
        let (gpu_fields, wgsl_struct, color_field, color_offset, alive_offset, scale_offset) =
            generate_particle_gpu_struct(fields, false); // false = include particle_type

        let gpu_field_tokens: Vec<_> = gpu_fields.iter().map(|(name, ty)| {
            let name_ident = Ident::new(name, Span::call_site());
            quote! { pub #name_ident: #ty }
        }).collect();

        // Build to_gpu conversions
        let to_gpu_conversions: Vec<_> = gpu_fields.iter().map(|(name, _ty)| {
            let name_ident = Ident::new(name, Span::call_site());
            if name.starts_with("_pad") {
                // Use Default::default() for all padding (works for both f32 and [f32; N])
                quote! { #name_ident: Default::default() }
            } else if name == "particle_type" {
                quote! { #name_ident: 0 }
            } else if name == "age" {
                quote! { #name_ident: 0.0 }
            } else if name == "alive" {
                quote! { #name_ident: 1u32 }
            } else if name == "scale" {
                quote! { #name_ident: 1.0 }
            } else {
                // User field - check if it needs to_array
                let field_info = fields.iter().find(|(n, _, _)| n.to_string() == *name);
                if let Some((_, type_str, _)) = field_info {
                    if type_str == "Vec3" || type_str == "Vec2" || type_str == "Vec4" ||
                       type_str == "glam::Vec3" || type_str == "glam::Vec2" || type_str == "glam::Vec4" {
                        quote! { #name_ident: self.#name_ident.to_array() }
                    } else {
                        quote! { #name_ident: self.#name_ident }
                    }
                } else {
                    quote! { #name_ident: Default::default() }
                }
            }
        }).collect();

        // Build from_gpu conversions for each user field
        let from_gpu_conversions: Vec<_> = fields.iter().map(|(name, type_str, _)| {
            let name_ident = name;
            let type_normalized = type_str.replace("glam::", "");
            match type_normalized.as_str() {
                "Vec3" => quote! { #name_ident: rdpe::Vec3::from_array(gpu.#name_ident) },
                "Vec2" => quote! { #name_ident: rdpe::Vec2::from_array(gpu.#name_ident) },
                "Vec4" => quote! { #name_ident: rdpe::Vec4::from_array(gpu.#name_ident) },
                _ => quote! { #name_ident: gpu.#name_ident },
            }
        }).collect();

        // Build inspect field entries
        let inspect_entries: Vec<_> = fields.iter().map(|(name, type_str, _)| {
            let name_str = name.to_string();
            let type_normalized = type_str.replace("glam::", "");
            match type_normalized.as_str() {
                "Vec3" => quote! { (#name_str, format!("({:.3}, {:.3}, {:.3})", self.#name.x, self.#name.y, self.#name.z)) },
                "Vec2" => quote! { (#name_str, format!("({:.3}, {:.3})", self.#name.x, self.#name.y)) },
                "Vec4" => quote! { (#name_str, format!("({:.3}, {:.3}, {:.3}, {:.3})", self.#name.x, self.#name.y, self.#name.z, self.#name.w)) },
                "f32" => quote! { (#name_str, format!("{:.3}", self.#name)) },
                "u32" | "i32" => quote! { (#name_str, format!("{}", self.#name)) },
                _ => quote! { (#name_str, format!("{:?}", self.#name)) },
            }
        }).collect();

        // Build editable widget entries
        let editable_entries: Vec<_> = fields.iter().map(|(name, type_str, _)| {
            let name_str = name.to_string();
            let type_normalized = type_str.replace("glam::", "");
            generate_editable_widget_from_string(name, &name_str, &type_normalized)
        }).collect();

        let color_field_expr = match &color_field {
            Some(name) => quote! { Some(#name) },
            None => quote! { None },
        };

        let color_offset_expr = match color_offset {
            Some(offset) => quote! { Some(#offset) },
            None => quote! { None },
        };

        standalone_structs.push(quote! {
            /// Auto-generated struct from MultiParticle variant
            #[derive(Clone)]
            #visibility struct #variant_name {
                #(#struct_fields),*
            }

            #[repr(C)]
            #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
            #visibility struct #struct_gpu_name {
                #(#gpu_field_tokens),*
            }

            impl rdpe::ParticleTrait for #variant_name {
                type Gpu = #struct_gpu_name;

                const WGSL_STRUCT: &'static str = #wgsl_struct;
                const COLOR_FIELD: Option<&'static str> = #color_field_expr;
                const COLOR_OFFSET: Option<u32> = #color_offset_expr;
                const ALIVE_OFFSET: u32 = #alive_offset;
                const SCALE_OFFSET: u32 = #scale_offset;

                fn to_gpu(&self) -> Self::Gpu {
                    #struct_gpu_name {
                        #(#to_gpu_conversions),*
                    }
                }

                fn from_gpu(gpu: &Self::Gpu) -> Self {
                    Self {
                        #(#from_gpu_conversions),*
                    }
                }

                fn inspect_fields(&self) -> Vec<(&'static str, String)> {
                    vec![
                        #(#inspect_entries),*
                    ]
                }

                #[cfg(feature = "egui")]
                fn render_editable_fields(&mut self, ui: &mut egui::Ui) -> bool {
                    let mut modified = false;
                    egui::Grid::new("editable_fields")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            #(#editable_entries)*
                        });
                    modified
                }
            }
        });
    }

    // ========================================
    // Build unified field list for the enum
    // ========================================
    let mut all_fields: Vec<(String, String, bool)> = vec![
        ("position".to_string(), "Vec3".to_string(), false),
        ("velocity".to_string(), "Vec3".to_string(), false),
    ];

    let mut seen_fields: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    seen_fields.insert("position".to_string(), "Vec3".to_string());
    seen_fields.insert("velocity".to_string(), "Vec3".to_string());

    for (variant_name, fields) in &variant_info {
        for (fname, ftype, is_color) in fields {
            let fname_str = fname.to_string();
            let ftype_normalized = ftype.replace("glam::", "");
            if fname_str == "position" || fname_str == "velocity" {
                continue; // Already added
            }
            if let Some(existing_type) = seen_fields.get(&fname_str) {
                if existing_type != &ftype_normalized {
                    panic!(
                        "Field '{}' has conflicting types: '{}' in one variant, '{}' in '{}'",
                        fname_str, existing_type, ftype_normalized, variant_name
                    );
                }
            } else {
                seen_fields.insert(fname_str.clone(), ftype_normalized.clone());
                all_fields.push((fname_str, ftype_normalized, *is_color));
            }
        }
    }

    // ========================================
    // Generate unified GPU struct for enum
    // ========================================
    let (enum_gpu_fields, enum_wgsl_struct, _, _, enum_alive_offset, enum_scale_offset) =
        generate_unified_gpu_struct(&all_fields);

    let enum_gpu_field_tokens: Vec<_> = enum_gpu_fields.iter().map(|(name, ty)| {
        let name_ident = Ident::new(name, Span::call_site());
        quote! { pub #name_ident: #ty }
    }).collect();

    // Generate EXTRA_WGSL with type constants and helpers
    let mut extra_wgsl_lines = Vec::new();
    extra_wgsl_lines.push("// MultiParticle type constants".to_string());

    for (i, (variant_name, _)) in variant_info.iter().enumerate() {
        let const_name = variant_name.to_string().to_uppercase();
        extra_wgsl_lines.push(format!("const {}: u32 = {}u;", const_name, i));
    }

    extra_wgsl_lines.push("".to_string());
    extra_wgsl_lines.push("// MultiParticle type helpers".to_string());

    for (i, (variant_name, _)) in variant_info.iter().enumerate() {
        let fn_name = format!("is_{}", variant_name.to_string().to_lowercase());
        extra_wgsl_lines.push(format!(
            "fn {}(p: Particle) -> bool {{ return p.particle_type == {}u; }}",
            fn_name, i
        ));
    }

    let extra_wgsl = extra_wgsl_lines.join("\n");

    // Generate to_gpu() match arms for enum (matching on struct-like variants)
    let to_gpu_arms: Vec<_> = variant_info
        .iter()
        .enumerate()
        .map(|(idx, (variant_name, variant_fields))| {
            let idx_u32 = idx as u32;

            // Generate field bindings for the match pattern
            let field_bindings: Vec<_> = variant_fields.iter().map(|(fname, _, _)| {
                quote! { #fname }
            }).collect();

            // Generate field assignments for the GPU struct
            let mut field_assignments = Vec::new();

            for (fname, ftype, _) in &all_fields {
                let field_ident = Ident::new(fname, Span::call_site());

                // Check if this variant has this field
                let has_field = variant_fields.iter().any(|(vf, _, _)| vf.to_string() == *fname);

                if has_field {
                    let type_info = type_info_from_string(ftype);
                    if type_info.needs_to_array {
                        field_assignments.push(quote! { #field_ident: #field_ident.to_array() });
                    } else {
                        field_assignments.push(quote! { #field_ident: *#field_ident });
                    }
                } else {
                    let zero_val = zero_value_for_type(ftype);
                    field_assignments.push(quote! { #field_ident: #zero_val });
                }
            }

            // Particle type and lifecycle
            field_assignments.push(quote! { particle_type: #idx_u32 });
            field_assignments.push(quote! { age: 0.0 });
            field_assignments.push(quote! { alive: 1u32 });
            field_assignments.push(quote! { scale: 1.0 });

            quote! {
                #enum_name::#variant_name { #(#field_bindings),* } => {
                    #enum_gpu_name {
                        #(#field_assignments,)*
                        ..bytemuck::Zeroable::zeroed()
                    }
                }
            }
        })
        .collect();

    // Generate from_gpu() match arms for enum
    let from_gpu_arms: Vec<_> = variant_info
        .iter()
        .enumerate()
        .map(|(idx, (variant_name, variant_fields))| {
            let idx_u32 = idx as u32;

            // Generate field assignments from GPU to enum variant
            let field_assignments: Vec<_> = variant_fields.iter().map(|(fname, ftype, _)| {
                let type_normalized = ftype.replace("glam::", "");
                match type_normalized.as_str() {
                    "Vec3" => quote! { #fname: rdpe::Vec3::from_array(gpu.#fname) },
                    "Vec2" => quote! { #fname: rdpe::Vec2::from_array(gpu.#fname) },
                    "Vec4" => quote! { #fname: rdpe::Vec4::from_array(gpu.#fname) },
                    _ => quote! { #fname: gpu.#fname },
                }
            }).collect();

            // Need a default case for the last variant
            if idx == variant_info.len() - 1 {
                quote! {
                    _ => #enum_name::#variant_name { #(#field_assignments),* }
                }
            } else {
                quote! {
                    #idx_u32 => #enum_name::#variant_name { #(#field_assignments),* },
                }
            }
        })
        .collect();

    // Generate inspect_fields() match arms for enum
    let inspect_arms: Vec<_> = variant_info
        .iter()
        .map(|(variant_name, variant_fields)| {
            // Generate field bindings for the match pattern
            let field_bindings: Vec<_> = variant_fields.iter().map(|(fname, _, _)| {
                quote! { #fname }
            }).collect();

            // Generate inspect entries
            let inspect_entries: Vec<_> = variant_fields.iter().map(|(fname, ftype, _)| {
                let fname_str = fname.to_string();
                let type_normalized = ftype.replace("glam::", "");
                match type_normalized.as_str() {
                    "Vec3" => quote! { (#fname_str, format!("({:.3}, {:.3}, {:.3})", #fname.x, #fname.y, #fname.z)) },
                    "Vec2" => quote! { (#fname_str, format!("({:.3}, {:.3})", #fname.x, #fname.y)) },
                    "Vec4" => quote! { (#fname_str, format!("({:.3}, {:.3}, {:.3}, {:.3})", #fname.x, #fname.y, #fname.z, #fname.w)) },
                    "f32" => quote! { (#fname_str, format!("{:.3}", #fname)) },
                    "u32" | "i32" => quote! { (#fname_str, format!("{}", #fname)) },
                    _ => quote! { (#fname_str, format!("{:?}", #fname)) },
                }
            }).collect();

            quote! {
                #enum_name::#variant_name { #(#field_bindings),* } => {
                    vec![#(#inspect_entries),*]
                }
            }
        })
        .collect();

    // Generate render_editable_fields() match arms for enum
    let editable_arms: Vec<_> = variant_info
        .iter()
        .map(|(variant_name, variant_fields)| {
            // Generate mutable field bindings for the match pattern
            let field_bindings: Vec<_> = variant_fields.iter().map(|(fname, _, _)| {
                quote! { #fname }
            }).collect();

            // Generate editable widgets for each field
            let editable_widgets: Vec<_> = variant_fields.iter().map(|(fname, ftype, _)| {
                let fname_str = fname.to_string();
                let type_normalized = ftype.replace("glam::", "");
                match type_normalized.as_str() {
                    "Vec3" => quote! {
                        ui.label(#fname_str);
                        ui.horizontal(|ui| {
                            if ui.add(egui::DragValue::new(&mut #fname.x).speed(0.01).prefix("x: ")).changed() {
                                modified = true;
                            }
                            if ui.add(egui::DragValue::new(&mut #fname.y).speed(0.01).prefix("y: ")).changed() {
                                modified = true;
                            }
                            if ui.add(egui::DragValue::new(&mut #fname.z).speed(0.01).prefix("z: ")).changed() {
                                modified = true;
                            }
                        });
                        ui.end_row();
                    },
                    "Vec2" => quote! {
                        ui.label(#fname_str);
                        ui.horizontal(|ui| {
                            if ui.add(egui::DragValue::new(&mut #fname.x).speed(0.01).prefix("x: ")).changed() {
                                modified = true;
                            }
                            if ui.add(egui::DragValue::new(&mut #fname.y).speed(0.01).prefix("y: ")).changed() {
                                modified = true;
                            }
                        });
                        ui.end_row();
                    },
                    "Vec4" => quote! {
                        ui.label(#fname_str);
                        ui.horizontal(|ui| {
                            if ui.add(egui::DragValue::new(&mut #fname.x).speed(0.01).prefix("x: ")).changed() {
                                modified = true;
                            }
                            if ui.add(egui::DragValue::new(&mut #fname.y).speed(0.01).prefix("y: ")).changed() {
                                modified = true;
                            }
                            if ui.add(egui::DragValue::new(&mut #fname.z).speed(0.01).prefix("z: ")).changed() {
                                modified = true;
                            }
                            if ui.add(egui::DragValue::new(&mut #fname.w).speed(0.01).prefix("w: ")).changed() {
                                modified = true;
                            }
                        });
                        ui.end_row();
                    },
                    "f32" => quote! {
                        ui.label(#fname_str);
                        if ui.add(egui::DragValue::new(#fname).speed(0.01)).changed() {
                            modified = true;
                        }
                        ui.end_row();
                    },
                    "u32" => quote! {
                        ui.label(#fname_str);
                        let mut val = *#fname as i64;
                        if ui.add(egui::DragValue::new(&mut val).speed(1.0)).changed() {
                            *#fname = val.max(0) as u32;
                            modified = true;
                        }
                        ui.end_row();
                    },
                    "i32" => quote! {
                        ui.label(#fname_str);
                        if ui.add(egui::DragValue::new(#fname).speed(1.0)).changed() {
                            modified = true;
                        }
                        ui.end_row();
                    },
                    _ => quote! {
                        ui.label(#fname_str);
                        ui.label(format!("{:?}", #fname));
                        ui.end_row();
                    },
                }
            }).collect();

            quote! {
                #enum_name::#variant_name { #(#field_bindings),* } => {
                    egui::Grid::new("editable_fields")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            #(#editable_widgets)*
                        });
                }
            }
        })
        .collect();

    // Generate type ID constants for the enum
    let type_constants: Vec<_> = variant_info
        .iter()
        .enumerate()
        .map(|(idx, (variant_name, _))| {
            let const_name = Ident::new(
                &variant_name.to_string().to_uppercase(),
                Span::call_site(),
            );
            let idx_u32 = idx as u32;
            quote! {
                /// Type ID for use in typed rules (Chase, Evade, Typed, etc.)
                pub const #const_name: u32 = #idx_u32;
            }
        })
        .collect();

    let expanded = quote! {
        // Standalone structs with full Particle implementations
        #(#standalone_structs)*

        // Type ID constants for the enum
        impl #enum_name {
            #(#type_constants)*
        }

        // Unified GPU struct for the enum (we don't re-declare the enum itself!)
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        #visibility struct #enum_gpu_name {
            #(#enum_gpu_field_tokens),*
        }

        impl rdpe::ParticleTrait for #enum_name {
            type Gpu = #enum_gpu_name;

            const WGSL_STRUCT: &'static str = #enum_wgsl_struct;
            const COLOR_FIELD: Option<&'static str> = None;
            const COLOR_OFFSET: Option<u32> = None;
            const ALIVE_OFFSET: u32 = #enum_alive_offset;
            const SCALE_OFFSET: u32 = #enum_scale_offset;
            const EXTRA_WGSL: &'static str = #extra_wgsl;

            fn to_gpu(&self) -> Self::Gpu {
                match self {
                    #(#to_gpu_arms)*
                }
            }

            fn from_gpu(gpu: &Self::Gpu) -> Self {
                match gpu.particle_type {
                    #(#from_gpu_arms)*
                }
            }

            fn inspect_fields(&self) -> Vec<(&'static str, String)> {
                match self {
                    #(#inspect_arms)*
                }
            }

            #[cfg(feature = "egui")]
            fn render_editable_fields(&mut self, ui: &mut egui::Ui) -> bool {
                let mut modified = false;
                match self {
                    #(#editable_arms)*
                }
                modified
            }
        }
    };

    TokenStream::from(expanded)
}

/// Generate a Rust type token from a type string
fn rust_type_from_string(ty: &str) -> proc_macro2::TokenStream {
    match ty.replace("glam::", "").as_str() {
        "Vec3" => quote! { rdpe::Vec3 },
        "Vec2" => quote! { rdpe::Vec2 },
        "Vec4" => quote! { rdpe::Vec4 },
        "f32" => quote! { f32 },
        "u32" => quote! { u32 },
        "i32" => quote! { i32 },
        _ => panic!("Unsupported type: {}", ty),
    }
}

/// Generate GPU struct fields, WGSL, and offsets for a single particle type
fn generate_particle_gpu_struct(
    fields: &[(Ident, String, bool)],
    _is_standalone: bool,
) -> (Vec<(String, proc_macro2::TokenStream)>, String, Option<String>, Option<u32>, u32, u32) {
    let mut gpu_fields: Vec<(String, proc_macro2::TokenStream)> = Vec::new();
    let mut wgsl_lines = Vec::new();
    let mut field_offset = 0u32;
    let mut padding_count = 0u32;
    let mut color_field: Option<String> = None;
    let mut color_offset: Option<u32> = None;

    for (field_name, type_str, is_color) in fields {
        let type_info = type_info_from_string(&type_str.replace("glam::", ""));

        // Add padding if needed
        let padding_needed = (type_info.align - (field_offset % type_info.align)) % type_info.align;
        if padding_needed > 0 {
            let pad_name = format!("_pad{}", padding_count);
            padding_count += 1;

            if padding_needed == 4 {
                wgsl_lines.push(format!("    {}: f32,", pad_name));
                gpu_fields.push((pad_name, quote! { f32 }));
            } else {
                let count = (padding_needed / 4) as usize;
                wgsl_lines.push(format!("    {}: array<f32, {}>,", pad_name, count));
                gpu_fields.push((pad_name, quote! { [f32; #count] }));
            }
            field_offset += padding_needed;
        }

        if *is_color {
            color_field = Some(field_name.to_string());
            color_offset = Some(field_offset);
        }

        wgsl_lines.push(format!("    {}: {},", field_name, type_info.wgsl_type));
        gpu_fields.push((field_name.to_string(), type_info.gpu_type.clone()));
        field_offset += type_info.size;
    }

    // Add particle_type
    {
        let padding_needed = (4 - (field_offset % 4)) % 4;
        if padding_needed > 0 {
            let pad_name = format!("_pad{}", padding_count);
            padding_count += 1;
            let count = (padding_needed / 4) as usize;
            if count == 1 {
                wgsl_lines.push(format!("    {}: f32,", pad_name));
                gpu_fields.push((pad_name, quote! { f32 }));
            } else {
                wgsl_lines.push(format!("    {}: array<f32, {}>,", pad_name, count));
                gpu_fields.push((pad_name, quote! { [f32; #count] }));
            }
            field_offset += padding_needed;
        }

        wgsl_lines.push("    particle_type: u32,".to_string());
        gpu_fields.push(("particle_type".to_string(), quote! { u32 }));
        field_offset += 4;
    }

    // Add lifecycle fields
    wgsl_lines.push("    age: f32,".to_string());
    gpu_fields.push(("age".to_string(), quote! { f32 }));
    field_offset += 4;

    let alive_offset = field_offset;
    wgsl_lines.push("    alive: u32,".to_string());
    gpu_fields.push(("alive".to_string(), quote! { u32 }));
    field_offset += 4;

    let scale_offset = field_offset;
    wgsl_lines.push("    scale: f32,".to_string());
    gpu_fields.push(("scale".to_string(), quote! { f32 }));
    field_offset += 4;

    // Final padding
    let final_padding = (16 - (field_offset % 16)) % 16;
    if final_padding > 0 {
        let pad_name = format!("_pad{}", padding_count);
        if final_padding == 4 {
            wgsl_lines.push(format!("    {}: f32,", pad_name));
            gpu_fields.push((pad_name, quote! { f32 }));
        } else {
            let count = (final_padding / 4) as usize;
            wgsl_lines.push(format!("    {}: array<f32, {}>,", pad_name, count));
            gpu_fields.push((pad_name, quote! { [f32; #count] }));
        }
    }

    let wgsl_struct = format!("struct Particle {{\n{}\n}}", wgsl_lines.join("\n"));

    (gpu_fields, wgsl_struct, color_field, color_offset, alive_offset, scale_offset)
}

/// Generate unified GPU struct for the enum (containing all fields from all variants)
fn generate_unified_gpu_struct(
    all_fields: &[(String, String, bool)],
) -> (Vec<(String, proc_macro2::TokenStream)>, String, Option<String>, Option<u32>, u32, u32) {
    let mut gpu_fields: Vec<(String, proc_macro2::TokenStream)> = Vec::new();
    let mut wgsl_lines = Vec::new();
    let mut field_offset = 0u32;
    let mut padding_count = 0u32;

    for (field_name, type_str, _) in all_fields {
        let type_info = type_info_from_string(type_str);

        // Add padding if needed
        let padding_needed = (type_info.align - (field_offset % type_info.align)) % type_info.align;
        if padding_needed > 0 {
            let pad_name = format!("_pad{}", padding_count);
            padding_count += 1;

            if padding_needed == 4 {
                wgsl_lines.push(format!("    {}: f32,", pad_name));
                gpu_fields.push((pad_name, quote! { f32 }));
            } else {
                let count = (padding_needed / 4) as usize;
                wgsl_lines.push(format!("    {}: array<f32, {}>,", pad_name, count));
                gpu_fields.push((pad_name, quote! { [f32; #count] }));
            }
            field_offset += padding_needed;
        }

        wgsl_lines.push(format!("    {}: {},", field_name, type_info.wgsl_type));
        gpu_fields.push((field_name.clone(), type_info.gpu_type.clone()));
        field_offset += type_info.size;
    }

    // Add particle_type
    {
        let padding_needed = (4 - (field_offset % 4)) % 4;
        if padding_needed > 0 {
            let pad_name = format!("_pad{}", padding_count);
            padding_count += 1;
            let count = (padding_needed / 4) as usize;
            if count == 1 {
                wgsl_lines.push(format!("    {}: f32,", pad_name));
                gpu_fields.push((pad_name, quote! { f32 }));
            } else {
                wgsl_lines.push(format!("    {}: array<f32, {}>,", pad_name, count));
                gpu_fields.push((pad_name, quote! { [f32; #count] }));
            }
            field_offset += padding_needed;
        }

        wgsl_lines.push("    particle_type: u32,".to_string());
        gpu_fields.push(("particle_type".to_string(), quote! { u32 }));
        field_offset += 4;
    }

    // Add lifecycle fields
    wgsl_lines.push("    age: f32,".to_string());
    gpu_fields.push(("age".to_string(), quote! { f32 }));
    field_offset += 4;

    let alive_offset = field_offset;
    wgsl_lines.push("    alive: u32,".to_string());
    gpu_fields.push(("alive".to_string(), quote! { u32 }));
    field_offset += 4;

    let scale_offset = field_offset;
    wgsl_lines.push("    scale: f32,".to_string());
    gpu_fields.push(("scale".to_string(), quote! { f32 }));
    field_offset += 4;

    // Final padding
    let final_padding = (16 - (field_offset % 16)) % 16;
    if final_padding > 0 {
        let pad_name = format!("_pad{}", padding_count);
        if final_padding == 4 {
            wgsl_lines.push(format!("    {}: f32,", pad_name));
            gpu_fields.push((pad_name, quote! { f32 }));
        } else {
            let count = (final_padding / 4) as usize;
            wgsl_lines.push(format!("    {}: array<f32, {}>,", pad_name, count));
            gpu_fields.push((pad_name, quote! { [f32; #count] }));
        }
    }

    let wgsl_struct = format!("struct Particle {{\n{}\n}}", wgsl_lines.join("\n"));

    (gpu_fields, wgsl_struct, None, None, alive_offset, scale_offset)
}

/// Type info from string for MultiParticle macro
struct MultiTypeInfo {
    wgsl_type: &'static str,
    gpu_type: proc_macro2::TokenStream,
    size: u32,
    align: u32,
    needs_to_array: bool,
}

fn type_info_from_string(ty: &str) -> MultiTypeInfo {
    match ty {
        "Vec3" => MultiTypeInfo {
            wgsl_type: "vec3<f32>",
            gpu_type: quote! { [f32; 3] },
            size: 12,
            align: 16,
            needs_to_array: true,
        },
        "Vec2" => MultiTypeInfo {
            wgsl_type: "vec2<f32>",
            gpu_type: quote! { [f32; 2] },
            size: 8,
            align: 8,
            needs_to_array: true,
        },
        "Vec4" => MultiTypeInfo {
            wgsl_type: "vec4<f32>",
            gpu_type: quote! { [f32; 4] },
            size: 16,
            align: 16,
            needs_to_array: true,
        },
        "f32" => MultiTypeInfo {
            wgsl_type: "f32",
            gpu_type: quote! { f32 },
            size: 4,
            align: 4,
            needs_to_array: false,
        },
        "u32" => MultiTypeInfo {
            wgsl_type: "u32",
            gpu_type: quote! { u32 },
            size: 4,
            align: 4,
            needs_to_array: false,
        },
        "i32" => MultiTypeInfo {
            wgsl_type: "i32",
            gpu_type: quote! { i32 },
            size: 4,
            align: 4,
            needs_to_array: false,
        },
        _ => panic!("Unsupported type in MultiParticle: {}", ty),
    }
}

fn zero_value_for_type(ty: &str) -> proc_macro2::TokenStream {
    match ty {
        "Vec3" => quote! { [0.0, 0.0, 0.0] },
        "Vec2" => quote! { [0.0, 0.0] },
        "Vec4" => quote! { [0.0, 0.0, 0.0, 0.0] },
        "f32" => quote! { 0.0 },
        "u32" => quote! { 0 },
        "i32" => quote! { 0 },
        _ => quote! { Default::default() },
    }
}
