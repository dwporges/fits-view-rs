use crate::header::HDU;
use crate::image::image::FitsData;
use crate::image::render::{
    FitsGpuResources, ShaderUniforms, build_shader_source, sample_colormap,
};
use crate::image::scalars::Scaling;
use crate::image::wgpu_shader_source::WGPU_SHADER_SOURCE;
use crate::render::FitsRenderCallback;
use eframe::{egui, wgpu};
use egui_extras::Column;
use indexmap::IndexMap;
use std::sync::Arc;

pub struct FitsViewerApp {
    pub image: crate::gui::state::ImageData,
    pub render: crate::gui::state::RenderSettings,
    pub viewport: crate::gui::state::ViewportState,
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
            image: crate::gui::state::ImageData {
                hdus,
                current_hdu_index: hdu_index,
                pending_hdu_change: true,
                width: 1,
                height: 1,
                image_data: None,
                slice_index,
                max_slices: 1,
            },
            render: crate::gui::state::RenderSettings::default(),
            viewport: crate::gui::state::ViewportState::default(),
        };

        let wgpu_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("WGPU must be enabled!");
        let device = &wgpu_state.device;
        let target_format = &wgpu_state.target_format;

        let res = crate::gui::render_setup::setup_wgpu_resources(
            device,
            target_format,
            WGPU_SHADER_SOURCE,
        );
        wgpu_state.renderer.write().callback_resources.insert(res);

        app
    }

    pub fn load_hdu(&mut self, wgpu_state: &egui_wgpu::RenderState) {
        let hdu = &self.image.hdus[&self.image.current_hdu_index];
        let basic_info = &hdu.basic_info;

        let width = basic_info.axes.get(0).copied().unwrap_or(1);
        let height = basic_info.axes.get(1).copied().unwrap_or(1);
        let plane_count = if basic_info.naxis <= 2 {
            1
        } else {
            basic_info.axes[2..].iter().product()
        };

        let image_data_ref = hdu.data.as_ref().map(|m| m.as_ref()).unwrap_or(&[]);

        let (image_data_opt, min, max) = if basic_info.n_pixels > 0 && !image_data_ref.is_empty() {
            if let Ok(data) = FitsData::new(image_data_ref, basic_info.bitpix as i32) {
                let arc_data = Arc::new(data);
                let (min, max) = arc_data.get_min_max(basic_info.bscale, basic_info.bzero);
                (Some(arc_data), min, max)
            } else {
                (None, 0.0, 1.0)
            }
        } else {
            (None, 0.0, 1.0)
        };

        self.image.width = width;
        self.image.height = height;
        self.image.max_slices = plane_count;
        self.image.slice_index = 0;
        self.render.bscale = basic_info.bscale;
        self.render.bzero = basic_info.bzero;
        self.render.min = min;
        self.render.max = max;
        self.render.black_point = min;
        self.render.white_point = max;
        self.image.image_data = image_data_opt.clone();
        self.viewport.pan = egui::Vec2::ZERO;
        self.viewport.zoom = 1.0;

        crate::gui::render_setup::update_wgpu_texture(
            wgpu_state,
            width,
            height,
            basic_info.bscale,
            basic_info.bzero,
            image_data_opt,
        );

        self.image.pending_hdu_change = false;
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
        crate::gui::render_setup::recompile_wgpu_shader(wgpu_state, shader_source)
    }

    pub fn name() -> &'static str {
        "FITS viewer"
    }

    pub fn aspect_scale(&self, canvas_size: egui::Vec2) -> egui::Vec2 {
        if self.image.width > 0
            && self.image.height > 0
            && canvas_size.x > 0.0
            && canvas_size.y > 0.0
        {
            let img_aspect = self.image.width as f32 / self.image.height as f32;
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
        if self.image.width == 0
            || self.image.height == 0
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
        let uv_unrotated = (centered * aspect_scale / self.viewport.zoom) - self.viewport.pan
            + egui::vec2(0.5, 0.5);

        let s = self.viewport.rotation.sin();
        let c = self.viewport.rotation.cos();
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
        let fits_x = uv.x * self.image.width as f32;
        let fits_y = (1.0 - uv.y) * self.image.height as f32;

        // Discrete pixel coordinates in the underlying image texture
        let px = (uv.x * self.image.width as f32).floor() as usize;
        let py = (uv.y * self.image.height as f32).floor() as usize;
        let px = px.min(self.image.width.saturating_sub(1));
        let py = py.min(self.image.height.saturating_sub(1));

        let plane_size = self.image.width * self.image.height;
        let slice_offset = self.image.slice_index * plane_size;
        let idx = slice_offset + py * self.image.width + px;

        let val = if let Some(image_data) = &self.image.image_data {
            if idx < image_data.len() {
                image_data.get_f64_pixel(idx, self.render.bscale, self.render.bzero)
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
        if self.image.pending_hdu_change {
            if let Some(wgpu_state) = _frame.wgpu_render_state() {
                self.load_hdu(wgpu_state);
            }
        }

        let ctx = ui.ctx().clone();

        egui::Panel::right("sliders").show(ui, |ui| {
            ui.add_space(4.0);
            crate::gui::components::hdu_selection::show(
                ui,
                &ctx,
                &mut self.image,
                &mut self.viewport,
            );
            ui.separator();
            crate::gui::components::scaling::show(ui, &mut self.render, &mut self.image);
            ui.separator();
            crate::gui::components::colormap::show(ui, &mut self.render);
            ui.separator();
            crate::gui::components::view_orientation::show(ui, &mut self.viewport);
            ui.separator();
            crate::gui::components::image_info::show(ui, &self.image, &self.render, &self.viewport);
            ui.separator();
            crate::gui::components::pointer_info::show(
                ui,
                &ctx,
                &self.image,
                &self.render,
                &mut self.viewport,
            );
        });

        egui::Panel::bottom("colorbar_panel").show(ui, |ui| {
            crate::gui::components::colorbar::show(ui, &mut self.render);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            crate::gui::components::canvas::show(
                ui,
                &ctx,
                &mut self.viewport,
                &mut self.render,
                &self.image,
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app(width: usize, height: usize, slices: usize) -> FitsViewerApp {
        FitsViewerApp {
            image: crate::gui::state::ImageData {
                hdus: indexmap::IndexMap::new(),
                current_hdu_index: 0,
                pending_hdu_change: false,
                width,
                height,
                image_data: Some(Arc::new(FitsData::F64(vec![0.0]))),
                slice_index: 0,
                max_slices: slices,
            },
            render: crate::gui::state::RenderSettings {
                min: 0.0,
                max: 0.0,
                bscale: 1.0,
                bzero: 0.0,
                black_point: 0.0,
                white_point: 0.0,
                scaling_method: Scaling::LINEAR,
                recolor_mode: 0,
                posterize_levels: 8.0,
                invert: false,
                bias: 0.5,
                contrast: 1.0,
                lock_bias: false,
                lock_contrast: false,
            },
            viewport: crate::gui::state::ViewportState {
                pan: egui::Vec2::ZERO,
                zoom: 1.0,
                rotation: 0.0,
                last_canvas_rect: None,
                window_header_open: false,
            },
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
        app.image.slice_index = 0;
        let (_, _, val0) = app
            .screen_to_fits_coord(egui::pos2(5.5, 4.5), canvas)
            .unwrap();
        // py = floor(4.5) = 4, px = floor(5.5) = 5 -> idx = 45
        assert_eq!(val0, 45.0);

        // Slice 1: slice_offset = 100 -> idx = 145
        app.image.slice_index = 1;
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
        app.viewport.zoom = 2.0;
        let (_, _, val) = app
            .screen_to_fits_coord(egui::pos2(50.0, 50.0), canvas)
            .unwrap();
        // py = 50, px = 50 -> idx = 50 * 100 + 50 = 5050
        assert_eq!(val, 5050.0);

        // With pan of 0.1 in x: center shifts
        app.viewport.pan = egui::vec2(0.1, 0.0);
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
        app.viewport.rotation = std::f32::consts::FRAC_PI_2; // 90 degrees
        let (fx, fy, _) = app
            .screen_to_fits_coord(egui::pos2(50.0, 50.0), canvas)
            .unwrap();
        assert!((fx - 50.0).abs() < 1e-3);
        assert!((fy - 50.0).abs() < 1e-3);
    }
}
