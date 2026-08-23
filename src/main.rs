pub mod constants;
pub mod errors;
pub mod gui;
pub mod header;
pub mod headers;
pub mod image;
pub mod wcs;

use std::fs::File;

use crate::gui::wgpu_gui;
use crate::header::mparser::crawl;
use crate::image::*;
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

    eframe::run_native(
        wgpu_gui::FitsViewerApp::name(),
        native_options,
        Box::new(move |cc| {
            let wgpu_render_state = cc.wgpu_render_state.as_ref().expect(
                "Failed to initialise Wgpu renderer. Ensure your GPU supports Vulkan/Metal/DX12.",
            );

            println!(
                "Connected to GPU: {:?}",
                wgpu_render_state.adapter.get_info().name
            );

            Ok(Box::new(wgpu_gui::FitsViewerApp::new(
                cc, hdus, hdu_n, args.slice,
            )))
        }),
    )
    .unwrap();
    // img.save(args.image_filename).unwrap();
    Ok(())
}
