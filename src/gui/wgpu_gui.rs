use crate::header::HDU;
use indexmap::IndexMap;
use crate::image::image::FitsData;
use crate::image::render::{
    FitsGpuResources, ShaderUniforms, build_shader_source, sample_colormap,
};
use crate::image::scalars::Scaling;
use crate::image::wgpu_shader_source::WGPU_SHADER_SOURCE;
use crate::render::FitsRenderCallback;
use eframe::{egui, wgpu};
use std::sync::Arc;

pub struct FitsViewerApp {
    hdus: IndexMap<usize, HDU>,
    current_hdu_index: usize,
    pending_hdu_change: bool,

    width: usize,
    height: usize,
    image_data: Option<Arc<FitsData>>,

    min: f64,
    max: f64,
    bscale: f64,
    bzero: f64,
    black_point: f64,
    white_point: f64,
    scaling_method: Scaling,

    slice_index: usize,
    max_slices: usize,

    pan: egui::Vec2,
    zoom: f32,
    rotation: f32,
    bias: f32,
    contrast: f32,
    lock_bias: bool,
    lock_contrast: bool,
    invert: bool,
    recolor_mode: u32,
    posterize_levels: f32,
    last_canvas_rect: Option<egui::Rect>,
}

impl FitsViewerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        hdus: IndexMap<usize, HDU>,
        hdu_index: usize,
        slice_index: usize,
    ) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut app = Self {
            hdus,
            current_hdu_index: hdu_index,
            pending_hdu_change: true,
            width: 1,
            height: 1,
            image_data: None,
            min: 0.0,
            max: 1.0,
            bscale: 1.0,
            bzero: 0.0,
            black_point: 0.0,
            white_point: 1.0,
            scaling_method: Scaling::ASINH,
            slice_index,
            max_slices: 1,
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            bias: 0.5,
            contrast: 1.0,
            lock_bias: false,
            lock_contrast: false,
            invert: false,
            recolor_mode: 0,
            posterize_levels: 8.0,
            last_canvas_rect: None,
        };

        let wgpu_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("WGPU must be enabled!");
        let device = &wgpu_state.device;
        let target_format = &wgpu_state.target_format;

        let uniforms = ShaderUniforms {
            bp: 0.0, wp: 1.0, pan: [0.0, 0.0], zoom: 1.0, rotation: 0.0,
            aspect_scale: [1.0, 1.0], bias: 0.5, contrast: 1.0, posterize_levels: 8.0,
            scaling_mode: 3, recolor_mode: 0, invert: 0, _pad0: 0.0, _pad1: 0.0,
        };

        use eframe::wgpu::util::DeviceExt;
        let uniform_buffer = device.create_buffer_init(&eframe::wgpu::util::BufferInitDescriptor {
            label: Some("FITS Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: eframe::wgpu::BufferUsages::UNIFORM | eframe::wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&eframe::wgpu::BindGroupLayoutDescriptor {
                label: Some("FITS Bind Group Layout"),
                entries: &[
                    eframe::wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                        ty: eframe::wgpu::BindingType::Buffer { ty: eframe::wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                        count: None,
                    },
                    eframe::wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                        ty: eframe::wgpu::BindingType::Texture { sample_type: eframe::wgpu::TextureSampleType::Float { filterable: false }, view_dimension: eframe::wgpu::TextureViewDimension::D2, multisampled: false },
                        count: None,
                    },
                    eframe::wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                        ty: eframe::wgpu::BindingType::Sampler(eframe::wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let size = eframe::wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 };
        let texture = device.create_texture(&eframe::wgpu::TextureDescriptor {
            label: Some("FITS Raw Data Texture"),
            size,
            mip_level_count: 1, sample_count: 1, dimension: eframe::wgpu::TextureDimension::D2,
            format: eframe::wgpu::TextureFormat::R32Float,
            usage: eframe::wgpu::TextureUsages::TEXTURE_BINDING | eframe::wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let texture_view = texture.create_view(&eframe::wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&eframe::wgpu::SamplerDescriptor {
            address_mode_u: eframe::wgpu::AddressMode::ClampToEdge,
            address_mode_v: eframe::wgpu::AddressMode::ClampToEdge,
            address_mode_w: eframe::wgpu::AddressMode::ClampToEdge,
            mag_filter: eframe::wgpu::FilterMode::Nearest,
            min_filter: eframe::wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&eframe::wgpu::BindGroupDescriptor {
            label: Some("FITS Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                eframe::wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                eframe::wgpu::BindGroupEntry { binding: 1, resource: eframe::wgpu::BindingResource::TextureView(&texture_view) },
                eframe::wgpu::BindGroupEntry { binding: 2, resource: eframe::wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        let pipeline_layout =
            device.create_pipeline_layout(&eframe::wgpu::PipelineLayoutDescriptor {
                label: Some("FITS Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        let shader = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
            label: Some("FITS Shader"),
            source: eframe::wgpu::ShaderSource::Wgsl(WGPU_SHADER_SOURCE.into()),
        });

        let pipeline = device.create_render_pipeline(&eframe::wgpu::RenderPipelineDescriptor {
            label: Some("FITS Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: eframe::wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(eframe::wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"),
                targets: &[Some(eframe::wgpu::ColorTargetState { format: *target_format, blend: Some(eframe::wgpu::BlendState::REPLACE), write_mask: eframe::wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: eframe::wgpu::PrimitiveState::default(),
            depth_stencil: None, multisample: eframe::wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });

        wgpu_state.renderer.write().callback_resources.insert(FitsGpuResources {
            pipeline, bind_group, uniform_buffer, bind_group_layout, sampler, texture, current_slice: 0, image_data: None, bscale: 1.0, bzero: 0.0, width: 1, height: 1,
        });

        app
    }

    pub fn load_hdu(&mut self, wgpu_state: &egui_wgpu::RenderState) {
        let hdu = &self.hdus[&self.current_hdu_index];
        let basic_info = &hdu.basic_info;

        let width = basic_info.axes.get(0).copied().unwrap_or(1);
        let height = basic_info.axes.get(1).copied().unwrap_or(1);
        let plane_count = if basic_info.naxis <= 2 { 1 } else { basic_info.axes[2..].iter().product() };

        let image_data_ref = hdu.data.as_ref().map(|m| m.as_ref()).unwrap_or(&[]);
        
        let (image_data_opt, min, max) = if basic_info.n_pixels > 0 && !image_data_ref.is_empty() {
            if let Ok(data) = FitsData::new(image_data_ref, basic_info.bitpix as i32) {
                let arc_data = Arc::new(data);
                let (min, max) = arc_data.get_min_max(basic_info.bscale, basic_info.bzero);
                (Some(arc_data), min, max)
            } else { (None, 0.0, 1.0) }
        } else { (None, 0.0, 1.0) };

        self.width = width; self.height = height; self.max_slices = plane_count; self.slice_index = 0;
        self.bscale = basic_info.bscale; self.bzero = basic_info.bzero;
        self.min = min; self.max = max; self.black_point = min; self.white_point = max;
        self.image_data = image_data_opt.clone();
        self.pan = egui::Vec2::ZERO; self.zoom = 1.0;

        let device = &wgpu_state.device;
        let queue = &wgpu_state.queue;

        let texture_width = width.max(1) as u32;
        let texture_height = height.max(1) as u32;
        let size = eframe::wgpu::Extent3d { width: texture_width, height: texture_height, depth_or_array_layers: 1 };

        let texture = device.create_texture(&eframe::wgpu::TextureDescriptor {
            label: Some("FITS Raw Data Texture"), size, mip_level_count: 1, sample_count: 1, dimension: eframe::wgpu::TextureDimension::D2,
            format: eframe::wgpu::TextureFormat::R32Float, usage: eframe::wgpu::TextureUsages::TEXTURE_BINDING | eframe::wgpu::TextureUsages::COPY_DST, view_formats: &[],
        });

        let plane_size = width * height;

        if let Some(data) = &image_data_opt {
            let initial_slice = data.get_f32_slice(0, plane_size, basic_info.bscale, basic_info.bzero);
            queue.write_texture(
                eframe::wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: eframe::wgpu::Origin3d::ZERO, aspect: eframe::wgpu::TextureAspect::All },
                bytemuck::cast_slice(&initial_slice),
                eframe::wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * texture_width), rows_per_image: Some(texture_height) },
                size,
            );
        } else {
            let zero: [f32; 1] = [0.0];
            queue.write_texture(
                eframe::wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: eframe::wgpu::Origin3d::ZERO, aspect: eframe::wgpu::TextureAspect::All },
                bytemuck::cast_slice(&zero),
                eframe::wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
                size,
            );
        }

        let texture_view = texture.create_view(&eframe::wgpu::TextureViewDescriptor::default());

        let mut renderer = wgpu_state.renderer.write();
        let res = renderer.callback_resources.get_mut::<FitsGpuResources>().unwrap();

        let bind_group = device.create_bind_group(&eframe::wgpu::BindGroupDescriptor {
            label: Some("FITS Bind Group"), layout: &res.bind_group_layout,
            entries: &[
                eframe::wgpu::BindGroupEntry { binding: 0, resource: res.uniform_buffer.as_entire_binding() },
                eframe::wgpu::BindGroupEntry { binding: 1, resource: eframe::wgpu::BindingResource::TextureView(&texture_view) },
                eframe::wgpu::BindGroupEntry { binding: 2, resource: eframe::wgpu::BindingResource::Sampler(&res.sampler) },
            ],
        });

        res.texture = texture; res.bind_group = bind_group; res.image_data = image_data_opt; res.bscale = basic_info.bscale; res.bzero = basic_info.bzero; res.width = texture_width; res.height = texture_height; res.current_slice = 0;
        self.pending_hdu_change = false;
    }

    pub fn recompile_shader_fragment(
        &mut self,
        frame: &eframe::Frame,
        shader_fragment: &str,
    ) -> Result<(), String> {
        let shader_source: String = build_shader_source(shader_fragment);
        self.recompile_shader(frame, shader_source)
    }

    pub fn recompile_shader(
        &mut self,
        frame: &eframe::Frame,
        shader_source: String,
    ) -> Result<(), String> {
        let wgpu_state = frame.wgpu_render_state().expect("WGPU must be enabled");
        let device = &wgpu_state.device;

        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FITS Custom Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let mut renderer = wgpu_state.renderer.write();
        let bind_group_layout = renderer
            .callback_resources
            .get::<FitsGpuResources>()
            .unwrap()
            .bind_group_layout
            .clone();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FITS Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let new_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FITS Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu_state.target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // create_shader_module never fails synchronously -- WGSL errors surface through
        // the error scope instead, so we have to check it explicitly.
        if let Some(err) = pollster::block_on(error_scope.pop()) {
            return Err(err.to_string());
        }

        renderer
            .callback_resources
            .get_mut::<FitsGpuResources>()
            .unwrap()
            .pipeline = new_pipeline;
        Ok(())
    }

    pub fn name() -> &'static str {
        "FITS viewer"
    }

    fn format_scaling_method(&self) -> &'static str {
        match self.scaling_method {
            Scaling::LINEAR => "Linear (lin)",
            Scaling::LOGARITHMIC => "Logarithmic (log)",
            Scaling::SQUAREROOT => "Square Root (sqrt)",
            Scaling::ASINH => "Asinh (asinh)",
        }
    }

    fn format_recolor_mode(&self) -> &'static str {
        match self.recolor_mode {
            0 => "Grayscale",
            1 => "Heat",
            2 => "Cool",
            3 => "Rainbow",
            4 => "Iron",
            5 => "Posterize",
            6 => "Tint",
            _ => "Custom",
        }
    }

    fn current_slice_min_max(&self) -> (f64, f64) {
        if let Some(image_data) = &self.image_data {
            let plane_size = self.width * self.height;
            let offset = self.slice_index * plane_size;
            if offset + plane_size <= image_data.len() {
                image_data.get_slice_min_max(offset, plane_size, self.bscale, self.bzero)
            } else {
                (self.min, self.max)
            }
        } else {
            (0.0, 1.0)
        }
    }

    pub fn aspect_scale(&self, canvas_size: egui::Vec2) -> egui::Vec2 {
        if self.width > 0 && self.height > 0 && canvas_size.x > 0.0 && canvas_size.y > 0.0 {
            let img_aspect = self.width as f32 / self.height as f32;
            let canvas_aspect = canvas_size.x / canvas_size.y;
            if canvas_aspect > img_aspect {
                egui::vec2(canvas_aspect / img_aspect, 1.0)
            } else {
                egui::vec2(1.0, img_aspect / canvas_aspect)
            }
        } else {
            egui::vec2(1.0, 1.0)
        }
    }

    /// Converts a screen position to FITS coordinates (relative to bottom-left of the image)
    /// and looks up the physical value at that pixel.
    pub fn screen_to_fits_coord(
        &self,
        screen_pos: egui::Pos2,
        canvas_rect: egui::Rect,
    ) -> Option<(f32, f32, f64)> {
        if self.width == 0
            || self.height == 0
            || canvas_rect.width() <= 0.0
            || canvas_rect.height() <= 0.0
        {
            return None;
        }

        if !canvas_rect.contains(screen_pos) {
            return None;
        }

        let aspect_scale = self.aspect_scale(canvas_rect.size());

        // Normalized coordinate in canvas rect [0.0, 1.0] (0,0 is top-left of canvas)
        let p = egui::vec2(
            (screen_pos.x - canvas_rect.min.x) / canvas_rect.width(),
            (screen_pos.y - canvas_rect.min.y) / canvas_rect.height(),
        );

        // Invert shader transformation:
        // In shader:
        //   var uv = ((in.tex_coords - vec2<f32>(0.5, 0.5)) * uniforms.aspect_scale / uniforms.zoom) - uniforms.pan + vec2<f32>(0.5, 0.5);
        //   uv = rotate_uv(uv, uniforms.rotation, vec2<f32>(0.5, 0.5));
        let centered = p - egui::vec2(0.5, 0.5);
        let uv_unrotated = (centered * aspect_scale / self.zoom) - self.pan + egui::vec2(0.5, 0.5);

        let s = self.rotation.sin();
        let c = self.rotation.cos();
        let uv_centered = uv_unrotated - egui::vec2(0.5, 0.5);
        let uv = egui::vec2(
            uv_centered.x * c - uv_centered.y * s,
            uv_centered.x * s + uv_centered.y * c,
        ) + egui::vec2(0.5, 0.5);

        // Check if pointer is within the FITS image boundary
        if uv.x < 0.0 || uv.x >= 1.0 || uv.y < 0.0 || uv.y >= 1.0 {
            return None;
        }

        // FITS coordinates relative to bottom-left:
        // Origin (0, 0) is at the bottom-left corner of the FITS image.
        // x increases from left (0) to right (width).
        // y increases from bottom (0) to top (height).
        let fits_x = uv.x * self.width as f32;
        let fits_y = (1.0 - uv.y) * self.height as f32;

        // Discrete pixel coordinates in the underlying image texture
        let px = (uv.x * self.width as f32).floor() as usize;
        let py = (uv.y * self.height as f32).floor() as usize;
        let px = px.min(self.width.saturating_sub(1));
        let py = py.min(self.height.saturating_sub(1));

        let plane_size = self.width * self.height;
        let slice_offset = self.slice_index * plane_size;
        let idx = slice_offset + py * self.width + px;

        let val = if let Some(image_data) = &self.image_data {
            if idx < image_data.len() {
                image_data.get_f64_pixel(idx, self.bscale, self.bzero)
            } else {
                f64::NAN
            }
        } else {
            f64::NAN
        };

        Some((fits_x, fits_y, val))
    }
}

impl eframe::App for FitsViewerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.pending_hdu_change {
            if let Some(wgpu_state) = _frame.wgpu_render_state() {
                self.load_hdu(wgpu_state);
            }
        }
        
        let ctx = ui.ctx().clone();

        // ========================
        // RIGHT PANEL
        // ========================

        egui::Panel::right("sliders").show(ui, |ui| {
            ui.add_space(4.0);

            // ==========================================
            // 0. HDU SELECTION
            // ==========================================
            egui::CollapsingHeader::new(egui::RichText::new("📁  HDU Selection").strong().size(13.0))
                .default_open(true)
                .show(ui, |ui| {
                    ui.add_space(2.0);
                    
                    let get_hdu_label = |index: usize, hdu: &crate::header::HDU| -> String {
                        let ext_type = hdu.header.get_value("XTENSION").unwrap_or("PRIMARY");
                        let clean_ext = ext_type.trim_matches('\'').trim();
                        format!("HDU {}: {} ({} axes)", index, clean_ext, hdu.basic_info.naxis)
                    };

                    ui.horizontal(|ui| {
                        ui.label("HDU:");
                        egui::ComboBox::new("hdu_combo_box", "")
                            .selected_text(get_hdu_label(self.current_hdu_index, &self.hdus[&self.current_hdu_index]))
                            .show_ui(ui, |ui| {
                                for key in self.hdus.keys() {
                                    let text = get_hdu_label(*key, &self.hdus[key]);
                                    if ui.selectable_label(self.current_hdu_index == *key, text).clicked() {
                                        if self.current_hdu_index != *key {
                                            self.current_hdu_index = *key;
                                            self.pending_hdu_change = true;
                                        }
                                    }
                                }
                            });
                    });
                    
                    if self.image_data.is_none() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("!!No Image Data in this HDU").color(egui::Color32::YELLOW));
                    }
                    ui.add_space(4.0);
                });
            
            ui.separator();

            // ==========================================
            // 1. SCALING & CUT LEVELS
            // ==========================================
            egui::CollapsingHeader::new(
                egui::RichText::new("Scaling & Cut Levels")
                    .strong()
                    .size(13.0),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    egui::ComboBox::new("combo_box_scaling_method", "")
                        .selected_text(self.format_scaling_method())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.scaling_method,
                                Scaling::LINEAR,
                                "Linear (lin)",
                            );
                            ui.selectable_value(
                                &mut self.scaling_method,
                                Scaling::LOGARITHMIC,
                                "Logarithmic (log)",
                            );
                            ui.selectable_value(
                                &mut self.scaling_method,
                                Scaling::SQUAREROOT,
                                "Square Root (sqrt)",
                            );
                            ui.selectable_value(
                                &mut self.scaling_method,
                                Scaling::ASINH,
                                "Asinh (asinh)",
                            );
                        });
                });

                ui.add_space(2.0);
                ui.add(
                    egui::Slider::new(&mut self.black_point, self.min..=self.white_point)
                        .text("Black Point"),
                );
                ui.add(
                    egui::Slider::new(&mut self.white_point, self.black_point..=self.max)
                        .text("White Point"),
                );

                ui.horizontal(|ui| {
                    if ui
                        .button("Full Range")
                        .on_hover_text("Reset cuts to full cube min and max")
                        .clicked()
                    {
                        self.black_point = self.min;
                        self.white_point = self.max;
                    }
                    if self.max_slices > 1 {
                        if ui
                            .button("Slice Range")
                            .on_hover_text("Reset cuts to active slice min and max")
                            .clicked()
                        {
                            let (s_min, s_max) = self.current_slice_min_max();
                            self.black_point = s_min;
                            self.white_point = s_max;
                        }
                    }
                });

                if self.max_slices > 1 {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Slice:");
                        if ui.button("◀").clicked() && self.slice_index > 0 {
                            self.slice_index -= 1;
                        }
                        ui.add(
                            egui::Slider::new(
                                &mut self.slice_index,
                                0..=(self.max_slices.saturating_sub(1)),
                            )
                            .text(format!("/ {}", self.max_slices.saturating_sub(1))),
                        );
                        if ui.button("▶").clicked() && self.slice_index + 1 < self.max_slices {
                            self.slice_index += 1;
                        }
                    });
                }
                ui.add_space(4.0);
            });

            ui.separator();

            // ==========================================
            // 2. COLORMAP & TRANSFER FUNCTION
            // ==========================================
            egui::CollapsingHeader::new(
                egui::RichText::new("Colormap & Transfer")
                    .strong()
                    .size(13.0),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label("Colormap:");
                    egui::ComboBox::new("side_combo_box_recolor_mode", "")
                        .selected_text(self.format_recolor_mode())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.recolor_mode, 0, "Grayscale");
                            ui.selectable_value(&mut self.recolor_mode, 1, "Heat");
                            ui.selectable_value(&mut self.recolor_mode, 2, "Cool");
                            ui.selectable_value(&mut self.recolor_mode, 3, "Rainbow");
                            ui.selectable_value(&mut self.recolor_mode, 4, "Iron");
                            ui.selectable_value(&mut self.recolor_mode, 5, "Posterize");
                            ui.selectable_value(&mut self.recolor_mode, 6, "Tint");
                        });
                });

                ui.checkbox(&mut self.invert, "Invert Colormap");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.lock_bias, "Lock Bias");
                    ui.checkbox(&mut self.lock_contrast, "Lock Contrast");
                });

                ui.add_space(2.0);
                ui.add_enabled(
                    !self.lock_bias,
                    egui::Slider::new(&mut self.bias, 0.0..=1.0).text("Bias"),
                );
                ui.add_enabled(
                    !self.lock_contrast,
                    egui::Slider::new(&mut self.contrast, 0.0..=10.0).text("Contrast"),
                );

                if self.recolor_mode == 5 {
                    ui.add(
                        egui::Slider::new(&mut self.posterize_levels, 2.0..=32.0)
                            .text("Posterize Levels"),
                    );
                }

                ui.add_space(2.0);
                if ui
                    .button("↺ Reset Bias & Contrast")
                    .on_hover_text("Reset Bias to 0.5 and Contrast to 1.0 (if unlocked)")
                    .clicked()
                {
                    if !self.lock_bias {
                        self.bias = 0.5;
                    }
                    if !self.lock_contrast {
                        self.contrast = 1.0;
                    }
                }
                ui.add_space(4.0);
            });

            ui.separator();

            // ==========================================
            // 3. VIEW & ORIENTATION
            // ==========================================
            egui::CollapsingHeader::new(
                egui::RichText::new("View & Orientation")
                    .strong()
                    .size(13.0),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let mut deg = self.rotation.to_degrees().rem_euclid(360.0);
                    if ui
                        .add(
                            egui::Slider::new(&mut deg, 0.0..=360.0)
                                .suffix("°")
                                .text("Rotation"),
                        )
                        .changed()
                    {
                        self.rotation = deg.to_radians();
                    }
                    if ui
                        .add(egui::Button::new("↻"))
                        .on_hover_text("Rotate 90° clockwise")
                        .clicked()
                    {
                        self.rotation =
                            (self.rotation - std::f32::consts::FRAC_PI_2) % std::f32::consts::TAU; // - for clockwise, this matches shader rotation
                    }
                    if ui
                        .add(egui::Button::new("↺"))
                        .on_hover_text("Rotate 90° anti-clockwise")
                        .clicked()
                    {
                        self.rotation =
                            (self.rotation + std::f32::consts::FRAC_PI_2) % std::f32::consts::TAU;
                    }
                });

                ui.add_space(2.0);
                ui.add(
                    egui::Slider::new(&mut self.zoom, 0.05..=50.0)
                        .logarithmic(true)
                        .suffix("x")
                        .text("Zoom"),
                );

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    if ui.button("1.0x Zoom").clicked() {
                        self.zoom = 1.0;
                    }
                    if ui
                        .button("Center View")
                        .on_hover_text("Reset pan to center (0, 0)")
                        .clicked()
                    {
                        self.pan = egui::Vec2::ZERO;
                    }
                    if ui
                        .button("Reset View")
                        .on_hover_text("Reset Pan, Zoom, and Rotation")
                        .clicked()
                    {
                        self.pan = egui::Vec2::ZERO;
                        self.zoom = 1.0;
                        self.rotation = 0.0;
                    }
                });
                ui.add_space(4.0);
            });

            ui.separator();

            // ==========================================
            // 4. IMAGE INFORMATION
            // ==========================================
            egui::CollapsingHeader::new(
                egui::RichText::new("Image Information").strong().size(13.0),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.add_space(2.0);
                egui::Grid::new("image_info_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Dimensions:").strong());
                        ui.label(format!("{} x {} px", self.width, self.height));
                        ui.end_row();

                        ui.label(egui::RichText::new("Data Min:").strong());
                        ui.label(format!("{:.4e}", self.min));
                        ui.end_row();

                        ui.label(egui::RichText::new("Data Max:").strong());
                        ui.label(format!("{:.4e}", self.max));
                        ui.end_row();

                        if self.max_slices > 1 {
                            ui.label(egui::RichText::new("Slices:").strong());
                            ui.label(format!(
                                "{} (active: #{})",
                                self.max_slices, self.slice_index
                            ));
                            ui.end_row();

                            let (s_min, s_max) = self.current_slice_min_max();
                            ui.label(egui::RichText::new("Slice Range:").strong());
                            ui.label(format!("{:.3e} .. {:.3e}", s_min, s_max));
                            ui.end_row();
                        }

                        ui.label(egui::RichText::new("Zoom:").strong());
                        ui.label(format!("{:.2}x", self.zoom));
                        ui.end_row();

                        ui.label(egui::RichText::new("Pan:").strong());
                        ui.label(format!("({:.2}, {:.2})", self.pan.x, self.pan.y));
                        ui.end_row();
                    });
                ui.add_space(4.0);
            });
            // ==========================================
            // 5. POINTER INFORMATION
            // ==========================================

            let hover_info = ctx.input(|i| i.pointer.hover_pos()).and_then(|screen_pos| {
                self.last_canvas_rect
                    .and_then(|rect| self.screen_to_fits_coord(screen_pos, rect))
            });

            egui::CollapsingHeader::new(
                egui::RichText::new("Pointer Information")
                    .strong()
                    .size(13.0),
            )
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
                egui::Grid::new("pointer_info_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Value:").strong());
                        let val_text = match hover_info {
                            Some((_, _, val)) => {
                                if val.is_nan() {
                                    "NaN".to_string()
                                } else if val.is_infinite() {
                                    if val.is_sign_positive() {
                                        "Infinity".to_string()
                                    } else {
                                        "-Infinity".to_string()
                                    }
                                } else if val.abs() >= 1e5 || (val.abs() < 1e-3 && val != 0.0) {
                                    format!("{:.4e}", val)
                                } else {
                                    format!("{:.4}", val)
                                }
                            }
                            None => "—".to_string(),
                        };
                        ui.label(val_text);
                        ui.end_row();

                        ui.label(egui::RichText::new("Image:").strong());
                        let pos_text = match hover_info {
                            Some((x, y, _)) => format!("x {:.3} y {:.3}", x, y),
                            None => "x  y  ".to_string(),
                        };
                        ui.label(pos_text);
                        ui.end_row();
                    });
                ui.add_space(4.0);
            });
        });

        // =====================
        // END RIGHT PANEL
        // =====================

        // =====================
        // BOTTOM COLORBAR PANEL
        // =====================

        egui::Panel::bottom("colorbar_panel").show(ui, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.strong("Colorbar");
                ui.separator();

                egui::ComboBox::new("bottom_combo_box_recolor_mode", "Colormap")
                    .selected_text(self.format_recolor_mode())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.recolor_mode, 0, "Grayscale");
                        ui.selectable_value(&mut self.recolor_mode, 1, "Heat");
                        ui.selectable_value(&mut self.recolor_mode, 2, "Cool");
                        ui.selectable_value(&mut self.recolor_mode, 3, "Rainbow");
                        ui.selectable_value(&mut self.recolor_mode, 4, "Iron");
                        ui.selectable_value(&mut self.recolor_mode, 5, "Posterize");
                        ui.selectable_value(&mut self.recolor_mode, 6, "Tint");
                    });

                ui.checkbox(&mut self.invert, "Invert");
                ui.checkbox(&mut self.lock_bias, "Lock Bias");
                ui.checkbox(&mut self.lock_contrast, "Lock Contrast");

                ui.separator();
                ui.label(format!("Bias: {:.2}", self.bias));
                ui.label(format!("Contrast: {:.2}", self.contrast));

                if ui.button("Reset (0.5, 1.0)").on_hover_text("Reset Bias to 0.5 and Contrast to 1.0 (or double-click the colorbar)").clicked() {
                    if !self.lock_bias {
                        self.bias = 0.5;
                    }
                    if !self.lock_contrast {
                        self.contrast = 1.0;
                    }
                }
            });

            ui.add_space(3.0);

            // Draggable Colorbar
            let bar_height = 24.0;
            let available_w = ui.available_width().max(100.0);
            let (bar_rect, bar_response) = ui.allocate_exact_size(
                egui::vec2(available_w, bar_height),
                egui::Sense::click_and_drag(),
            );

            // Mouse interactions:
            if bar_response.dragged_by(egui::PointerButton::Primary) {
                let delta = bar_response.drag_delta();
                // Dragging horizontally shifts Bias (if not locked)
                if !self.lock_bias {
                    self.bias = (self.bias + delta.x / bar_rect.width()).clamp(0.0, 1.0);
                }
                // Dragging vertically shifts Contrast (if not locked) with reduced sensitivity
                if !self.lock_contrast {
                    self.contrast = (self.contrast - delta.y * 0.005).clamp(0.0, 10.0);
                }
            } else if bar_response.clicked_by(egui::PointerButton::Primary) {
                if !self.lock_bias {
                    if let Some(pos) = bar_response.interact_pointer_pos() {
                        self.bias = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                    }
                }
            }

            if bar_response.double_clicked() || bar_response.clicked_by(egui::PointerButton::Secondary) {
                if !self.lock_bias {
                    self.bias = 0.5;
                }
                if !self.lock_contrast {
                    self.contrast = 1.0;
                }
            }

            bar_response.on_hover_text(
                "• Drag left/right to adjust Bias (if unlocked)\n• Drag up/down to adjust Contrast (if unlocked)\n• Click to set Bias position\n• Double-click or Right-click to Reset"
            );

            if ui.is_rect_visible(bar_rect) {
                let painter = ui.painter();
                let num_segments = 256;
                let mut mesh = egui::Mesh::default();

                for i in 0..=num_segments {
                    let t = i as f32 / num_segments as f32;
                    let color = sample_colormap(
                        t,
                        self.bias,
                        self.contrast,
                        self.recolor_mode,
                        self.posterize_levels,
                        self.invert,
                    );
                    let x = bar_rect.min.x + t * bar_rect.width();

                    let top_idx = mesh.vertices.len() as u32;
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: egui::pos2(x, bar_rect.min.y),
                        uv: egui::epaint::WHITE_UV,
                        color,
                    });
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: egui::pos2(x, bar_rect.max.y),
                        uv: egui::epaint::WHITE_UV,
                        color,
                    });

                    if i < num_segments {
                        mesh.indices.push(top_idx);
                        mesh.indices.push(top_idx + 1);
                        mesh.indices.push(top_idx + 2);

                        mesh.indices.push(top_idx + 1);
                        mesh.indices.push(top_idx + 3);
                        mesh.indices.push(top_idx + 2);
                    }
                }

                painter.add(egui::Shape::mesh(mesh));

                // Border around colorbar
                painter.rect_stroke(
                    bar_rect,
                    1.0,
                    egui::Stroke::new(1.0, egui::Color32::from_gray(140)),
                    egui::StrokeKind::Inside,
                );

                // Bias indicator line and pointer ticks
                let bias_x = (bar_rect.min.x + self.bias * bar_rect.width()).clamp(bar_rect.min.x, bar_rect.max.x);
                painter.line_segment(
                    [
                        egui::pos2(bias_x, bar_rect.min.y),
                        egui::pos2(bias_x, bar_rect.max.y),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );
                painter.line_segment(
                    [
                        egui::pos2(bias_x - 3.0, bar_rect.min.y),
                        egui::pos2(bias_x + 3.0, bar_rect.min.y),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );
                painter.line_segment(
                    [
                        egui::pos2(bias_x - 3.0, bar_rect.max.y),
                        egui::pos2(bias_x + 3.0, bar_rect.max.y),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );
            }
            ui.add_space(2.0);
        });

        // =========================
        // END BOTTOM COLORBAR PANEL
        // =========================

        // =====================
        // CENTRAL IMAGE PANEL
        // =====================

        egui::CentralPanel::default().show(ui, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            self.last_canvas_rect = Some(rect);

            let aspect_scale = self.aspect_scale(rect.size());

            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                let d_norm = egui::vec2(delta.x / rect.width(), delta.y / rect.height());
                self.pan += (d_norm * aspect_scale) / self.zoom;
            } else if response.dragged_by(egui::PointerButton::Secondary) {
                // SAO DS9 Right-click drag on canvas adjusts Bias and Contrast
                let delta = response.drag_delta();
                if !self.lock_bias {
                    self.bias = (self.bias + delta.x / rect.width()).clamp(0.0, 1.0);
                }
                if !self.lock_contrast {
                    self.contrast =
                        (self.contrast - delta.y / rect.height() * 1.5).clamp(0.0, 10.0);
                }
            }

            if response.hovered() {
                let zoom_delta = ctx.input(|i| i.zoom_delta());
                if zoom_delta != 1.0 {
                    if let Some(mouse_pos) = response.hover_pos() {
                        let mouse_frac = egui::vec2(
                            (mouse_pos.x - rect.min.x) / rect.width(),
                            (mouse_pos.y - rect.min.y) / rect.height(),
                        );
                        let c_mouse = mouse_frac - egui::vec2(0.5, 0.5);

                        let old_zoom = self.zoom;
                        self.zoom *= zoom_delta;

                        let scale = 1.0 / self.zoom - 1.0 / old_zoom;
                        self.pan += c_mouse * aspect_scale * scale;
                    }
                }
            }

            let mode_int = match self.scaling_method {
                Scaling::LINEAR => 0,
                Scaling::LOGARITHMIC => 1,
                Scaling::SQUAREROOT => 2,
                Scaling::ASINH => 3,
            };

            let callback = FitsRenderCallback {
                bp: self.black_point as f32,
                wp: self.white_point as f32,
                pan: self.pan,
                zoom: self.zoom,
                scaling_mode: mode_int,
                rotation: self.rotation,
                bias: self.bias,
                contrast: self.contrast,
                recolor_mode: self.recolor_mode,
                invert: self.invert,
                posterize_levels: self.posterize_levels,
                aspect_scale,
                slice_index: self.slice_index,
            };

            ui.painter()
                .add(egui_wgpu::Callback::new_paint_callback(rect, callback));
        });

        // =======================
        // END CENTRAL IMAGE PANEL
        // =======================
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app(width: usize, height: usize, slices: usize) -> FitsViewerApp {
        FitsViewerApp {
            hdus: indexmap::IndexMap::new(),
            current_hdu_index: 0,
            pending_hdu_change: false,
            width,
            height,
            image_data: Some(Arc::new(FitsData::F64(vec![0.0]))),
            min: 0.0,
            max: 0.0,
            bscale: 1.0,
            bzero: 0.0,
            black_point: 0.0,
            white_point: 0.0,
            scaling_method: Scaling::LINEAR,
            slice_index: 0,
            max_slices: slices,
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            bias: 0.5,
            contrast: 1.0,
            lock_bias: false,
            lock_contrast: false,
            invert: false,
            recolor_mode: 0,
            posterize_levels: 8.0,
            last_canvas_rect: None,
        }
    }

    #[test]
    fn test_screen_to_fits_coord_center() {
        let app = create_test_app(100, 50, 1);
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 50.0));

        let res = app.screen_to_fits_coord(egui::pos2(50.0, 25.0), canvas);
        assert!(res.is_some());
        let (fx, fy, val) = res.unwrap();
        assert!((fx - 50.0).abs() < 1e-3);
        assert!((fy - 25.0).abs() < 1e-3);
        // py = 25, px = 50 -> idx = 25 * 100 + 50 = 2550
        assert_eq!(val, 2550.0);
    }

    #[test]
    fn test_screen_to_fits_coord_bottom_left() {
        let app = create_test_app(100, 50, 1);
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 50.0));

        // Bottom-left in screen coordinates is near (0.0, 50.0)
        let res = app.screen_to_fits_coord(egui::pos2(0.5, 49.5), canvas);
        assert!(res.is_some());
        let (fx, fy, val) = res.unwrap();
        assert!((fx - 0.5).abs() < 1e-3);
        assert!((fy - 0.5).abs() < 1e-3);
        // py = 49, px = 0 -> idx = 49 * 100 + 0 = 4900
        assert_eq!(val, 4900.0);
    }

    #[test]
    fn test_screen_to_fits_coord_top_left() {
        let app = create_test_app(100, 50, 1);
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 50.0));

        // Top-left in screen coordinates is near (0.0, 0.0)
        let res = app.screen_to_fits_coord(egui::pos2(0.5, 0.5), canvas);
        assert!(res.is_some());
        let (fx, fy, val) = res.unwrap();
        assert!((fx - 0.5).abs() < 1e-3);
        assert!((fy - 49.5).abs() < 1e-3);
        // py = 0, px = 0 -> idx = 0
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_screen_to_fits_coord_outside_canvas() {
        let app = create_test_app(100, 50, 1);
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 50.0));

        assert!(
            app.screen_to_fits_coord(egui::pos2(-5.0, 25.0), canvas)
                .is_none()
        );
        assert!(
            app.screen_to_fits_coord(egui::pos2(105.0, 25.0), canvas)
                .is_none()
        );
    }

    #[test]
    fn test_screen_to_fits_coord_aspect_letterbox() {
        // Image aspect = 2.0 (100x50), Canvas aspect = 4.0 (200x50)
        // Image will occupy middle 100 pixels horizontally (from x = 50 to x = 150)
        let app = create_test_app(100, 50, 1);
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 50.0));

        // Center of canvas is x = 100, y = 25
        let center = app.screen_to_fits_coord(egui::pos2(100.0, 25.0), canvas);
        assert!(center.is_some());
        let (fx, fy, val) = center.unwrap();
        assert!((fx - 50.0).abs() < 1e-3);
        assert!((fy - 25.0).abs() < 1e-3);
        assert_eq!(val, 2550.0);

        // x = 20 is in the left letterbox border (outside the image)
        assert!(
            app.screen_to_fits_coord(egui::pos2(20.0, 25.0), canvas)
                .is_none()
        );
        // x = 180 is in the right letterbox border (outside the image)
        assert!(
            app.screen_to_fits_coord(egui::pos2(180.0, 25.0), canvas)
                .is_none()
        );
    }

    #[test]
    fn test_screen_to_fits_coord_slices() {
        let mut app = create_test_app(10, 10, 3);
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0));

        // Slice 0 center (5, 5) -> py = 5, px = 5 -> idx = 55
        app.slice_index = 0;
        let (_, _, val0) = app
            .screen_to_fits_coord(egui::pos2(5.5, 4.5), canvas)
            .unwrap();
        // py = floor(4.5) = 4, px = floor(5.5) = 5 -> idx = 45
        assert_eq!(val0, 45.0);

        // Slice 1: slice_offset = 100 -> idx = 145
        app.slice_index = 1;
        let (_, _, val1) = app
            .screen_to_fits_coord(egui::pos2(5.5, 4.5), canvas)
            .unwrap();
        assert_eq!(val1, 145.0);
    }

    #[test]
    fn test_screen_to_fits_coord_zoom_and_pan() {
        let mut app = create_test_app(100, 100, 1);
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0));

        // 2x zoom centered: screen center (50, 50) still points to FITS center (50, 50)
        app.zoom = 2.0;
        let (_, _, val) = app
            .screen_to_fits_coord(egui::pos2(50.0, 50.0), canvas)
            .unwrap();
        // py = 50, px = 50 -> idx = 50 * 100 + 50 = 5050
        assert_eq!(val, 5050.0);

        // With pan of 0.1 in x: center shifts
        app.pan = egui::vec2(0.1, 0.0);
        let (fx, fy, _) = app
            .screen_to_fits_coord(egui::pos2(50.0, 50.0), canvas)
            .unwrap();
        // uv_x was 0.5 - 0.1 = 0.4 -> fits_x = 40.0
        assert!((fx - 40.0).abs() < 1e-3);
        assert!((fy - 50.0).abs() < 1e-3);
    }

    #[test]
    fn test_screen_to_fits_coord_rotation() {
        let mut app = create_test_app(100, 100, 1);
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0));

        // Center remains invariant under rotation
        app.rotation = std::f32::consts::FRAC_PI_2; // 90 degrees
        let (fx, fy, _) = app
            .screen_to_fits_coord(egui::pos2(50.0, 50.0), canvas)
            .unwrap();
        assert!((fx - 50.0).abs() < 1e-3);
        assert!((fy - 50.0).abs() < 1e-3);
    }
}
