use anyhow::{Result, bail};
use byteorder::{BigEndian, ByteOrder};

#[derive(Debug, Clone)]
pub enum FitsData {
    U8(Vec<u8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

impl FitsData {
    pub fn new(bytes: &[u8], bitpix: i32) -> Result<Self> {
        match bitpix {
            8 => Ok(FitsData::U8(bytes.to_vec())),
            16 => {
                let mut data = vec![0; bytes.len() / 2];
                BigEndian::read_i16_into(bytes, &mut data);
                Ok(FitsData::I16(data))
            },
            32 => {
                let mut data = vec![0; bytes.len() / 4];
                BigEndian::read_i32_into(bytes, &mut data);
                Ok(FitsData::I32(data))
            },
            64 => {
                let mut data = vec![0; bytes.len() / 8];
                BigEndian::read_i64_into(bytes, &mut data);
                Ok(FitsData::I64(data))
            },
            -32 => {
                let mut data = vec![0.0; bytes.len() / 4];
                BigEndian::read_f32_into(bytes, &mut data);
                Ok(FitsData::F32(data))
            }
            -64 => {
                let mut data = vec![0.0; bytes.len() / 8];
                BigEndian::read_f64_into(bytes, &mut data);
                Ok(FitsData::F64(data))
            },
            _ => bail!("Invalid BITPIX {}", bitpix),          
        }
    }

    // Returns a Vec<f32> to feed to the GPU rendering pipeline
    pub fn get_f32_slice(&self, offset: usize, plane_size: usize, bscale: f64, bzero: f64) -> Vec<f32> {
        match self {
            FitsData::U8(data) => data[offset..offset + plane_size]
                .iter()
                .map(|&v| ((v as f64 * bscale) + bzero) as f32)
                .collect(),
            FitsData::I16(data) => data[offset..offset + plane_size]
                .iter()
                .map(|&v| ((v as f64 * bscale) + bzero) as f32)
                .collect(),
            FitsData::I32(data) => data[offset..offset + plane_size]
                .iter()
                .map(|&v| ((v as f64 * bscale) + bzero) as f32)
                .collect(),
            FitsData::I64(data) => data[offset..offset + plane_size]
                .iter()
                .map(|&v| ((v as f64 * bscale) + bzero) as f32)
                .collect(),
            FitsData::F32(data) => data[offset..offset + plane_size]
                .iter()
                .map(|&v| ((v as f64 * bscale) + bzero) as f32)
                .collect(),
            FitsData::F64(data) => data[offset..offset + plane_size]
                .iter()
                .map(|&v| ((v * bscale) + bzero) as f32)
                .collect(),
        }
    }

    pub fn get_f64_pixel(&self, idx: usize, bscale: f64, bzero: f64) -> f64 {
        match self {
            FitsData::U8(data) => (data[idx] as f64 * bscale) + bzero,
            FitsData::I16(data) => (data[idx] as f64 * bscale) + bzero,
            FitsData::I32(data) => (data[idx] as f64 * bscale) + bzero,
            FitsData::I64(data) => (data[idx] as f64 * bscale) + bzero,
            FitsData::F32(data) => (data[idx] as f64 * bscale) + bzero,
            FitsData::F64(data) => (data[idx] * bscale) + bzero,
        }
    }

    // Computes the (min, max) for the entire datacube
    pub fn get_min_max(&self, bscale: f64, bzero: f64) -> (f64, f64) {
        self.get_slice_min_max(0, self.len(), bscale, bzero)
    }

    // Computes the (min, max) for a specific range/slice
    pub fn get_slice_min_max(&self, offset: usize, length: usize, bscale: f64, bzero: f64) -> (f64, f64) {
        macro_rules! compute_min_max {
            ($data:expr) => {{
                $data[offset..offset + length].iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &v| {
                    let val = (v as f64 * bscale) + bzero;
                    if val.is_finite() {
                        (min.min(val), max.max(val))
                    } else {
                        (min, max)
                    }
                })
            }}
        }

        match self {
            FitsData::U8(data) => compute_min_max!(data),
            FitsData::I16(data) => compute_min_max!(data),
            FitsData::I32(data) => compute_min_max!(data),
            FitsData::I64(data) => compute_min_max!(data),
            FitsData::F32(data) => compute_min_max!(data),
            FitsData::F64(data) => compute_min_max!(data),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            FitsData::U8(data) => data.len(),
            FitsData::I16(data) => data.len(),
            FitsData::I32(data) => data.len(),
            FitsData::I64(data) => data.len(),
            FitsData::F32(data) => data.len(),
            FitsData::F64(data) => data.len(),
        }
    }
}


pub fn get_physical_values(bytes: &[u8], bitpix: i32, bscale: f64, bzero: f64) -> Result<Vec<f64>> {
    match bitpix {
        8 => Ok(bytes.iter().map(|&x| (x as f64 * bscale) + bzero).collect()),
        16 => {
            let elements = bytes.len() / 2;
            Ok(bytes[..elements * 2]
                .chunks_exact(2)
                .map(|c| (i16::from_be_bytes(c.try_into().unwrap()) as f64 * bscale) + bzero)
                .collect())
        }
        32 => {
            let elements = bytes.len() / 4;
            Ok(bytes[..elements * 4]
                .chunks_exact(4)
                .map(|c| (i32::from_be_bytes(c.try_into().unwrap()) as f64 * bscale) + bzero)
                .collect())
        }
        64 => {
            let elements = bytes.len() / 8;
            Ok(bytes[..elements * 8]
                .chunks_exact(8)
                .map(|c| (i64::from_be_bytes(c.try_into().unwrap()) as f64 * bscale) + bzero)
                .collect())
        }
        -32 => {
            let elements = bytes.len() / 4;
            Ok(bytes[..elements * 4]
                .chunks_exact(4)
                .map(|c| (f32::from_be_bytes(c.try_into().unwrap()) as f64 * bscale) + bzero)
                .collect())
        }
        -64 => {
            let elements = bytes.len() / 8;
            Ok(bytes[..elements * 8]
                .chunks_exact(8)
                .map(|c| (f64::from_be_bytes(c.try_into().unwrap()) * bscale) + bzero)
                .collect())
        }
        _ => bail!("Invalid BITPIX {}", bitpix),
    }
}