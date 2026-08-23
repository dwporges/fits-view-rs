use crate::gui::state::{ImageData, RenderSettings, ViewportState};
use crate::gui::utils::screen_to_fits_coord;
use eframe::egui;

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    image: &ImageData,
    render: &RenderSettings,
    viewport: &mut ViewportState,
) {
    let hover_info = ctx.input(|i| i.pointer.hover_pos()).and_then(|screen_pos| {
        viewport
            .last_canvas_rect
            .and_then(|rect| screen_to_fits_coord(image, render, viewport, screen_pos, rect))
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
}
