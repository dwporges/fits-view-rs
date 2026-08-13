use super::*;

use crate::errors::FitsError;
use byteorder::{BigEndian, ByteOrder};


pub fn get_physical_values(bytes: &[u8], bitpix: i32, bscale: f64, bzero: f64) -> Result<Vec<f64>, FitsError> {
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
        _ => Err(FitsError::InvalidBitpix(bitpix)),
    }
}