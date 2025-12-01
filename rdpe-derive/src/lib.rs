use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

/// Derive macro for particle structs.
///
/// Automatically generates a GPU-compatible struct with proper padding,
/// the WGSL definition, and conversion methods. Users write clean Rust,
/// the macro handles GPU memory layout.
///
/// # Example
/// ```ignore
/// #[derive(Particle, Clone)]
/// struct MyParticle {
///     position: Vec3,
///     velocity: Vec3,
///     #[color]
///     tint: Vec3,
/// }
/// ```
///
/// # Supported field types
/// - `Vec3` -> `vec3<f32>` (auto-padded to 16-byte alignment)
/// - `Vec2` -> `vec2<f32>`
/// - `Vec4` -> `vec4<f32>`
/// - `f32` -> `f32`
/// - `u32` -> `u32`
/// - `i32` -> `i32`
///
/// # Attributes
/// - `#[color]` - marks a Vec3 field as the particle color for rendering
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

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;
        let type_info = rust_type_info(field_type);

        // Check for #[color] attribute
        for attr in &field.attrs {
            if attr.path().is_ident("color") {
                color_field = Some(field_name_str.clone());
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

        // Add the actual field
        wgsl_fields.push(format!("    {}: {},", field_name_str, type_info.wgsl_type));

        let gpu_field_type = type_info.gpu_type;
        gpu_struct_fields.push(quote! { #field_name: #gpu_field_type });

        let conversion = generate_conversion(field_name, field_type);
        to_gpu_conversions.push(quote! { #field_name: #conversion });

        field_offset += type_info.size;
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

            fn to_gpu(&self) -> Self::Gpu {
                #gpu_name {
                    #(#to_gpu_conversions),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

struct TypeInfo {
    wgsl_type: &'static str,
    gpu_type: proc_macro2::TokenStream,
    size: u32,
    align: u32,
}

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
