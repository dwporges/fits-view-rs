use crate::header::HDU;
use crate::image::image::FitsData;
use crate::image::scalars::Scaling;
use eframe::egui;
use indexmap::IndexMap;
use std::sync::Arc;

pub struct ImageData {
    pub hdus: IndexMap<usize, HDU>,
    pub current_hdu_index: usize,
    pub pending_hdu_change: bool,
    pub width: usize,
    pub height: usize,
    pub image_data: Option<Arc<FitsData>>,
    pub slice_index: usize,
    pub max_slices: usize,
}

pub struct RenderSettings {
    pub min: f64,
    pub max: f64,
    pub bscale: f64,
    pub bzero: f64,
    pub black_point: f64,
    pub white_point: f64,
    pub scaling_method: Scaling,
    pub recolor_mode: u32,
    pub posterize_levels: f32,
    pub invert: bool,
    pub bias: f32,
    pub contrast: f32,
    pub lock_bias: bool,
    pub lock_contrast: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            bscale: 1.0,
            bzero: 0.0,
            black_point: 0.0,
            white_point: 1.0,
            scaling_method: Scaling::ASINH,
            recolor_mode: 0,
            posterize_levels: 8.0,
            invert: false,
            bias: 0.5,
            contrast: 1.0,
            lock_bias: false,
            lock_contrast: false,
        }
    }
}

pub struct ViewportState {
    pub pan: egui::Vec2,
    pub zoom: f32,
    pub rotation: f32,
    pub last_canvas_rect: Option<egui::Rect>,
    pub window_header_open: bool,
    pub header_table_state: crate::gui::utils::TableState,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            last_canvas_rect: None,
            window_header_open: false,
            header_table_state: crate::gui::utils::TableState::new(3),
        }
    }
}
