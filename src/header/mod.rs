pub mod parser;

use crate::errors::FitsError;
use crate::headers::*;
use anyhow::{Context, Result};
use std::str;

pub struct BasicFitsInfo {
    pub bitpix: FitsHeader,
    pub naxis: usize,
    pub axes: Vec<usize>,
    pub n_pixels: usize,
    pub n_bytes: usize,
    pub bscale: f64,
    pub bzero: f64,
}

impl BasicFitsInfo {
    pub fn from_block(block: &[u8]) -> Result<Self> {
        // 1. Mandatory cards (error out if missing or unparseable)
        let bitpix_raw =
            parser::find_card(block, HEADER_BITPIX).context("Missing mandatory BITPIX header")?;
        let bitpix_h = FitsHeader::new(bitpix_raw).context("BITPIX value is invalid")?;

        let naxis_raw =
            parser::find_card(block, HEADER_NAXIS).context("Missing mandatory NAXIS header")?;
        let naxis_h = FitsHeader::new(naxis_raw).context("NAXIS value is invalid")?;

        // 2. Optional cards (fallback to standard FITS defaults if card is missing)
        let bscale = match parser::find_card(block, HEADER_BSCALE) {
            Some(card_bytes) => FitsHeader::new(card_bytes)?.value.unwrap_or(1.0),
            None => 1.0, // FITS default
        };

        let bzero = match parser::find_card(block, HEADER_BZERO) {
            Some(card_bytes) => FitsHeader::new(card_bytes)?.value.unwrap_or(0.0),
            None => 0.0, // FITS default
        };

        let naxis = naxis_h.value.ok_or(FitsError::MissingEqualSign)? as usize;
        let mut axes = Vec::with_capacity(naxis);
        for axis_idx in 1..=naxis {
            let axis_key = format!("NAXIS{}", axis_idx);
            let axis_raw = parser::find_card(block, &axis_key)
                .with_context(|| format!("Could not find axis {}", axis_idx))?;
            let axis_header = FitsHeader::new(axis_raw)?;
            let axis_len = axis_header.value.ok_or(FitsError::MissingEqualSign)? as usize;
            axes.push(axis_len);
        }

        let bp_val = bitpix_h.value.ok_or(FitsError::MissingEqualSign)? as i32;
        let pixels = if naxis == 0 { 0 } else { axes.iter().product() };
        let bytes = n_bytes(pixels, bp_val);

        Ok(Self {
            bitpix: bitpix_h,
            naxis,
            axes,
            n_pixels: pixels,
            n_bytes: bytes,
            bscale,
            bzero,
        })
    }
}

pub struct FitsHeader {
    pub card: String,
    pub name: String,
    pub value: Option<f64>,
}

impl FitsHeader {
    pub fn new(card_bytes: &[u8]) -> Result<Self, FitsError> {
        // FITS headers are exactly 80 bytes long
        if card_bytes.len() != crate::constants::FITS_CARDSIZE {
            return Err(FitsError::InvalidCardLength(card_bytes.len()));
        }

        let card = str::from_utf8(card_bytes)?;
        let name = card[0..8].trim().to_string();

        if name == "END" || name == "COMMENT" || name == "HISTORY" || name.is_empty() {
            return Ok(Self {
                card: card.to_string(),
                name,
                value: None,
            });
        }

        let val_part = card.split('=').nth(1).ok_or(FitsError::MissingEqualSign)?;

        let val_str = val_part.split('/').next().unwrap().trim();

        // Convert Fortran 'D'/'d' exponents to standard 'E'/'e' so Rust can parse it
        let sanitized_val = val_str.replace("D", "E").replace("d", "e");

        let parsed_val = sanitized_val.parse::<f64>().ok();

        Ok(Self {
            card: card.to_string(),
            name,
            value: parsed_val,
        })
    }
}

fn n_bytes(n_pixels: usize, bitpix: i32) -> usize {
    n_pixels * bitpix.abs() as usize / 8
}
