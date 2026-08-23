use crate::gui::components::scaling::current_slice_min_max;
use crate::gui::state::{ImageData, RenderSettings, ViewportState};
use eframe::egui;

pub fn show(
    ui: &mut egui::Ui,
    image: &ImageData,
    render: &RenderSettings,
    viewport: &ViewportState,
) {
    egui::CollapsingHeader::new(egui::RichText::new("Image Information").strong().size(13.0))
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(2.0);
            egui::Grid::new("image_info_grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Dimensions:").strong());
                    ui.label(format!("{} x {} px", image.width, image.height));
                    ui.end_row();

                    ui.label(egui::RichText::new("Data Min:").strong());
                    ui.label(format!("{:.4e}", render.min));
                    ui.end_row();

                    ui.label(egui::RichText::new("Data Max:").strong());
                    ui.label(format!("{:.4e}", render.max));
                    ui.end_row();

                    if image.max_slices > 1 {
                        ui.label(egui::RichText::new("Slices:").strong());
                        ui.label(format!(
                            "{} (active: #{})",
                            image.max_slices, image.slice_index
                        ));
                        ui.end_row();

                        let (s_min, s_max) = current_slice_min_max(image, render);
                        ui.label(egui::RichText::new("Slice Range:").strong());
                        ui.label(format!("{:.3e} .. {:.3e}", s_min, s_max));
                        ui.end_row();
                    }

                    ui.label(egui::RichText::new("Zoom:").strong());
                    ui.label(format!("{:.2}x", viewport.zoom));
                    ui.end_row();

                    ui.label(egui::RichText::new("Pan:").strong());
                    ui.label(format!("({:.2}, {:.2})", viewport.pan.x, viewport.pan.y));
                    ui.end_row();
                });
            ui.add_space(4.0);
        });
}
