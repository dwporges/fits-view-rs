use crate::gui::state::{ImageData, RenderSettings};
use crate::image::scalars::Scaling;
use eframe::egui;

fn format_scaling_method(method: Scaling) -> &'static str {
    match method {
        Scaling::LINEAR => "Linear (lin)",
        Scaling::LOGARITHMIC => "Logarithmic (log)",
        Scaling::SQUAREROOT => "Square Root (sqrt)",
        Scaling::ASINH => "Asinh (asinh)",
    }
}

pub fn show(ui: &mut egui::Ui, render: &mut RenderSettings, image: &mut ImageData) {
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
                .selected_text(format_scaling_method(render.scaling_method))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut render.scaling_method,
                        Scaling::LINEAR,
                        "Linear (lin)",
                    );
                    ui.selectable_value(
                        &mut render.scaling_method,
                        Scaling::LOGARITHMIC,
                        "Logarithmic (log)",
                    );
                    ui.selectable_value(
                        &mut render.scaling_method,
                        Scaling::SQUAREROOT,
                        "Square Root (sqrt)",
                    );
                    ui.selectable_value(
                        &mut render.scaling_method,
                        Scaling::ASINH,
                        "Asinh (asinh)",
                    );
                });
        });

        ui.add_space(2.0);
        ui.add(
            egui::Slider::new(&mut render.black_point, render.min..=render.white_point)
                .text("Black Point"),
        );
        ui.add(
            egui::Slider::new(&mut render.white_point, render.black_point..=render.max)
                .text("White Point"),
        );

        ui.horizontal(|ui| {
            if ui
                .button("Full Range")
                .on_hover_text("Reset cuts to full cube min and max")
                .clicked()
            {
                render.black_point = render.min;
                render.white_point = render.max;
            }
            if image.max_slices > 1 {
                if ui
                    .button("Slice Range")
                    .on_hover_text("Reset cuts to active slice min and max")
                    .clicked()
                {
                    let (s_min, s_max) = current_slice_min_max(image, render);
                    render.black_point = s_min;
                    render.white_point = s_max;
                }
            }
        });

        if image.max_slices > 1 {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Slice:");
                if ui.button("◀").clicked() && image.slice_index > 0 {
                    image.slice_index -= 1;
                }
                ui.add(
                    egui::Slider::new(
                        &mut image.slice_index,
                        0..=(image.max_slices.saturating_sub(1)),
                    )
                    .text(format!("/ {}", image.max_slices.saturating_sub(1))),
                );
                if ui.button("▶").clicked() && image.slice_index + 1 < image.max_slices {
                    image.slice_index += 1;
                }
            });
        }
        ui.add_space(4.0);
    });
}

pub fn current_slice_min_max(image: &ImageData, render: &RenderSettings) -> (f64, f64) {
    if let Some(image_data) = &image.image_data {
        let plane_size = image.width * image.height;
        let offset = image.slice_index * plane_size;
        if offset + plane_size <= image_data.len() {
            image_data.get_slice_min_max(offset, plane_size, render.bscale, render.bzero)
        } else {
            (render.min, render.max)
        }
    } else {
        (0.0, 1.0)
    }
}
