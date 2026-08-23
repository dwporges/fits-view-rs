use crate::gui::state::{ImageData, RenderSettings, ViewportState};
use crate::gui::utils::aspect_scale;
use crate::image::scalars::Scaling;
use crate::render::FitsRenderCallback;
use eframe::egui;

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    viewport: &mut ViewportState,
    render: &mut RenderSettings,
    image: &ImageData,
) {
    let (rect, response) =
        ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
    viewport.last_canvas_rect = Some(rect);

    let a_scale = aspect_scale(image, rect.size());

    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        let d_norm = egui::vec2(delta.x / rect.width(), delta.y / rect.height());
        viewport.pan += (d_norm * a_scale) / viewport.zoom;
    } else if response.dragged_by(egui::PointerButton::Secondary) {
        let delta = response.drag_delta();
        if !render.lock_bias {
            render.bias = (render.bias + delta.x / rect.width()).clamp(0.0, 1.0);
        }
        if !render.lock_contrast {
            render.contrast = (render.contrast - delta.y / rect.height() * 1.5).clamp(0.0, 10.0);
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

                let old_zoom = viewport.zoom;
                viewport.zoom *= zoom_delta;

                let scale = 1.0 / viewport.zoom - 1.0 / old_zoom;
                viewport.pan += c_mouse * a_scale * scale;
            }
        }
    }

    let mode_int = match render.scaling_method {
        Scaling::LINEAR => 0,
        Scaling::LOGARITHMIC => 1,
        Scaling::SQUAREROOT => 2,
        Scaling::ASINH => 3,
    };

    let callback = FitsRenderCallback {
        bp: render.black_point as f32,
        wp: render.white_point as f32,
        pan: viewport.pan,
        zoom: viewport.zoom,
        scaling_mode: mode_int,
        rotation: viewport.rotation,
        bias: render.bias,
        contrast: render.contrast,
        recolor_mode: render.recolor_mode,
        invert: render.invert,
        posterize_levels: render.posterize_levels,
        aspect_scale: a_scale,
        slice_index: image.slice_index,
    };

    ui.painter()
        .add(eframe::egui_wgpu::Callback::new_paint_callback(
            rect, callback,
        ));
}
