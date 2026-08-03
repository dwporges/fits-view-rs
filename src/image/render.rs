use crate::image::scalars::Scaling;
use egui::Color32;
use log::{debug, trace};

// Your callback struct
pub struct FitsRenderCallback {
    pub bp: f32,
    pub wp: f32,
    pub pan: egui::Vec2,
    pub zoom: f32,
    pub scaling_mode: u32,
    pub rotation: f32,
    pub bias: f32,
    pub contrast: f32,
    pub recolor_mode: u32,
    pub invert: bool,
    pub posterize_levels: f32,
    pub aspect_scale: egui::Vec2,
    pub slice_index: usize,
}

impl egui_wgpu::CallbackTrait for FitsRenderCallback {
    fn prepare(
        &self,
        _device: &eframe::wgpu::Device,
        queue: &eframe::wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut eframe::wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<eframe::wgpu::CommandBuffer> {
        let gpu_res: &mut FitsGpuResources = resources.get_mut().unwrap();

        if gpu_res.current_slice != self.slice_index {
            let plane_size = (gpu_res.width * gpu_res.height) as usize;
            let offset = self.slice_index * plane_size;
            if offset + plane_size <= gpu_res.all_f32_pixels.len() {
                let slice_data = &gpu_res.all_f32_pixels[offset..offset + plane_size];
                let size = eframe::wgpu::Extent3d {
                    width: gpu_res.width,
                    height: gpu_res.height,
                    depth_or_array_layers: 1,
                };
                queue.write_texture(
                    eframe::wgpu::TexelCopyTextureInfo {
                        texture: &gpu_res.texture,
                        mip_level: 0,
                        origin: eframe::wgpu::Origin3d::ZERO,
                        aspect: eframe::wgpu::TextureAspect::All,
                    },
                    bytemuck::cast_slice(slice_data),
                    eframe::wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * gpu_res.width),
                        rows_per_image: Some(gpu_res.height),
                    },
                    size,
                );
                gpu_res.current_slice = self.slice_index;
            }
        }

        let uniforms = ShaderUniforms {
            bp: self.bp,
            wp: self.wp,
            pan: [self.pan.x, self.pan.y],
            zoom: self.zoom,
            rotation: self.rotation,
            aspect_scale: [self.aspect_scale.x, self.aspect_scale.y],
            bias: self.bias,
            contrast: self.contrast,
            posterize_levels: self.posterize_levels,
            scaling_mode: self.scaling_mode,
            recolor_mode: self.recolor_mode,
            invert: if self.invert { 1 } else { 0 },
            _pad0: 0.0,
            _pad1: 0.0,
        };

        queue.write_buffer(
            &gpu_res.uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let gpu_res: &FitsGpuResources = resources.get().unwrap();
        render_pass.set_pipeline(&gpu_res.pipeline);
        render_pass.set_bind_group(0, &gpu_res.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderUniforms {
    pub bp: f32,
    pub wp: f32,
    pub pan: [f32; 2],
    pub zoom: f32,
    pub rotation: f32,
    pub aspect_scale: [f32; 2],
    pub bias: f32,
    pub contrast: f32,
    pub posterize_levels: f32,
    pub scaling_mode: u32,
    pub recolor_mode: u32,
    pub invert: u32,
    pub _pad0: f32,
    pub _pad1: f32,
}

pub struct FitsGpuResources {
    pub pipeline: eframe::wgpu::RenderPipeline,
    pub bind_group: eframe::wgpu::BindGroup,
    pub uniform_buffer: eframe::wgpu::Buffer,
    pub bind_group_layout: eframe::wgpu::BindGroupLayout,
    pub texture: eframe::wgpu::Texture,
    pub current_slice: usize,
    pub all_f32_pixels: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

/// Evaluates a colormap at normalized position `val` in [0, 1] with SAO DS9 Bias, Contrast, and Invert applied.
pub fn sample_colormap(
    val: f32,
    bias: f32,
    contrast: f32,
    recolor_mode: u32,
    posterize_levels: f32,
    invert: bool,
) -> egui::Color32 {
    let mut v = ((val - bias) * contrast + 0.5).clamp(0.0, 1.0);
    if invert {
        v = 1.0 - v;
    }

    let (r, g, b) = match recolor_mode {
        1 => {
            // Heat: black -> purple/blue -> red -> yellow -> white
            if v < 0.25 {
                let f = v * 4.0;
                (0.0, 0.0, 0.8 * f)
            } else if v < 0.5 {
                let f = (v - 0.25) * 4.0;
                (0.9 * f, 0.0, 0.8 * (1.0 - f) + 0.1 * f)
            } else if v < 0.75 {
                let f = (v - 0.5) * 4.0;
                (0.9 + 0.1 * f, 0.9 * f, 0.0)
            } else {
                let f = (v - 0.75) * 4.0;
                (1.0, 0.9 + 0.1 * f, 1.0 * f)
            }
        }
        2 => {
            // Cool
            (0.05 + 0.95 * v, 0.05 + 0.80 * v, 0.30 + 0.10 * v)
        }
        3 => {
            // Rainbow: blue -> cyan -> green -> yellow -> red
            if v < 0.25 {
                let f = v * 4.0;
                (0.0, f, 1.0)
            } else if v < 0.5 {
                let f = (v - 0.25) * 4.0;
                (0.0, 1.0, 1.0 - f)
            } else if v < 0.75 {
                let f = (v - 0.5) * 4.0;
                (f, 1.0, 0.0)
            } else {
                let f = (v - 0.75) * 4.0;
                (1.0, 1.0 - f, 0.0)
            }
        }
        4 => {
            // Iron / AIPS0
            if v < 0.2 {
                let f = v * 5.0;
                (0.1 * f, 0.0, 0.5 * f)
            } else if v < 0.4 {
                let f = (v - 0.2) * 5.0;
                (0.1 + 0.5 * f, 0.0, 0.5 + 0.1 * f)
            } else if v < 0.6 {
                let f = (v - 0.4) * 5.0;
                (0.6 + 0.3 * f, 0.3 * f, 0.6 * (1.0 - f))
            } else if v < 0.8 {
                let f = (v - 0.6) * 5.0;
                (0.9 + 0.1 * f, 0.3 + 0.6 * f, 0.2 * f)
            } else {
                let f = (v - 0.8) * 5.0;
                (1.0, 0.9 + 0.1 * f, 0.2 + 0.8 * f)
            }
        }
        5 => {
            // Posterize
            let lev = posterize_levels.max(2.0);
            let step = (v * lev).floor() / (lev - 1.0).max(1.0);
            (step, step, step)
        }
        6 => {
            // Tint
            (v * 0.4, v * 0.8, v * 1.0)
        }
        _ => {
            // Grayscale
            (v, v, v)
        }
    };

    egui::Color32::from_rgb(
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

// image/wgpu_shader_source.rs
pub fn build_shader_source(shader_fragment: &str) -> String {
    format!(
        r#"
struct Uniforms {{
    bp: f32, wp: f32, pan: vec2<f32>, zoom: f32, rotation: f32,
    aspect_scale: vec2<f32>, bias: f32, contrast: f32, posterize_levels: f32,
    scaling_mode: u32, recolor_mode: u32, invert: u32, _pad0: f32, _pad1: f32,
}};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var t_diffuse: texture_2d<f32>;
@group(0) @binding(2) var s_diffuse: sampler;

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {{
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(0.0,0.0), vec2<f32>(1.0,0.0), vec2<f32>(0.0,1.0),
        vec2<f32>(1.0,1.0), vec2<f32>(0.0,1.0), vec2<f32>(1.0,0.0)
    );
    let p = pos[in_vertex_index];
    var out: VertexOutput;
    out.clip_position = vec4<f32>(p.x * 2.0 - 1.0, (1.0 - p.y) * 2.0 - 1.0, 0.0, 1.0);
    out.tex_coords = p;
    return out;
}}

// ---- user-supplied ----
{shader_fragment}
// ------------------------

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let uv = ((in.tex_coords - vec2<f32>(0.5, 0.5)) * uniforms.aspect_scale / uniforms.zoom) - uniforms.pan + vec2<f32>(0.5, 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {{
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }}
    let dim_f32 = vec2<f32>(textureDimensions(t_diffuse));
    var tc = vec2<i32>(uv * dim_f32);
    tc = clamp(tc, vec2<i32>(0,0), vec2<i32>(dim_f32) - vec2<i32>(1,1));
    let raw = textureLoad(t_diffuse, tc, 0).r;
    let range = uniforms.wp - uniforms.bp;
    if (range <= 0.0) {{ return vec4<f32>(0.0,0.0,0.0,1.0); }}
    let val = (clamp(raw, uniforms.bp, uniforms.wp) - uniforms.bp) / range;
    return vec4<f32>(user_color(val), 1.0);
}}
"#
    )
}

#[deprecated]
pub fn calculate_pixels(plane: &[u8], width: usize, height: usize, pixels_buf: &mut Vec<Color32>) {
    for x in 0..width {
        for y in 0..height {
            let fits_y = (height - 1) - y;
            let idx = (fits_y * width + x) as usize;

            let val = plane[idx];
            trace!("{}", val);

            pixels_buf[idx] = Color32::from_gray(val);
        }
    }
}

pub fn normalise(
    data: &[f64],
    scaling_method: &Scaling,
    black_point: f64,
    white_point: f64,
) -> Option<Vec<u8>> {
    to_u8(
        &scaling_method
            .scale(data, black_point, white_point)
            .expect("FAILURE IN SCALING"),
    )
}

fn to_u8(data: &[f64]) -> Option<Vec<u8>> {
    debug!(
        "Max value in f64 array: {:?}",
        data.iter().copied().reduce(f64::max)
    );
    Some(
        data.iter()
            .map(|&x| x.round().clamp(0.0, 255.0) as u8)
            .collect(),
    )
}

fn to_f32(data: &[f64]) -> Option<Vec<f32>> {
    Some(
        data.iter()
            .map(|&x| x.round().clamp(0.0, f32::MAX as f64) as f32)
            .collect(),
    )
}

pub fn w_normalise(data: &[f64], scaling_method: &Scaling, min: f64, max: f64) -> Option<Vec<f32>> {
    to_f32(
        &scaling_method
            .scale(data, min, max)
            .expect("FAILURE IN SCALING"),
    )
}
