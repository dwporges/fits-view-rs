use crate::image::render::normalise;
use crate::image::scalars::Scaling;
use anyhow::{Context, Result, bail};
use eframe::egui;
use egui_plot::{Plot, PlotImage, PlotPoint};

#[derive(Default)]
pub struct FitsViewerApp {
    width: usize,
    height: usize,
    physical_values: Vec<f64>,
    _pixels_buffer: Vec<egui::Color32>,
    min: f64,
    max: f64,
    black_point: f64,
    white_point: f64,

    scaling_method: Scaling,
    slice_index: usize,
    max_slices: usize,

    texture: Option<egui::TextureHandle>,
    needs_recalc: bool,
    hover_info: Option<(f64, f64, f64)>,
}

impl FitsViewerApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        width: usize,
        height: usize,
        physical_values: Vec<f64>,
        slice_index: usize,
        max_slices: usize,
    ) -> Self {
        let min_val = physical_values
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max_val = physical_values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        // let (initial_bp, initial_wp) = get_percentile_min_max(
        //     &physical_values,
        //     SCALING_ASINH_PERCENTILE_LOWER,
        //     SCALING_ASINH_PERCENTILE_UPPER
        // ).unwrap_or((min_val, max_val));

        let (initial_bp, initial_wp) = (min_val, max_val);

        Self {
            width,
            height,
            physical_values,
            _pixels_buffer: vec![],
            min: min_val,
            max: max_val,
            black_point: initial_bp,
            white_point: initial_wp,
            scaling_method: Scaling::ASINH,
            slice_index,
            max_slices,
            texture: None,
            needs_recalc: true,
            hover_info: None,
        }
    }

    pub fn name() -> &'static str {
        "FITS viewer"
    }

    pub fn recalculate_image(&mut self, ctx: &egui::Context) -> Result<()> {
        if self.width == 0 || self.height == 0 || self.physical_values.is_empty() {
            bail!("No image data loaded into the viewer");
        }

        let plane_size = self.width * self.height;
        let slice_offset = self.slice_index * plane_size;
        let physical_values_plane = &self.physical_values[slice_offset..slice_offset + plane_size];
        let plane = normalise(
            &physical_values_plane,
            &self.scaling_method,
            self.black_point,
            self.white_point,
        )
        .expect("FAILURE IN SCALING");

        // calculate_pixels(&plane, self.width, self.height, &mut self.pixels_buffer);

        let size = [self.width as _, self.height as _];
        let color_image = egui::ColorImage::from_gray(size, &plane);

        match &mut self.texture {
            Some(t) => t.set(color_image, egui::TextureOptions::NEAREST),
            _ => {
                self.texture = Some(ctx.load_texture(
                    "fits_image",
                    color_image,
                    egui::TextureOptions::NEAREST,
                ));
            }
        }

        self.needs_recalc = false;

        Ok(())
    }

    fn format_scaling_method_short(&mut self) -> &str {
        match self.scaling_method {
            Scaling::LINEAR => "lin",
            Scaling::LOGARITHMIC => "log",
            Scaling::SQUAREROOT => "sqrt",
            Scaling::ASINH => "asinh",
        }
    }
}

impl eframe::App for FitsViewerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        egui::Panel::right("sliders").show(ui, |ui| {
            ui.heading("Side panel");

            ui.separator();

            let combo_response_scaling_method =
                egui::ComboBox::new("combo_box_scaling_method", "Scaling Method")
                    .selected_text(format!("{}", self.format_scaling_method_short()))
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        changed |= ui
                            .selectable_value(&mut self.scaling_method, Scaling::LINEAR, "lin")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut self.scaling_method, Scaling::LOGARITHMIC, "log")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut self.scaling_method, Scaling::SQUAREROOT, "sqrt")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut self.scaling_method, Scaling::ASINH, "asinh")
                            .changed();
                        changed
                    });

            let slider_response_slice_idx = ui.add(
                egui::Slider::new(&mut self.slice_index, 0..=(self.max_slices - 1)).text("Slice"),
            );

            let slider_response_blackpoint = ui.add(
                egui::Slider::new(&mut self.black_point, self.min..=(self.white_point))
                    .text("blackpoint")
                    .logarithmic(true),
            );
            let slider_response_whitepoint = ui.add(
                egui::Slider::new(&mut self.white_point, (self.black_point)..=self.max)
                    .text("whitepoint")
                    .logarithmic(true),
            );

            if combo_response_scaling_method.inner == Some(true) {
                self.needs_recalc = true;
            }
            if slider_response_slice_idx.drag_stopped() || slider_response_slice_idx.changed() {
                self.needs_recalc = true;
            }
            if slider_response_blackpoint.drag_stopped()
                || slider_response_whitepoint.drag_stopped()
            {
                self.needs_recalc = true;
            }

            ui.separator();

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
                        let val_text = match self.hover_info {
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
                        let pos_text = match self.hover_info {
                            Some((x, y, _)) => format!("x {:.3} y {:.3}", x, y),
                            None => "x  y  ".to_string(),
                        };
                        ui.label(pos_text);
                        ui.end_row();
                    });
                ui.add_space(4.0);
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(texture) = &self.texture {
                let image_size = texture.size();

                let plot_image = PlotImage::new(
                    "image",
                    texture.id(),
                    PlotPoint::new(0.0, 0.0),
                    [image_size[0] as f32, image_size[1] as f32],
                );

                let mut hovered = None;
                Plot::new("fits_image_plot")
                    .data_aspect(1.0)
                    .show(ui, |plot_ui| {
                        plot_ui.image(plot_image);
                        if let Some(plot_coord) = plot_ui.pointer_coordinate() {
                            let fits_x = plot_coord.x + self.width as f64 / 2.0;
                            let fits_y = plot_coord.y + self.height as f64 / 2.0;
                            if fits_x >= 0.0
                                && fits_x < self.width as f64
                                && fits_y >= 0.0
                                && fits_y < self.height as f64
                            {
                                let px = fits_x.floor() as usize;
                                let py = (self.height as f64 - 1.0 - fits_y).floor() as usize;
                                let plane_size = self.width * self.height;
                                let slice_offset = self.slice_index * plane_size;
                                let idx = slice_offset + py * self.width + px;
                                let val =
                                    self.physical_values.get(idx).copied().unwrap_or(f64::NAN);
                                hovered = Some((fits_x, fits_y, val));
                            }
                        }
                    });
                self.hover_info = hovered;
            } else {
                ui.spinner();
            }
        });

        if self.needs_recalc {
            let _ = self
                .recalculate_image(&ctx)
                .context("Failed to recalculate image");
        }
    }
}
