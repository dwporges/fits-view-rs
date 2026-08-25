use crate::gui::state::{ImageData, ViewportState};
use crate::gui::utils::build_header_table;
use eframe::egui;

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    image: &mut ImageData,
    viewport: &mut ViewportState,
) {
    egui::CollapsingHeader::new(egui::RichText::new("HDU Selection").strong().size(13.0))
        .default_open(true)
        .show(ui, |ui| {
            ui.add_space(2.0);

            let get_hdu_label = |index: usize, hdu: &crate::header::HDU| -> String {
                let ext_type = hdu.header.get_value("XTENSION").unwrap_or("PRIMARY");
                let clean_ext = ext_type.trim_matches('\'').trim();
                format!(
                    "HDU {}: {} ({} axes)",
                    index, clean_ext, hdu.basic_info.naxis
                )
            };

            ui.horizontal(|ui| {
                ui.label("HDU:");
                egui::ComboBox::new("hdu_combo_box", "")
                    .selected_text(get_hdu_label(
                        image.current_hdu_index,
                        &image.hdus[&image.current_hdu_index],
                    ))
                    .show_ui(ui, |ui| {
                        for key in image.hdus.keys() {
                            let text = get_hdu_label(*key, &image.hdus[key]);
                            if ui
                                .selectable_label(image.current_hdu_index == *key, text)
                                .clicked()
                            {
                                if image.current_hdu_index != *key {
                                    image.current_hdu_index = *key;
                                    image.pending_hdu_change = true;
                                }
                            }
                        }
                    });
            });

            if image.image_data.is_none() {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("!!No Image Data in this HDU").color(egui::Color32::YELLOW),
                );
            }
            ui.add_space(4.0);

            if ui.button("Header").clicked() {
                viewport.window_header_open = true;
            }

            let mut is_open = viewport.window_header_open;
            if is_open {
                egui::Window::new("Header")
                    .open(&mut is_open)
                    .default_size([700.0, 500.0])
                    .show(ctx, |ui| {
                        build_header_table(ui, image, &mut viewport.header_table_state)
                    });
                viewport.window_header_open = is_open;
            }
        });
}
