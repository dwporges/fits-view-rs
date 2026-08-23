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

pub struct ViewportState {
    pub pan: egui::Vec2,
    pub zoom: f32,
    pub rotation: f32,
    pub last_canvas_rect: Option<egui::Rect>,
    pub window_header_open: bool,
}
