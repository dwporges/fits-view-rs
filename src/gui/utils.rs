use crate::gui::state::{ImageData, RenderSettings, ViewportState};
use eframe::egui;
use egui_extras::Column;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TableColumnSort {
    Ascending,
    Descending,
    None,
}

pub struct TableState {
    pub sort_column: Option<usize>,
    pub sort_direction: TableColumnSort,
    pub filters: Vec<String>,
    pub cached_indices: Vec<usize>,
    pub last_hdu_index: Option<usize>,
    pub needs_sort: bool,
}

impl TableState {
    pub fn new(cols: usize) -> Self {
        Self {
            sort_column: None,
            sort_direction: TableColumnSort::None,
            filters: vec![String::new(); cols],
            cached_indices: Vec::new(),
            last_hdu_index: None,
            needs_sort: true,
        }
    }

    pub fn toggle_sort(&mut self, column: usize) {
        if self.sort_column == Some(column) {
            self.sort_direction = match self.sort_direction {
                TableColumnSort::Ascending => TableColumnSort::Descending,
                TableColumnSort::Descending => TableColumnSort::None,
                TableColumnSort::None => TableColumnSort::Ascending,
            };
        } else {
            self.sort_column = Some(column);
            self.sort_direction = TableColumnSort::Ascending;
        }
        self.needs_sort = true;
    }
}

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

pub fn build_header_table(ui: &mut egui::Ui, image: &ImageData, table_state: &mut TableState) {
    let current_hdu = &image.hdus[&image.current_hdu_index];
    let header = &current_hdu.header;

    if table_state.last_hdu_index != Some(image.current_hdu_index) {
        table_state.last_hdu_index = Some(image.current_hdu_index);
        table_state.needs_sort = true;
    }

    if table_state.needs_sort {
        let mut indices: Vec<usize> = header
            .cards
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let has_data = c.key.as_str() != ""
                    || c.value.as_deref().unwrap_or("") != ""
                    || c.comment.as_deref().unwrap_or("") != "";
                
                if !has_data {
                    return None;
                }

                let matches_filter = |col: usize, text: &str| -> bool {
                    if table_state.filters.len() <= col || table_state.filters[col].is_empty() {
                        return true;
                    }
                    let filter = table_state.filters[col].to_lowercase();
                    text.to_lowercase().contains(&filter)
                };

                let key_match = matches_filter(0, &c.key);
                let val_match = matches_filter(1, c.value.as_deref().unwrap_or(""));
                let comm_match = matches_filter(2, c.comment.as_deref().unwrap_or(""));

                if key_match && val_match && comm_match {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        if let Some(col) = table_state.sort_column {
            match table_state.sort_direction {
                TableColumnSort::Ascending | TableColumnSort::Descending => {
                    indices.sort_by(|&i, &j| {
                        let a = &header.cards[i];
                        let b = &header.cards[j];

                        let ordering = match col {
                            0 => a.key.cmp(&b.key),
                            1 => {
                                let val_a = a.value.as_deref().unwrap_or("");
                                let val_b = b.value.as_deref().unwrap_or("");
                                val_a.cmp(val_b)
                            }
                            2 => {
                                let comm_a = a.comment.as_deref().unwrap_or("");
                                let comm_b = b.comment.as_deref().unwrap_or("");
                                comm_a.cmp(comm_b)
                            }
                            _ => std::cmp::Ordering::Equal,
                        };
                        if matches!(table_state.sort_direction, TableColumnSort::Descending) {
                            ordering.reverse()
                        } else {
                            ordering
                        }
                    });
                }
                TableColumnSort::None => {}
            }
        }
        table_state.cached_indices = indices;
        table_state.needs_sort = false;
    }

    let n_rows = table_state.cached_indices.len();

    egui_extras::TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .column(Column::initial(120.0).at_least(80.0))
        .column(Column::initial(180.0).at_least(100.0))
        .column(Column::remainder().at_least(150.0))
        .header(45.0, |mut header| {
            header.col(|ui| {
                ui.vertical(|ui| {
                    let text = match (table_state.sort_column, table_state.sort_direction) {
                        (Some(0), TableColumnSort::Ascending) => "Key ▲",
                        (Some(0), TableColumnSort::Descending) => "Key ▼",
                        _ => "Key",
                    };
                    if ui
                        .add(egui::Button::new(egui::RichText::new(text).heading()).frame(false))
                        .clicked()
                    {
                        table_state.toggle_sort(0);
                    }
                    if ui.add(egui::TextEdit::singleline(&mut table_state.filters[0]).hint_text("Filter...")).changed() {
                        table_state.needs_sort = true;
                    }
                });
            });
            header.col(|ui| {
                ui.vertical(|ui| {
                    let text = match (table_state.sort_column, table_state.sort_direction) {
                        (Some(1), TableColumnSort::Ascending) => "Value ▲",
                        (Some(1), TableColumnSort::Descending) => "Value ▼",
                        _ => "Value",
                    };
                    if ui
                        .add(egui::Button::new(egui::RichText::new(text).heading()).frame(false))
                        .clicked()
                    {
                        table_state.toggle_sort(1);
                    }
                    if ui.add(egui::TextEdit::singleline(&mut table_state.filters[1]).hint_text("Filter...")).changed() {
                        table_state.needs_sort = true;
                    }
                });
            });
            header.col(|ui| {
                ui.vertical(|ui| {
                    let text = match (table_state.sort_column, table_state.sort_direction) {
                        (Some(2), TableColumnSort::Ascending) => "Comment ▲",
                        (Some(2), TableColumnSort::Descending) => "Comment ▼",
                        _ => "Comment",
                    };
                    if ui
                        .add(egui::Button::new(egui::RichText::new(text).heading()).frame(false))
                        .clicked()
                    {
                        table_state.toggle_sort(2);
                    }
                    if ui.add(egui::TextEdit::singleline(&mut table_state.filters[2]).hint_text("Filter...")).changed() {
                        table_state.needs_sort = true;
                    }
                });
            });
        })
        .body(|body| {
            body.rows(18.0, n_rows, |mut row| {
                let row_index = row.index();
                let card = &header.cards[table_state.cached_indices[row_index]];

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
