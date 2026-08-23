use crate::gui::state::{ImageData, RenderSettings, ViewportState};
use eframe::egui;
use egui_extras::Column;

pub fn aspect_scale(image: &ImageData, canvas_size: egui::Vec2) -> egui::Vec2 {
    if image.width > 0 && image.height > 0 && canvas_size.x > 0.0 && canvas_size.y > 0.0 {
        let img_aspect = image.width as f32 / image.height as f32;
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

pub fn screen_to_fits_coord(
    image: &ImageData,
    render: &RenderSettings,
    viewport: &ViewportState,
    screen_pos: egui::Pos2,
    canvas_rect: egui::Rect,
) -> Option<(f32, f32, f64)> {
    if image.width == 0
        || image.height == 0
        || canvas_rect.width() <= 0.0
        || canvas_rect.height() <= 0.0
    {
        return None;
    }

    if !canvas_rect.contains(screen_pos) {
        return None;
    }

    let a_scale = aspect_scale(image, canvas_rect.size());

    let p = egui::vec2(
        (screen_pos.x - canvas_rect.min.x) / canvas_rect.width(),
        (screen_pos.y - canvas_rect.min.y) / canvas_rect.height(),
    );

    let centered = p - egui::vec2(0.5, 0.5);
    let uv_unrotated = (centered * a_scale / viewport.zoom) - viewport.pan + egui::vec2(0.5, 0.5);

    let s = viewport.rotation.sin();
    let c = viewport.rotation.cos();
    let uv_centered = uv_unrotated - egui::vec2(0.5, 0.5);
    let uv = egui::vec2(
        uv_centered.x * c - uv_centered.y * s,
        uv_centered.x * s + uv_centered.y * c,
    ) + egui::vec2(0.5, 0.5);

    if uv.x < 0.0 || uv.x >= 1.0 || uv.y < 0.0 || uv.y >= 1.0 {
        return None;
    }

    let fits_x = uv.x * image.width as f32;
    let fits_y = (1.0 - uv.y) * image.height as f32;

    let px = (uv.x * image.width as f32).floor() as usize;
    let py = (uv.y * image.height as f32).floor() as usize;
    let px = px.min(image.width.saturating_sub(1));
    let py = py.min(image.height.saturating_sub(1));

    let plane_size = image.width * image.height;
    let slice_offset = image.slice_index * plane_size;
    let idx = slice_offset + py * image.width + px;

    let val = if let Some(image_data) = &image.image_data {
        if idx < image_data.len() {
            image_data.get_f64_pixel(idx, render.bscale, render.bzero)
        } else {
            f64::NAN
        }
    } else {
        f64::NAN
    };

    Some((fits_x, fits_y, val))
}

pub fn build_header_table(ui: &mut egui::Ui, image: &ImageData) {
    let current_hdu = &image.hdus[&image.current_hdu_index];
    let header = &current_hdu.header;
    let n_rows = header.cards.len();

    egui_extras::TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .columns(Column::auto(), 3)
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.heading("Key");
            });
            header.col(|ui| {
                ui.heading("Value");
            });
            header.col(|ui| {
                ui.heading("Comment");
            });
        })
        .body(|body| {
            body.rows(18.0, n_rows, |mut row| {
                let row_index = row.index();
                let card = &header.cards[row_index];

                row.col(|ui| {
                    ui.label(&card.key);
                });

                let val_str = card.value.as_deref().unwrap_or("");
                row.col(|ui| {
                    ui.label(val_str);
                });

                let comment_str = card.comment.as_deref().unwrap_or("");
                row.col(|ui| {
                    ui.label(comment_str);
                });
            });
        });
}
