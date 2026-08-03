use super::*;

use crate::errors::FitsError;
use byteorder::{BigEndian, ByteOrder};

pub fn get_image(bytes: &[u8], bitpix: i32) -> Result<FitsData, FitsError> {
    match bitpix {
        8 => Ok(FitsData::Int8(bytes.to_vec())),
        16 => {
            let elements = bytes.len() / 2;
            let mut out = vec![0i16; elements];
            // Slice the bytes to exactly elements * 2 to prevent byteorder panics
            BigEndian::read_i16_into(&bytes[..elements * 2], &mut out);
            Ok(FitsData::Int16(out))
        }
        32 => {
            let elements = bytes.len() / 4;
            let mut out = vec![0i32; elements];
            BigEndian::read_i32_into(&bytes[..elements * 4], &mut out);
            Ok(FitsData::Int32(out))
        }
        64 => {
            let elements = bytes.len() / 8;
            let mut out = vec![0i64; elements];
            BigEndian::read_i64_into(&bytes[..elements * 8], &mut out);
            Ok(FitsData::Int64(out))
        }
        -32 => {
            let elements = bytes.len() / 4;
            let mut out = vec![0.0f32; elements];
            BigEndian::read_f32_into(&bytes[..elements * 4], &mut out);
            Ok(FitsData::Float32(out))
        }
        -64 => {
            let elements = bytes.len() / 8;
            let mut out = vec![0.0f64; elements];
            BigEndian::read_f64_into(&bytes[..elements * 8], &mut out);
            Ok(FitsData::Float64(out))
        }
        _ => Err(FitsError::InvalidBitpix(bitpix)),
    }
}