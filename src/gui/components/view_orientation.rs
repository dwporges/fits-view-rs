use crate::gui::state::ViewportState;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, viewport: &mut ViewportState) {
    egui::CollapsingHeader::new(
        egui::RichText::new("View & Orientation")
            .strong()
            .size(13.0),
    )
    .default_open(true)
    .show(ui, |ui| {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let mut deg = viewport.rotation.to_degrees().rem_euclid(360.0);
            if ui
                .add(
                    egui::Slider::new(&mut deg, 0.0..=360.0)
                        .suffix("°")
                        .text("Rotation"),
                )
                .changed()
            {
                viewport.rotation = deg.to_radians();
            }
            if ui
                .add(egui::Button::new("↻"))
                .on_hover_text("Rotate 90° clockwise")
                .clicked()
            {
                viewport.rotation =
                    (viewport.rotation - std::f32::consts::FRAC_PI_2) % std::f32::consts::TAU;
            }
            if ui
                .add(egui::Button::new("↺"))
                .on_hover_text("Rotate 90° anti-clockwise")
                .clicked()
            {
                viewport.rotation =
                    (viewport.rotation + std::f32::consts::FRAC_PI_2) % std::f32::consts::TAU;
            }
        });

        ui.add_space(2.0);
        ui.add(
            egui::Slider::new(&mut viewport.zoom, 0.05..=50.0)
                .logarithmic(true)
                .suffix("x")
                .text("Zoom"),
        );

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if ui.button("1.0x Zoom").clicked() {
                viewport.zoom = 1.0;
            }
            if ui
                .button("Center View")
                .on_hover_text("Reset pan to center (0, 0)")
                .clicked()
            {
                viewport.pan = egui::Vec2::ZERO;
            }
            if ui
                .button("Reset View")
                .on_hover_text("Reset Pan, Zoom, and Rotation")
                .clicked()
            {
                viewport.pan = egui::Vec2::ZERO;
                viewport.zoom = 1.0;
                viewport.rotation = 0.0;
            }
        });
        ui.add_space(4.0);
    });
}
