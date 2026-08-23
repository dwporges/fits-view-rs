use crate::gui::state::RenderSettings;
use eframe::egui;

pub fn format_recolor_mode(mode: u32) -> &'static str {
    match mode {
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

pub fn show(ui: &mut egui::Ui, render: &mut RenderSettings) {
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
                .selected_text(format_recolor_mode(render.recolor_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut render.recolor_mode, 0, "Grayscale");
                    ui.selectable_value(&mut render.recolor_mode, 1, "Heat");
                    ui.selectable_value(&mut render.recolor_mode, 2, "Cool");
                    ui.selectable_value(&mut render.recolor_mode, 3, "Rainbow");
                    ui.selectable_value(&mut render.recolor_mode, 4, "Iron");
                    ui.selectable_value(&mut render.recolor_mode, 5, "Posterize");
                    ui.selectable_value(&mut render.recolor_mode, 6, "Tint");
                });
        });

        ui.checkbox(&mut render.invert, "Invert Colormap");
        ui.horizontal(|ui| {
            ui.checkbox(&mut render.lock_bias, "Lock Bias");
            ui.checkbox(&mut render.lock_contrast, "Lock Contrast");
        });

        ui.add_space(2.0);
        ui.add_enabled(
            !render.lock_bias,
            egui::Slider::new(&mut render.bias, 0.0..=1.0).text("Bias"),
        );
        ui.add_enabled(
            !render.lock_contrast,
            egui::Slider::new(&mut render.contrast, 0.0..=10.0).text("Contrast"),
        );

        if render.recolor_mode == 5 {
            ui.add(
                egui::Slider::new(&mut render.posterize_levels, 2.0..=32.0)
                    .text("Posterize Levels"),
            );
        }

        ui.add_space(2.0);
        if ui
            .button("↺ Reset Bias & Contrast")
            .on_hover_text("Reset Bias to 0.5 and Contrast to 1.0 (if unlocked)")
            .clicked()
        {
            if !render.lock_bias {
                render.bias = 0.5;
            }
            if !render.lock_contrast {
                render.contrast = 1.0;
            }
        }
        ui.add_space(4.0);
    });
}
