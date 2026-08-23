use crate::gui::components::colormap::format_recolor_mode;
use crate::gui::state::RenderSettings;
use crate::image::render::sample_colormap;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, render: &mut RenderSettings) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.strong("Colorbar");
        ui.separator();

        egui::ComboBox::new("bottom_combo_box_recolor_mode", "Colormap")
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

        ui.checkbox(&mut render.invert, "Invert");
        ui.checkbox(&mut render.lock_bias, "Lock Bias");
        ui.checkbox(&mut render.lock_contrast, "Lock Contrast");

        ui.separator();
        ui.label(format!("Bias: {:.2}", render.bias));
        ui.label(format!("Contrast: {:.2}", render.contrast));

        if ui
            .button("Reset (0.5, 1.0)")
            .on_hover_text("Reset Bias to 0.5 and Contrast to 1.0 (or double-click the colorbar)")
            .clicked()
        {
            if !render.lock_bias {
                render.bias = 0.5;
            }
            if !render.lock_contrast {
                render.contrast = 1.0;
            }
        }
    });

    ui.add_space(3.0);

    let bar_height = 24.0;
    let available_w = ui.available_width().max(100.0);
    let (bar_rect, bar_response) = ui.allocate_exact_size(
        egui::vec2(available_w, bar_height),
        egui::Sense::click_and_drag(),
    );

    if bar_response.dragged_by(egui::PointerButton::Primary) {
        let delta = bar_response.drag_delta();
        if !render.lock_bias {
            render.bias = (render.bias + delta.x / bar_rect.width()).clamp(0.0, 1.0);
        }
        if !render.lock_contrast {
            render.contrast = (render.contrast - delta.y * 0.005).clamp(0.0, 10.0);
        }
    } else if bar_response.clicked_by(egui::PointerButton::Primary) {
        if !render.lock_bias {
            if let Some(pos) = bar_response.interact_pointer_pos() {
                render.bias = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
            }
        }
    }

    if bar_response.double_clicked() || bar_response.clicked_by(egui::PointerButton::Secondary) {
        if !render.lock_bias {
            render.bias = 0.5;
        }
        if !render.lock_contrast {
            render.contrast = 1.0;
        }
    }

    bar_response.on_hover_text(
        "• Drag left/right to adjust Bias (if unlocked)\n• Drag up/down to adjust Contrast (if unlocked)\n• Click to set Bias position\n• Double-click or Right-click to Reset"
    );

    if ui.is_rect_visible(bar_rect) {
        let painter = ui.painter();
        let num_segments = 256;
        let mut mesh = egui::Mesh::default();

        for i in 0..=num_segments {
            let t = i as f32 / num_segments as f32;
            let color = sample_colormap(
                t,
                render.bias,
                render.contrast,
                render.recolor_mode,
                render.posterize_levels,
                render.invert,
            );
            let x = bar_rect.min.x + t * bar_rect.width();

            let top_idx = mesh.vertices.len() as u32;
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(x, bar_rect.min.y),
                uv: egui::epaint::WHITE_UV,
                color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(x, bar_rect.max.y),
                uv: egui::epaint::WHITE_UV,
                color,
            });

            if i < num_segments {
                mesh.indices.push(top_idx);
                mesh.indices.push(top_idx + 1);
                mesh.indices.push(top_idx + 2);

                mesh.indices.push(top_idx + 1);
                mesh.indices.push(top_idx + 3);
                mesh.indices.push(top_idx + 2);
            }
        }

        painter.add(egui::Shape::mesh(mesh));

        painter.rect_stroke(
            bar_rect,
            1.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(140)),
            egui::StrokeKind::Inside,
        );

        let bias_x =
            (bar_rect.min.x + render.bias * bar_rect.width()).clamp(bar_rect.min.x, bar_rect.max.x);
        painter.line_segment(
            [
                egui::pos2(bias_x, bar_rect.min.y),
                egui::pos2(bias_x, bar_rect.max.y),
            ],
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );
        painter.line_segment(
            [
                egui::pos2(bias_x - 3.0, bar_rect.min.y),
                egui::pos2(bias_x + 3.0, bar_rect.min.y),
            ],
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );
        painter.line_segment(
            [
                egui::pos2(bias_x - 3.0, bar_rect.max.y),
                egui::pos2(bias_x + 3.0, bar_rect.max.y),
            ],
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );
    }
}
