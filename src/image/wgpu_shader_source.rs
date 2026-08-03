// Rendering pipeline (in order): black/white point normalize -> scaling curve
// (linear/log/sqrt/asinh) -> gain/offset -> recolor. Rotation happens earlier,
// in UV space, before the texture is sampled at all.
pub const WGPU_SHADER_SOURCE: &str = r#"
struct Uniforms {
    bp: f32,
    wp: f32,
    pan: vec2<f32>,
    zoom: f32,
    rotation: f32,          // radians
    aspect_scale: vec2<f32>,
    bias: f32,              // DS9 colormap bias (default 0.5)
    contrast: f32,          // DS9 colormap contrast (default 1.0)
    posterize_levels: f32,  // used when recolor_mode == 5
    scaling_mode: u32,      // 0=Linear, 1=Log, 2=Sqrt, 3=Asinh
    recolor_mode: u32,      // 0=Gray, 1=Heat, 2=Cool, 3=Rainbow, 4=Iron, 5=Posterize, 6=Tint
    invert: u32,            // 0=Normal, 1=Inverted
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var t_diffuse: texture_2d<f32>;
@group(0) @binding(2) var s_diffuse: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>( 0.0,  0.0), vec2<f32>( 1.0,  0.0), vec2<f32>( 0.0,  1.0),
        vec2<f32>( 1.0,  1.0), vec2<f32>( 0.0,  1.0), vec2<f32>( 1.0,  0.0)
    );
    let p = pos[in_vertex_index];
    var out: VertexOutput;
    out.clip_position = vec4<f32>(p.x * 2.0 - 1.0, (1.0 - p.y) * 2.0 - 1.0, 0.0, 1.0);
    out.tex_coords = p;
    return out;
}

// ---------------------------------------------------------------------------
// UV-space transforms
// ---------------------------------------------------------------------------

// Rotates uv by angle_radians around pivot. Use vec2(0.5, 0.5) to rotate
// around the image center rather than the (0,0) corner.
fn rotate_uv(uv: vec2<f32>, angle_radians: f32, pivot: vec2<f32>) -> vec2<f32> {
    let s = sin(angle_radians);
    let c = cos(angle_radians);
    let centered = uv - pivot;
    let rotated = vec2<f32>(
        centered.x * c - centered.y * s,
        centered.x * s + centered.y * c,
    );
    return rotated + pivot;
}

// ---------------------------------------------------------------------------
// Value-space adjustments
// ---------------------------------------------------------------------------

fn asinh_custom(x: f32) -> f32 {
    return log(x + sqrt(x * x + 1.0));
}

// ---------------------------------------------------------------------------
// SAO DS9 Color Maps
// ---------------------------------------------------------------------------

fn colorize_grayscale(v: f32) -> vec3<f32> {
    return vec3<f32>(v, v, v);
}

// SAO DS9 Heat: black -> purple/blue -> red -> yellow -> white
fn colorize_heat(v: f32) -> vec3<f32> {
    let t = clamp(v, 0.0, 1.0);
    if (t < 0.25) {
        let f = t * 4.0;
        return vec3<f32>(0.0, 0.0, 0.8 * f);
    } else if (t < 0.5) {
        let f = (t - 0.25) * 4.0;
        return vec3<f32>(0.9 * f, 0.0, 0.8 * (1.0 - f) + 0.1 * f);
    } else if (t < 0.75) {
        let f = (t - 0.5) * 4.0;
        return vec3<f32>(0.9 + 0.1 * f, 0.9 * f, 0.0);
    } else {
        let f = (t - 0.75) * 4.0;
        return vec3<f32>(1.0, 0.9 + 0.1 * f, 1.0 * f);
    }
}

fn colorize_cool(v: f32) -> vec3<f32> {
    let t = clamp(v, 0.0, 1.0);
    return mix(vec3<f32>(0.05, 0.05, 0.3), vec3<f32>(1.0, 0.85, 0.4), t);
}

// SAO DS9 Rainbow: blue -> cyan -> green -> yellow -> red
fn colorize_rainbow(v: f32) -> vec3<f32> {
    let t = clamp(v, 0.0, 1.0);
    if (t < 0.25) {
        let f = t * 4.0;
        return vec3<f32>(0.0, f, 1.0);
    } else if (t < 0.5) {
        let f = (t - 0.25) * 4.0;
        return vec3<f32>(0.0, 1.0, 1.0 - f);
    } else if (t < 0.75) {
        let f = (t - 0.5) * 4.0;
        return vec3<f32>(f, 1.0, 0.0);
    } else {
        let f = (t - 0.75) * 4.0;
        return vec3<f32>(1.0, 1.0 - f, 0.0);
    }
}

// SAO DS9 Iron / AIPS0
fn colorize_iron(v: f32) -> vec3<f32> {
    let t = clamp(v, 0.0, 1.0);
    if (t < 0.2) {
        let f = t * 5.0;
        return vec3<f32>(0.1 * f, 0.0, 0.5 * f);
    } else if (t < 0.4) {
        let f = (t - 0.2) * 5.0;
        return vec3<f32>(0.1 + 0.5 * f, 0.0, 0.5 + 0.1 * f);
    } else if (t < 0.6) {
        let f = (t - 0.4) * 5.0;
        return vec3<f32>(0.6 + 0.3 * f, 0.3 * f, 0.6 * (1.0 - f));
    } else if (t < 0.8) {
        let f = (t - 0.6) * 5.0;
        return vec3<f32>(0.9 + 0.1 * f, 0.3 + 0.6 * f, 0.2 * f);
    } else {
        let f = (t - 0.8) * 5.0;
        return vec3<f32>(1.0, 0.9 + 0.1 * f, 0.2 + 0.8 * f);
    }
}

// Contour-style banding
fn colorize_posterize(v: f32, levels: f32) -> vec3<f32> {
    let lev = max(levels, 2.0);
    let step = floor(clamp(v, 0.0, 1.0) * lev) / max(lev - 1.0, 1.0);
    return vec3<f32>(step, step, step);
}

// False-color tint
fn colorize_tint(v: f32, tint: vec3<f32>) -> vec3<f32> {
    return v * tint;
}

// ---------------------------------------------------------------------------
// Fragment entry point
// ---------------------------------------------------------------------------

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = ((in.tex_coords - vec2<f32>(0.5, 0.5)) * uniforms.aspect_scale / uniforms.zoom) - uniforms.pan + vec2<f32>(0.5, 0.5);
    uv = rotate_uv(uv, uniforms.rotation, vec2<f32>(0.5, 0.5));

    // Safety check for boundaries
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let dim_u32 = textureDimensions(t_diffuse);
    let dim_f32 = vec2<f32>(dim_u32);

    var tc = vec2<i32>(uv * dim_f32);
    let max_tc = vec2<i32>(dim_u32) - vec2<i32>(1, 1);

    // Clamp to guarantee we never read outside the texture memory
    tc = clamp(tc, vec2<i32>(0, 0), max_tc);

    // Read the raw 32-bit float value directly from memory
    let raw = textureLoad(t_diffuse, tc, 0).r;

    let range = uniforms.wp - uniforms.bp;
    if (range <= 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let clamped = clamp(raw, uniforms.bp, uniforms.wp);
    var val = (clamped - uniforms.bp) / range;

    if (uniforms.scaling_mode == 1u) {
        val = log(val * 1000.0 + 1.0) / log(1001.0);
    } else if (uniforms.scaling_mode == 2u) {
        val = sqrt(val);
    } else if (uniforms.scaling_mode == 3u) {
        val = asinh_custom(val * 10.0) / asinh_custom(10.0);
    }

    // Apply SAO DS9 Bias and Contrast
    val = clamp((val - uniforms.bias) * uniforms.contrast + 0.5, 0.0, 1.0);

    if (uniforms.invert == 1u) {
        val = 1.0 - val;
    }

    var color: vec3<f32>;
    if (uniforms.recolor_mode == 1u) {
        color = colorize_heat(val);
    } else if (uniforms.recolor_mode == 2u) {
        color = colorize_cool(val);
    } else if (uniforms.recolor_mode == 3u) {
        color = colorize_rainbow(val);
    } else if (uniforms.recolor_mode == 4u) {
        color = colorize_iron(val);
    } else if (uniforms.recolor_mode == 5u) {
        color = colorize_posterize(val, uniforms.posterize_levels);
    } else if (uniforms.recolor_mode == 6u) {
        color = colorize_tint(val, vec3<f32>(0.4, 0.8, 1.0));
    } else {
        color = colorize_grayscale(val);
    }

    return vec4<f32>(color, 1.0);
}
"#;