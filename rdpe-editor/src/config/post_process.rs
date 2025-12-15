//! Post-processing configuration types

use serde::{Deserialize, Serialize};

/// Format a float for WGSL, ensuring it always has a decimal point
fn wgsl_float(v: f32) -> String {
    let s = format!("{}", v);
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{}.0", s)
    }
}

/// Individual post-processing effect with its parameters
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PostProcessEffect {
    /// Bloom - bright areas glow
    Bloom {
        intensity: f32,
        threshold: f32,
        radius: f32,
    },
    /// Chromatic aberration - RGB color fringing
    ChromaticAberration {
        intensity: f32,
        #[serde(default)]
        radial: bool,
    },
    /// Vignette - darkened edges
    Vignette {
        intensity: f32,
        softness: f32,
    },
    /// CRT/Retro scanlines
    CRT {
        intensity: f32,
        scanline_count: f32,
    },
    /// Film grain noise
    FilmGrain {
        intensity: f32,
        size: f32,
    },
    /// Pixelate - retro pixel art
    Pixelate {
        block_size: f32,
    },
    /// Grayscale / desaturate
    Grayscale {
        intensity: f32,
    },
    /// Sepia tone
    Sepia {
        intensity: f32,
    },
    /// Posterize - reduce color levels
    Posterize {
        levels: f32,
    },
    /// Sharpen edges
    Sharpen {
        intensity: f32,
    },
    /// Blur
    Blur {
        radius: f32,
    },
    /// Edge glow / neon outline
    EdgeGlow {
        intensity: f32,
        threshold: f32,
        color: [f32; 3],
    },
    /// Barrel distortion / fisheye
    BarrelDistortion {
        intensity: f32,
    },
    /// Ripple / wave distortion
    Ripple {
        intensity: f32,
        frequency: f32,
        speed: f32,
    },
    /// Digital glitch effect
    Glitch {
        intensity: f32,
        block_size: f32,
    },
    /// Night vision
    NightVision {
        intensity: f32,
    },
    /// Invert colors
    Invert {
        intensity: f32,
    },
    /// Color tint
    Tint {
        color: [f32; 3],
        intensity: f32,
    },
    /// Custom WGSL shader
    Custom {
        code: String,
    },
    /// Dithering - ordered Bayer dithering for retro look
    Dithering {
        intensity: f32,
        scale: f32,
    },
    /// Halftone - comic book dot pattern
    Halftone {
        scale: f32,
        angle: f32,
    },
    /// Scanline interlace - old TV interlacing
    ScanlineInterlace {
        intensity: f32,
        speed: f32,
    },
    /// Radial blur - zoom/motion blur from center
    RadialBlur {
        intensity: f32,
        center_x: f32,
        center_y: f32,
    },
    /// Kaleidoscope - mirror/repeat pattern
    Kaleidoscope {
        segments: f32,
        rotation: f32,
    },
    /// Emboss - raised 3D surface effect
    Emboss {
        intensity: f32,
        angle: f32,
    },
    /// Duotone - map to two colors
    Duotone {
        color1: [f32; 3],
        color2: [f32; 3],
    },
    /// Thermal/heat map - false color infrared
    Thermal {
        intensity: f32,
    },
    /// Brightness/Contrast/Saturation adjustments
    ColorAdjust {
        brightness: f32,
        contrast: f32,
        saturation: f32,
    },
    /// Sobel outline - hard edge detection
    SobelOutline {
        intensity: f32,
        threshold: f32,
        color: [f32; 3],
    },
}

impl PostProcessEffect {
    /// Get the display name for this effect
    pub fn name(&self) -> &'static str {
        match self {
            PostProcessEffect::Bloom { .. } => "Bloom",
            PostProcessEffect::ChromaticAberration { .. } => "Chromatic Aberration",
            PostProcessEffect::Vignette { .. } => "Vignette",
            PostProcessEffect::CRT { .. } => "CRT",
            PostProcessEffect::FilmGrain { .. } => "Film Grain",
            PostProcessEffect::Pixelate { .. } => "Pixelate",
            PostProcessEffect::Grayscale { .. } => "Grayscale",
            PostProcessEffect::Sepia { .. } => "Sepia",
            PostProcessEffect::Posterize { .. } => "Posterize",
            PostProcessEffect::Sharpen { .. } => "Sharpen",
            PostProcessEffect::Blur { .. } => "Blur",
            PostProcessEffect::EdgeGlow { .. } => "Edge Glow",
            PostProcessEffect::BarrelDistortion { .. } => "Barrel Distortion",
            PostProcessEffect::Ripple { .. } => "Ripple",
            PostProcessEffect::Glitch { .. } => "Glitch",
            PostProcessEffect::NightVision { .. } => "Night Vision",
            PostProcessEffect::Invert { .. } => "Invert",
            PostProcessEffect::Tint { .. } => "Tint",
            PostProcessEffect::Custom { .. } => "Custom",
            PostProcessEffect::Dithering { .. } => "Dithering",
            PostProcessEffect::Halftone { .. } => "Halftone",
            PostProcessEffect::ScanlineInterlace { .. } => "Scanline Interlace",
            PostProcessEffect::RadialBlur { .. } => "Radial Blur",
            PostProcessEffect::Kaleidoscope { .. } => "Kaleidoscope",
            PostProcessEffect::Emboss { .. } => "Emboss",
            PostProcessEffect::Duotone { .. } => "Duotone",
            PostProcessEffect::Thermal { .. } => "Thermal",
            PostProcessEffect::ColorAdjust { .. } => "Color Adjust",
            PostProcessEffect::SobelOutline { .. } => "Sobel Outline",
        }
    }

    /// Get a short description of the effect
    pub fn description(&self) -> &'static str {
        match self {
            PostProcessEffect::Bloom { .. } => "Bright areas glow and bleed",
            PostProcessEffect::ChromaticAberration { .. } => "RGB color channel separation",
            PostProcessEffect::Vignette { .. } => "Darken edges of screen",
            PostProcessEffect::CRT { .. } => "Retro CRT monitor scanlines",
            PostProcessEffect::FilmGrain { .. } => "Cinematic film noise",
            PostProcessEffect::Pixelate { .. } => "Retro pixelated look (place early)",
            PostProcessEffect::Grayscale { .. } => "Desaturate to black & white",
            PostProcessEffect::Sepia { .. } => "Vintage brownish tint",
            PostProcessEffect::Posterize { .. } => "Reduce color levels",
            PostProcessEffect::Sharpen { .. } => "Enhance edge contrast",
            PostProcessEffect::Blur { .. } => "Soften/blur the image (place early)",
            PostProcessEffect::EdgeGlow { .. } => "Neon glow on edges",
            PostProcessEffect::BarrelDistortion { .. } => "Fisheye lens distortion (place early)",
            PostProcessEffect::Ripple { .. } => "Animated wave distortion (place early)",
            PostProcessEffect::Glitch { .. } => "Digital glitch artifacts (place early)",
            PostProcessEffect::NightVision { .. } => "Green-tinted night vision",
            PostProcessEffect::Invert { .. } => "Invert all colors",
            PostProcessEffect::Tint { .. } => "Apply color tint",
            PostProcessEffect::Custom { .. } => "Custom WGSL shader code",
            PostProcessEffect::Dithering { .. } => "Ordered Bayer dithering pattern",
            PostProcessEffect::Halftone { .. } => "Comic book dot pattern (place early)",
            PostProcessEffect::ScanlineInterlace { .. } => "Old TV interlacing effect",
            PostProcessEffect::RadialBlur { .. } => "Zoom blur from center (place early)",
            PostProcessEffect::Kaleidoscope { .. } => "Mirror pattern (place early)",
            PostProcessEffect::Emboss { .. } => "Raised 3D surface (place early)",
            PostProcessEffect::Duotone { .. } => "Map image to two colors",
            PostProcessEffect::Thermal { .. } => "False color heat map",
            PostProcessEffect::ColorAdjust { .. } => "Brightness/contrast/saturation",
            PostProcessEffect::SobelOutline { .. } => "Hard edge detection outline",
        }
    }

    /// Create default instance of each effect type
    pub fn default_bloom() -> Self {
        PostProcessEffect::Bloom { intensity: 0.5, threshold: 0.7, radius: 0.003 }
    }
    pub fn default_chromatic_aberration() -> Self {
        PostProcessEffect::ChromaticAberration { intensity: 0.5, radial: false }
    }
    pub fn default_vignette() -> Self {
        PostProcessEffect::Vignette { intensity: 0.5, softness: 0.5 }
    }
    pub fn default_crt() -> Self {
        PostProcessEffect::CRT { intensity: 0.5, scanline_count: 300.0 }
    }
    pub fn default_film_grain() -> Self {
        PostProcessEffect::FilmGrain { intensity: 0.3, size: 1.0 }
    }
    pub fn default_pixelate() -> Self {
        PostProcessEffect::Pixelate { block_size: 4.0 }
    }
    pub fn default_grayscale() -> Self {
        PostProcessEffect::Grayscale { intensity: 1.0 }
    }
    pub fn default_sepia() -> Self {
        PostProcessEffect::Sepia { intensity: 0.8 }
    }
    pub fn default_posterize() -> Self {
        PostProcessEffect::Posterize { levels: 4.0 }
    }
    pub fn default_sharpen() -> Self {
        PostProcessEffect::Sharpen { intensity: 0.5 }
    }
    pub fn default_blur() -> Self {
        PostProcessEffect::Blur { radius: 2.0 }
    }
    pub fn default_edge_glow() -> Self {
        PostProcessEffect::EdgeGlow { intensity: 0.7, threshold: 0.3, color: [0.0, 1.0, 0.5] }
    }
    pub fn default_barrel_distortion() -> Self {
        PostProcessEffect::BarrelDistortion { intensity: 0.3 }
    }
    pub fn default_ripple() -> Self {
        PostProcessEffect::Ripple { intensity: 0.02, frequency: 20.0, speed: 3.0 }
    }
    pub fn default_glitch() -> Self {
        PostProcessEffect::Glitch { intensity: 0.3, block_size: 0.05 }
    }
    pub fn default_night_vision() -> Self {
        PostProcessEffect::NightVision { intensity: 1.0 }
    }
    pub fn default_invert() -> Self {
        PostProcessEffect::Invert { intensity: 1.0 }
    }
    pub fn default_tint() -> Self {
        PostProcessEffect::Tint { color: [1.0, 0.8, 0.6], intensity: 0.3 }
    }
    pub fn default_custom() -> Self {
        PostProcessEffect::Custom { code: DEFAULT_CUSTOM_CODE.to_string() }
    }
    pub fn default_dithering() -> Self {
        PostProcessEffect::Dithering { intensity: 1.0, scale: 1.0 }
    }
    pub fn default_halftone() -> Self {
        PostProcessEffect::Halftone { scale: 4.0, angle: 0.0 }
    }
    pub fn default_scanline_interlace() -> Self {
        PostProcessEffect::ScanlineInterlace { intensity: 0.5, speed: 1.0 }
    }
    pub fn default_radial_blur() -> Self {
        PostProcessEffect::RadialBlur { intensity: 0.3, center_x: 0.5, center_y: 0.5 }
    }
    pub fn default_kaleidoscope() -> Self {
        PostProcessEffect::Kaleidoscope { segments: 6.0, rotation: 0.0 }
    }
    pub fn default_emboss() -> Self {
        PostProcessEffect::Emboss { intensity: 1.0, angle: 0.0 }
    }
    pub fn default_duotone() -> Self {
        PostProcessEffect::Duotone { color1: [0.1, 0.1, 0.3], color2: [1.0, 0.9, 0.7] }
    }
    pub fn default_thermal() -> Self {
        PostProcessEffect::Thermal { intensity: 1.0 }
    }
    pub fn default_color_adjust() -> Self {
        PostProcessEffect::ColorAdjust { brightness: 0.0, contrast: 1.0, saturation: 1.0 }
    }
    pub fn default_sobel_outline() -> Self {
        PostProcessEffect::SobelOutline { intensity: 1.0, threshold: 0.1, color: [1.0, 1.0, 1.0] }
    }

    /// Available effect types for the UI
    pub fn available_types() -> &'static [&'static str] {
        &[
            "Bloom", "Chromatic Aberration", "Vignette", "CRT", "Film Grain",
            "Pixelate", "Grayscale", "Sepia", "Posterize", "Sharpen", "Blur",
            "Edge Glow", "Barrel Distortion", "Ripple", "Glitch", "Night Vision",
            "Invert", "Tint", "Custom", "Dithering", "Halftone", "Scanline Interlace",
            "Radial Blur", "Kaleidoscope", "Emboss", "Duotone", "Thermal",
            "Color Adjust", "Sobel Outline"
        ]
    }

    /// Create a default effect from type name
    pub fn from_type_name(name: &str) -> Self {
        match name {
            "Bloom" => Self::default_bloom(),
            "Chromatic Aberration" => Self::default_chromatic_aberration(),
            "Vignette" => Self::default_vignette(),
            "CRT" => Self::default_crt(),
            "Film Grain" => Self::default_film_grain(),
            "Pixelate" => Self::default_pixelate(),
            "Grayscale" => Self::default_grayscale(),
            "Sepia" => Self::default_sepia(),
            "Posterize" => Self::default_posterize(),
            "Sharpen" => Self::default_sharpen(),
            "Blur" => Self::default_blur(),
            "Edge Glow" => Self::default_edge_glow(),
            "Barrel Distortion" => Self::default_barrel_distortion(),
            "Ripple" => Self::default_ripple(),
            "Glitch" => Self::default_glitch(),
            "Night Vision" => Self::default_night_vision(),
            "Invert" => Self::default_invert(),
            "Tint" => Self::default_tint(),
            "Custom" => Self::default_custom(),
            "Dithering" => Self::default_dithering(),
            "Halftone" => Self::default_halftone(),
            "Scanline Interlace" => Self::default_scanline_interlace(),
            "Radial Blur" => Self::default_radial_blur(),
            "Kaleidoscope" => Self::default_kaleidoscope(),
            "Emboss" => Self::default_emboss(),
            "Duotone" => Self::default_duotone(),
            "Thermal" => Self::default_thermal(),
            "Color Adjust" => Self::default_color_adjust(),
            "Sobel Outline" => Self::default_sobel_outline(),
            _ => Self::default_bloom(),
        }
    }

    /// Generate the WGSL shader snippet for this effect
    /// Takes `current_color` as input variable name, outputs to same variable
    pub fn to_wgsl(&self) -> String {
        match self {
            PostProcessEffect::Bloom { intensity, threshold, radius } => format!(
                r#"    // Bloom
    {{
        let blur_size = {} * 2.0;
        var bloom = vec3<f32>(0.0);
        for (var x = -1; x <= 1; x++) {{
            for (var y = -1; y <= 1; y++) {{
                let offset = vec2<f32>(f32(x), f32(y)) * blur_size;
                let s = textureSample(input_texture, input_sampler, uv + offset).rgb;
                let brightness = max(max(s.r, s.g), s.b);
                if brightness > {} {{
                    bloom += s * (brightness - {}) / (1.0 - {});
                }}
            }}
        }}
        bloom /= 9.0;
        current_color = vec4<f32>(current_color.rgb + bloom * {} * 2.0, current_color.a);
    }}"#, wgsl_float(*radius), wgsl_float(*threshold), wgsl_float(*threshold), wgsl_float(*threshold), wgsl_float(*intensity)
            ),

            PostProcessEffect::ChromaticAberration { intensity, radial } => {
                let i = wgsl_float(*intensity);
                if *radial {
                    format!(
                        r#"    // Chromatic Aberration (radial)
    {{
        let center = vec2<f32>(0.5, 0.5);
        let dir = normalize(uv - center);
        let dist = length(uv - center);
        let offset = dir * dist * {i} * 0.02;
        let r = textureSample(input_texture, input_sampler, uv + offset).r;
        let b = textureSample(input_texture, input_sampler, uv - offset).b;
        current_color = vec4<f32>(r, current_color.g, b, current_color.a);
    }}"#
                    )
                } else {
                    format!(
                        r#"    // Chromatic Aberration
    {{
        let offset = {i} * 0.008;
        let r = textureSample(input_texture, input_sampler, uv + vec2<f32>(offset, 0.0)).r;
        let b = textureSample(input_texture, input_sampler, uv - vec2<f32>(offset, 0.0)).b;
        current_color = vec4<f32>(r, current_color.g, b, current_color.a);
    }}"#
                    )
                }
            }

            PostProcessEffect::Vignette { intensity, softness } => format!(
                r#"    // Vignette
    {{
        let center = vec2<f32>(0.5, 0.5);
        let dist = distance(uv, center);
        let inner = 0.2 + (1.0 - {}) * 0.4;
        let outer = 0.7 + {} * 0.3;
        let vignette = 1.0 - smoothstep(inner, outer, dist) * {};
        current_color = vec4<f32>(current_color.rgb * vignette, current_color.a);
    }}"#, wgsl_float(*softness), wgsl_float(*softness), wgsl_float(*intensity)
            ),

            PostProcessEffect::CRT { intensity, scanline_count } => format!(
                r#"    // CRT Scanlines
    {{
        let scanline = sin(uv.y * {} * 3.14159) * 0.5 + 0.5;
        let strength = 0.25 * {};
        current_color = vec4<f32>(current_color.rgb * (1.0 - strength + scanline * strength), current_color.a);
    }}"#, wgsl_float(*scanline_count), wgsl_float(*intensity)
            ),

            PostProcessEffect::FilmGrain { intensity, size } => format!(
                r#"    // Film Grain
    {{
        let noise_uv = uv * pp_resolution / {};
        let noise = fract(sin(dot(noise_uv + pp_time * 0.1, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let grain = (noise - 0.5) * {} * 0.3;
        current_color = vec4<f32>(current_color.rgb + grain, current_color.a);
    }}"#, wgsl_float(*size), wgsl_float(*intensity)
            ),

            PostProcessEffect::Pixelate { block_size } => format!(
                r#"    // Pixelate
    {{
        let pixel_size = {} / pp_resolution;
        let pixelated_uv = floor(uv / pixel_size) * pixel_size + pixel_size * 0.5;
        current_color = textureSample(input_texture, input_sampler, pixelated_uv);
    }}"#, wgsl_float(*block_size)
            ),

            PostProcessEffect::Grayscale { intensity } => format!(
                r#"    // Grayscale
    {{
        let luminance = dot(current_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let gray = vec3<f32>(luminance);
        current_color = vec4<f32>(mix(current_color.rgb, gray, {}), current_color.a);
    }}"#, wgsl_float(*intensity)
            ),

            PostProcessEffect::Sepia { intensity } => format!(
                r#"    // Sepia
    {{
        let sepia_r = dot(current_color.rgb, vec3<f32>(0.393, 0.769, 0.189));
        let sepia_g = dot(current_color.rgb, vec3<f32>(0.349, 0.686, 0.168));
        let sepia_b = dot(current_color.rgb, vec3<f32>(0.272, 0.534, 0.131));
        let sepia = vec3<f32>(sepia_r, sepia_g, sepia_b);
        current_color = vec4<f32>(mix(current_color.rgb, sepia, {}), current_color.a);
    }}"#, wgsl_float(*intensity)
            ),

            PostProcessEffect::Posterize { levels } => format!(
                r#"    // Posterize
    {{
        let levels = {};
        current_color = vec4<f32>(floor(current_color.rgb * levels) / levels, current_color.a);
    }}"#, wgsl_float(*levels)
            ),

            PostProcessEffect::Sharpen { intensity } => format!(
                r#"    // Sharpen
    {{
        let pixel = 1.0 / pp_resolution;
        let n = textureSample(input_texture, input_sampler, uv + vec2<f32>(0.0, -pixel.y)).rgb;
        let s = textureSample(input_texture, input_sampler, uv + vec2<f32>(0.0, pixel.y)).rgb;
        let e = textureSample(input_texture, input_sampler, uv + vec2<f32>(pixel.x, 0.0)).rgb;
        let w = textureSample(input_texture, input_sampler, uv + vec2<f32>(-pixel.x, 0.0)).rgb;
        let sharp = current_color.rgb * (1.0 + 4.0 * {}) - (n + s + e + w) * {};
        current_color = vec4<f32>(clamp(sharp, vec3<f32>(0.0), vec3<f32>(1.0)), current_color.a);
    }}"#, wgsl_float(*intensity), wgsl_float(*intensity)
            ),

            PostProcessEffect::Blur { radius } => format!(
                r#"    // Blur
    {{
        let blur_size = {} / pp_resolution.x;
        var blurred = vec3<f32>(0.0);
        var total = 0.0;
        for (var x = -2; x <= 2; x++) {{
            for (var y = -2; y <= 2; y++) {{
                let offset = vec2<f32>(f32(x), f32(y)) * blur_size;
                let weight = 1.0 / (1.0 + f32(abs(x) + abs(y)));
                blurred += textureSample(input_texture, input_sampler, uv + offset).rgb * weight;
                total += weight;
            }}
        }}
        current_color = vec4<f32>(blurred / total, current_color.a);
    }}"#, wgsl_float(*radius)
            ),

            PostProcessEffect::EdgeGlow { intensity, threshold, color } => format!(
                r#"    // Edge Glow
    {{
        let pixel = 1.0 / pp_resolution;
        let c = current_color.rgb;
        let n = textureSample(input_texture, input_sampler, uv + vec2<f32>(0.0, -pixel.y)).rgb;
        let s = textureSample(input_texture, input_sampler, uv + vec2<f32>(0.0, pixel.y)).rgb;
        let e = textureSample(input_texture, input_sampler, uv + vec2<f32>(pixel.x, 0.0)).rgb;
        let w = textureSample(input_texture, input_sampler, uv + vec2<f32>(-pixel.x, 0.0)).rgb;
        let edge = abs(c - n) + abs(c - s) + abs(c - e) + abs(c - w);
        let edge_strength = (edge.r + edge.g + edge.b) / 3.0;
        let glow_color = vec3<f32>({}, {}, {});
        if edge_strength > {} {{
            current_color = vec4<f32>(current_color.rgb + glow_color * edge_strength * {}, current_color.a);
        }}
    }}"#, wgsl_float(color[0]), wgsl_float(color[1]), wgsl_float(color[2]), wgsl_float(*threshold), wgsl_float(*intensity)
            ),

            PostProcessEffect::BarrelDistortion { intensity } => format!(
                r#"    // Barrel Distortion
    {{
        let center = vec2<f32>(0.5, 0.5);
        let centered_uv = uv - center;
        let dist = length(centered_uv);
        let distorted = centered_uv * (1.0 + dist * dist * {});
        let final_uv = distorted + center;
        if final_uv.x >= 0.0 && final_uv.x <= 1.0 && final_uv.y >= 0.0 && final_uv.y <= 1.0 {{
            current_color = textureSample(input_texture, input_sampler, final_uv);
        }} else {{
            current_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }}
    }}"#, wgsl_float(*intensity)
            ),

            PostProcessEffect::Ripple { intensity, frequency, speed } => format!(
                r#"    // Ripple
    {{
        let center = vec2<f32>(0.5, 0.5);
        let dist = length(uv - center);
        let angle = atan2(uv.y - center.y, uv.x - center.x);
        let wave = sin(dist * {} - pp_time * {}) * {};
        let ripple_uv = uv + normalize(uv - center) * wave;
        current_color = textureSample(input_texture, input_sampler, ripple_uv);
    }}"#, wgsl_float(*frequency), wgsl_float(*speed), wgsl_float(*intensity)
            ),

            PostProcessEffect::Glitch { intensity, block_size } => format!(
                r#"    // Glitch
    {{
        let block = floor(uv.y / {});
        let noise = fract(sin(block * 43758.5453 + floor(pp_time * 10.0)) * 12345.6789);
        if noise > (1.0 - {} * 0.3) {{
            let offset = (fract(sin(block * 12345.6789) * 43758.5453) - 0.5) * 0.1 * {};
            let glitch_uv = uv + vec2<f32>(offset, 0.0);
            current_color = textureSample(input_texture, input_sampler, glitch_uv);
            // Color channel shift
            let r = textureSample(input_texture, input_sampler, glitch_uv + vec2<f32>(0.01, 0.0)).r;
            current_color = vec4<f32>(r, current_color.g, current_color.b, current_color.a);
        }}
    }}"#, wgsl_float(*block_size), wgsl_float(*intensity), wgsl_float(*intensity)
            ),

            PostProcessEffect::NightVision { intensity } => format!(
                r#"    // Night Vision
    {{
        let luminance = dot(current_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let green = vec3<f32>(0.1, luminance * 1.5, 0.1);
        // Scanlines
        let scanline = sin(uv.y * pp_resolution.y * 2.0) * 0.5 + 0.5;
        let scan_green = green * (0.8 + scanline * 0.2);
        // Noise
        let noise = fract(sin(dot(uv * pp_resolution + pp_time, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let noisy = scan_green + (noise - 0.5) * 0.1;
        current_color = vec4<f32>(mix(current_color.rgb, noisy, {}), current_color.a);
    }}"#, wgsl_float(*intensity)
            ),

            PostProcessEffect::Invert { intensity } => format!(
                r#"    // Invert
    {{
        let inverted = vec3<f32>(1.0) - current_color.rgb;
        current_color = vec4<f32>(mix(current_color.rgb, inverted, {}), current_color.a);
    }}"#, wgsl_float(*intensity)
            ),

            PostProcessEffect::Tint { color, intensity } => format!(
                r#"    // Color Tint
    {{
        let tint = vec3<f32>({}, {}, {});
        current_color = vec4<f32>(mix(current_color.rgb, current_color.rgb * tint, {}), current_color.a);
    }}"#, wgsl_float(color[0]), wgsl_float(color[1]), wgsl_float(color[2]), wgsl_float(*intensity)
            ),

            PostProcessEffect::Custom { code } => format!(
                r#"    // Custom Effect
    {{
{}
    }}"#, code
            ),

            PostProcessEffect::Dithering { intensity, scale } => format!(
                r#"    // Dithering (Bayer 4x4)
    {{
        let bayer = array<f32, 16>(
            0.0/16.0, 8.0/16.0, 2.0/16.0, 10.0/16.0,
            12.0/16.0, 4.0/16.0, 14.0/16.0, 6.0/16.0,
            3.0/16.0, 11.0/16.0, 1.0/16.0, 9.0/16.0,
            15.0/16.0, 7.0/16.0, 13.0/16.0, 5.0/16.0
        );
        let pixel = vec2<i32>(i32(uv.x * pp_resolution.x / {}) % 4, i32(uv.y * pp_resolution.y / {}) % 4);
        let threshold = bayer[pixel.y * 4 + pixel.x];
        let dither_amount = (threshold - 0.5) * {} * 0.2;
        current_color = vec4<f32>(current_color.rgb + dither_amount, current_color.a);
    }}"#, wgsl_float(*scale), wgsl_float(*scale), wgsl_float(*intensity)
            ),

            PostProcessEffect::Halftone { scale, angle } => format!(
                r#"    // Halftone
    {{
        let angle_rad = {} * 3.14159 / 180.0;
        let cos_a = cos(angle_rad);
        let sin_a = sin(angle_rad);
        let rotated = vec2<f32>(
            uv.x * cos_a - uv.y * sin_a,
            uv.x * sin_a + uv.y * cos_a
        );
        let grid = rotated * pp_resolution / {};
        let cell = floor(grid);
        let cell_uv = fract(grid) - 0.5;
        let dist = length(cell_uv);
        let luminance = dot(current_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let radius = sqrt(1.0 - luminance) * 0.5;
        let dot_val = 1.0 - smoothstep(radius - 0.05, radius + 0.05, dist);
        current_color = vec4<f32>(vec3<f32>(dot_val), current_color.a);
    }}"#, wgsl_float(*angle), wgsl_float(*scale)
            ),

            PostProcessEffect::ScanlineInterlace { intensity, speed } => format!(
                r#"    // Scanline Interlace
    {{
        let line = floor(uv.y * pp_resolution.y);
        let frame = floor(pp_time * {});
        let interlace = f32((i32(line) + i32(frame)) % 2);
        let dark = 1.0 - {} * 0.5 * interlace;
        current_color = vec4<f32>(current_color.rgb * dark, current_color.a);
    }}"#, wgsl_float(*speed), wgsl_float(*intensity)
            ),

            PostProcessEffect::RadialBlur { intensity, center_x, center_y } => format!(
                r#"    // Radial Blur
    {{
        let center = vec2<f32>({}, {});
        let dir = uv - center;
        var blurred = vec3<f32>(0.0);
        let samples = 8;
        for (var i = 0; i < samples; i++) {{
            let scale = 1.0 - {} * f32(i) / f32(samples) * 0.1;
            let sample_uv = center + dir * scale;
            blurred += textureSample(input_texture, input_sampler, sample_uv).rgb;
        }}
        current_color = vec4<f32>(blurred / f32(samples), current_color.a);
    }}"#, wgsl_float(*center_x), wgsl_float(*center_y), wgsl_float(*intensity)
            ),

            PostProcessEffect::Kaleidoscope { segments, rotation } => format!(
                r#"    // Kaleidoscope
    {{
        let center = vec2<f32>(0.5, 0.5);
        let centered = uv - center;
        var angle = atan2(centered.y, centered.x) + {} * 3.14159 / 180.0;
        let radius = length(centered);
        let segment_angle = 3.14159 * 2.0 / {};
        angle = abs(((angle % segment_angle) + segment_angle) % segment_angle - segment_angle * 0.5);
        let kaleid_uv = center + vec2<f32>(cos(angle), sin(angle)) * radius;
        current_color = textureSample(input_texture, input_sampler, kaleid_uv);
    }}"#, wgsl_float(*rotation), wgsl_float(*segments)
            ),

            PostProcessEffect::Emboss { intensity, angle } => format!(
                r#"    // Emboss
    {{
        let angle_rad = {} * 3.14159 / 180.0;
        let offset = vec2<f32>(cos(angle_rad), sin(angle_rad)) / pp_resolution;
        let c1 = textureSample(input_texture, input_sampler, uv + offset).rgb;
        let c2 = textureSample(input_texture, input_sampler, uv - offset).rgb;
        let emboss = (c1 - c2) * {} + 0.5;
        current_color = vec4<f32>(emboss, current_color.a);
    }}"#, wgsl_float(*angle), wgsl_float(*intensity)
            ),

            PostProcessEffect::Duotone { color1, color2 } => format!(
                r#"    // Duotone
    {{
        let luminance = dot(current_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let dark = vec3<f32>({}, {}, {});
        let light = vec3<f32>({}, {}, {});
        current_color = vec4<f32>(mix(dark, light, luminance), current_color.a);
    }}"#, wgsl_float(color1[0]), wgsl_float(color1[1]), wgsl_float(color1[2]),
                 wgsl_float(color2[0]), wgsl_float(color2[1]), wgsl_float(color2[2])
            ),

            PostProcessEffect::Thermal { intensity } => format!(
                r#"    // Thermal / Heat Map
    {{
        let luminance = dot(current_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        // Heat map: black -> blue -> cyan -> green -> yellow -> red -> white
        var thermal: vec3<f32>;
        if luminance < 0.2 {{
            thermal = mix(vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), luminance * 5.0);
        }} else if luminance < 0.4 {{
            thermal = mix(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 1.0), (luminance - 0.2) * 5.0);
        }} else if luminance < 0.6 {{
            thermal = mix(vec3<f32>(0.0, 1.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), (luminance - 0.4) * 5.0);
        }} else if luminance < 0.8 {{
            thermal = mix(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 1.0, 0.0), (luminance - 0.6) * 5.0);
        }} else {{
            thermal = mix(vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), (luminance - 0.8) * 5.0);
        }}
        current_color = vec4<f32>(mix(current_color.rgb, thermal, {}), current_color.a);
    }}"#, wgsl_float(*intensity)
            ),

            PostProcessEffect::ColorAdjust { brightness, contrast, saturation } => format!(
                r#"    // Color Adjust (Brightness/Contrast/Saturation)
    {{
        var color = current_color.rgb;
        // Brightness
        color = color + {};
        // Contrast
        color = (color - 0.5) * {} + 0.5;
        // Saturation
        let gray = dot(color, vec3<f32>(0.299, 0.587, 0.114));
        color = mix(vec3<f32>(gray), color, {});
        current_color = vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), current_color.a);
    }}"#, wgsl_float(*brightness), wgsl_float(*contrast), wgsl_float(*saturation)
            ),

            PostProcessEffect::SobelOutline { intensity, threshold, color } => format!(
                r#"    // Sobel Outline
    {{
        let pixel = 1.0 / pp_resolution;
        // Sample 3x3 neighborhood
        let tl = dot(textureSample(input_texture, input_sampler, uv + vec2<f32>(-pixel.x, -pixel.y)).rgb, vec3<f32>(0.299, 0.587, 0.114));
        let t  = dot(textureSample(input_texture, input_sampler, uv + vec2<f32>(0.0, -pixel.y)).rgb, vec3<f32>(0.299, 0.587, 0.114));
        let tr = dot(textureSample(input_texture, input_sampler, uv + vec2<f32>(pixel.x, -pixel.y)).rgb, vec3<f32>(0.299, 0.587, 0.114));
        let l  = dot(textureSample(input_texture, input_sampler, uv + vec2<f32>(-pixel.x, 0.0)).rgb, vec3<f32>(0.299, 0.587, 0.114));
        let r  = dot(textureSample(input_texture, input_sampler, uv + vec2<f32>(pixel.x, 0.0)).rgb, vec3<f32>(0.299, 0.587, 0.114));
        let bl = dot(textureSample(input_texture, input_sampler, uv + vec2<f32>(-pixel.x, pixel.y)).rgb, vec3<f32>(0.299, 0.587, 0.114));
        let b  = dot(textureSample(input_texture, input_sampler, uv + vec2<f32>(0.0, pixel.y)).rgb, vec3<f32>(0.299, 0.587, 0.114));
        let br = dot(textureSample(input_texture, input_sampler, uv + vec2<f32>(pixel.x, pixel.y)).rgb, vec3<f32>(0.299, 0.587, 0.114));
        // Sobel kernels
        let gx = -tl - 2.0*l - bl + tr + 2.0*r + br;
        let gy = -tl - 2.0*t - tr + bl + 2.0*b + br;
        let edge = sqrt(gx*gx + gy*gy);
        let outline_color = vec3<f32>({}, {}, {});
        if edge > {} {{
            current_color = vec4<f32>(mix(current_color.rgb, outline_color, edge * {}), current_color.a);
        }}
    }}"#, wgsl_float(color[0]), wgsl_float(color[1]), wgsl_float(color[2]), wgsl_float(*threshold), wgsl_float(*intensity)
            ),
        }
    }
}

/// Post-processing configuration with stackable effects
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PostProcessConfig {
    /// Master enable for post-processing
    pub enabled: bool,
    /// List of effects to apply (in order)
    pub effects: Vec<PostProcessEffect>,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            effects: Vec::new(),
        }
    }
}

impl PostProcessConfig {
    /// Generate the complete combined shader code for all effects
    pub fn generate_shader(&self) -> String {
        if self.effects.is_empty() {
            return SHADER_PASSTHROUGH.to_string();
        }

        let mut effect_code = String::new();
        for effect in &self.effects {
            effect_code.push_str(&effect.to_wgsl());
            effect_code.push('\n');
        }

        format!(
            r#"// Combined post-process shader ({} effects)
var current_color = textureSample(input_texture, input_sampler, uv);

{effect_code}
output_color = current_color;
"#,
            self.effects.len()
        )
    }

    /// Add a new effect to the chain
    pub fn add_effect(&mut self, effect: PostProcessEffect) {
        self.effects.push(effect);
    }

    /// Remove effect at index
    pub fn remove_effect(&mut self, index: usize) {
        if index < self.effects.len() {
            self.effects.remove(index);
        }
    }

    /// Move effect from one position to another
    pub fn move_effect(&mut self, from: usize, to: usize) {
        if from < self.effects.len() && to < self.effects.len() {
            let effect = self.effects.remove(from);
            self.effects.insert(to, effect);
        }
    }
}

const SHADER_PASSTHROUGH: &str = r#"// Passthrough - no effects
let current_color = textureSample(input_texture, input_sampler, uv);
output_color = current_color;
"#;

const DEFAULT_CUSTOM_CODE: &str = r#"        // Custom effect code
        // current_color contains the input
        // Modify current_color to apply your effect
        // Example: current_color = vec4<f32>(1.0 - current_color.rgb, current_color.a);"#;
