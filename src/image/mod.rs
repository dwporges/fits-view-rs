pub mod image;
pub mod render;
pub mod scalars;
pub mod wgpu_shader_source;

use log::warn;

#[derive(Debug)]
pub enum FitsData {
    Int8(Vec<u8>),      // BITPIX =   8
    Int16(Vec<i16>),    // BITPIX =  16
    Int32(Vec<i32>),    // BITPIX =  32
    Float32(Vec<f32>),  // BITPIX = -32
    Int64(Vec<i64>),    // BITPIX =  64
    Float64(Vec<f64>),  // BITPIX = -64
}

impl FitsData {
    pub fn to_f64(&self) -> Vec<f64> {
        let mut flag_i64_precision_loss = false;
        
        match self {
            FitsData::Int8(v) => v.iter().map(|&x| x as f64).collect(), 
            FitsData::Int16(v) => v.iter().map(|&x| x as f64).collect(),
            FitsData::Int32(v) => v.iter().map(|&x| x as f64).collect(),
            FitsData::Float32(v) => v.iter().map(|&x| x as f64).collect(),
            FitsData::Int64(v) => v.iter().map(|&x| {
                let f = x as f64;
                if (f as i64) != x && !flag_i64_precision_loss {
                    warn!("I64 precision loss due to casting to f64");
                    flag_i64_precision_loss = true; 
                }
                f
            }).collect(),
            FitsData::Float64(v) => v.clone(),
        }
    }
}