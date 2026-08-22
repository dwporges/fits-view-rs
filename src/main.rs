pub mod constants;
pub mod errors;
pub mod gui;
pub mod header;
pub mod headers;
pub mod image;
pub mod wcs;

use std::fs::File;
use std::sync::Arc;

#[allow(deprecated)]
use crate::gui::glow_gui;
use crate::header::mparser::crawl;
use crate::image::*;
use crate::{gui::wgpu_gui, image::image::FitsData};
use clap::Parser;

use env_logger;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    fname: String,
    #[arg(short, long, default_value_t = 0)]
    scaling_method: i8,
    #[arg(long, default_value_t = 0)]
    hdu: usize,
    #[arg(long, default_value_t = 0)]
    slice: usize,
    #[arg(short, long, default_value_t = String::from("out.png"))]
    image_filename: String,
    #[arg(long)]
    use_glow: bool,
}

#[allow(deprecated)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();
    let fname = args.fname;
    let hdu_n = args.hdu;

    let mut file = File::open(fname)?;

    let hdus = crawl(&mut file)?;

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_maximized(true),
        renderer: eframe::Renderer::Wgpu,
        ..eframe::NativeOptions::default()
    };

    match args.use_glow {
        false => {
            eframe::run_native(wgpu_gui::FitsViewerApp::name(),
            native_options,
            Box::new(move |cc| {
                let wgpu_render_state = cc.wgpu_render_state.as_ref()
                    .expect("Failed to initialise Wgpu renderer. Ensure your GPU supports Vulkan/Metal/DX12.");

                println!("Connected to GPU: {:?}", wgpu_render_state.adapter.get_info().name);

                Ok(Box::new(
                    wgpu_gui::FitsViewerApp::new(cc, hdus, hdu_n, args.slice)
                ))
            }))
            .unwrap();
        }

        true => {
            let mut user_choice = String::new();
            println!(
                "Warning: GLOW backend is not developed and may not work. Are you sure you want to use it? (y/n)"
            );

            std::io::stdin()
                .read_line(&mut user_choice)
                .expect("Failed to read line");

            match user_choice.to_uppercase().as_ref() {
                "Y" => {
                    let hdu = &hdus[hdu_n];
                    let basic_info = &hdu.basic_info;
                    let image_data_ref = hdu.data.as_ref().map(|m| m.as_ref()).unwrap_or(&[]);
                    let physical_values =
                        FitsData::new(image_data_ref, basic_info.bitpix as i32).unwrap();
                    let image_data: Arc<FitsData> = Arc::from(physical_values);
                    let width = basic_info.axes.get(0).copied().unwrap_or(1);
                    let height = basic_info.axes.get(1).copied().unwrap_or(1);
                    let bscale = basic_info.bscale;
                    let bzero = basic_info.bzero;
                    let plane_count = if basic_info.naxis <= 2 {
                        1
                    } else {
                        basic_info.axes[2..].iter().product()
                    };
                    let slice_f64 = (0..width * height)
                        .map(|i| {
                            image_data.get_f64_pixel(i + args.slice * width * height, bscale, bzero)
                        })
                        .collect::<Vec<f64>>();
                    let image_data_f64: Arc<[f64]> = Arc::from(slice_f64);
                    eframe::run_native(
                        glow_gui::FitsViewerApp::name(),
                        native_options,
                        Box::new(move |cc| {
                            Ok(Box::new(glow_gui::FitsViewerApp::new(
                                cc,
                                width,
                                height,
                                image_data_f64.clone(),
                                args.slice,
                                plane_count,
                            )))
                        }),
                    )
                    .unwrap();
                }
                _ => return Ok(()),
            }
        }
    }
    // img.save(args.image_filename).unwrap();
    Ok(())
}
