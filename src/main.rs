pub mod constants;
pub mod header;
pub mod headers;
pub mod image;
pub mod errors;
pub mod gui;
pub mod wcs;

#[allow(deprecated)]
use crate::gui::glow_gui;
use crate::gui::wgpu_gui;
use crate::header::*;
use crate::image::*;
use crate::errors::*;
use clap::Parser;
use header::parser::*;

use crate::image::image::get_image;

use crate::constants::{FITS_BLOCKSIZE};

use env_logger;
use log::{info};


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

    let all_data = read_file(&fname)?;

    let data = get_hdu(&all_data, hdu_n)?;

    let basic_info = BasicFitsInfo::from_block(&data)?;

    info!(
        "Basic Information: BITPIX {:?}, NAXIS {}, AXES {:?}, N_PIXELS {}, N_BYTES {}, BSCALE {:?}, BZERO {:?}",
        basic_info.bitpix.value,
        basic_info.naxis,
        basic_info.axes,
        basic_info.n_pixels,
        basic_info.n_bytes,
        basic_info.bscale,
        basic_info.bzero,
    );

    let image_data = extract_image_data(&data, &basic_info)?;

    let width: usize = basic_info.axes.get(0).copied().unwrap_or(1);
    let height:usize = basic_info.axes.get(1).copied().unwrap_or(1);
    let bscale: f64 = basic_info.bscale;
    let bzero: f64 = basic_info.bzero;

    let plane_count = if basic_info.naxis <= 2 {
        1
    } else {
        basic_info.axes[2..].iter().product()
    };

    if args.slice >= plane_count {
        return Err(format!("slice index {} out of range, only {} plane(s) available", args.slice, plane_count).into());
    }

    let physical_values = to_physical_values(image_data.to_f64(), bscale, bzero);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
                .with_maximized(true),
        renderer: eframe::Renderer::Wgpu,
        ..eframe::NativeOptions::default() 
    };

    match args.use_glow {
        false => {
            eframe::run_native(wgpu_gui::FitsViewerApp::name(), 
            native_options, 
            Box::new(|cc| { 
                let wgpu_render_state = cc.wgpu_render_state.as_ref()
                    .expect("Failed to initialise Wgpu renderer. Ensure your GPU supports Vulkan/Metal/DX12.");

                println!("Connected to GPU: {:?}", wgpu_render_state.adapter.get_info().name);

                Ok(Box::new(
                    wgpu_gui::FitsViewerApp::new(cc, width, height, physical_values, args.slice, plane_count)
                ))
            }))
            .unwrap();
        }

        true => {
            let mut user_choice = String::new();
            println!("Warning: GLOW backend is not developed and may not work. Are you sure you want to use it? (y/n)");

            std::io::stdin()
                .read_line(&mut user_choice)
                .expect("Failed to read line");

            match user_choice.to_uppercase().as_ref() {
                "Y" => {
                    eframe::run_native(
                    glow_gui::FitsViewerApp::name(), 
                    native_options, 
                    Box::new(|cc| Ok(Box::new(glow_gui::FitsViewerApp::new(cc, width, height, physical_values, args.slice, plane_count)))))
                    .unwrap();
                } 
                _ => return Ok(())
            }
        }
    }



    // img.save(args.image_filename).unwrap();
    Ok(())
}


fn extract_image_data(data: &[u8], info: &BasicFitsInfo) -> Result<FitsData, FitsError> {
    let final_header_block = find_end_block(&data)?;
    println!("Final header block at index: {}", final_header_block);

    let image_start = (final_header_block + 1) * FITS_BLOCKSIZE;
    let image_end = image_start + info.n_bytes;
    let image_bytes = &data[image_start..image_end];

    let fits_data = get_image(image_bytes, info.bitpix.value.ok_or(FitsError::MissingBitpix)? as i32)?;

    Ok(fits_data)
}


fn to_physical_values(data: Vec<f64>, bscale: f64, bzero: f64) -> Vec<f64> {
    data
    .iter()
    .map(|&v| {
        (v as f64 * bscale) + bzero
    }).collect()
}

