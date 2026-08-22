pub mod mparser;

use crate::headers::*;
use anyhow::{Context, Result};
use std::str::{self, FromStr};

pub struct BasicHDUInfo {
    pub bitpix: isize,
    pub naxis: usize,
    pub axes: Vec<usize>,
    pub n_pixels: usize,
    pub n_bytes: usize,
    pub bscale: f64,
    pub bzero: f64,
}

impl BasicHDUInfo {
    pub fn from_header(header: &FitsHeader) -> Result<Self> {
        let bitpix: isize = header
            .get(HEADER_BITPIX)
            .context("Missing mandatory BITPIX header")?;
        let naxis: usize = header
            .get(HEADER_NAXIS)
            .context("Missing mandatory NAXIS header")?;
        let bscale: f64 = header.get_float(HEADER_BSCALE).unwrap_or(1.0);
        let bzero: f64 = header.get_float(HEADER_BZERO).unwrap_or(0.0);
        let pcount: usize = header.get(HEADER_PCOUNT).unwrap_or(0);
        let gcount: usize = header.get(HEADER_GCOUNT).unwrap_or(1);

        let mut axes = Vec::with_capacity(naxis);

        for i in 1..=naxis {
            let axis_key = format!("NAXIS{}", i);
            let axis_len: usize = header.get(&axis_key).context(format!(
                "HDU has {} axes but {} header is not preset",
                naxis, axis_key
            ))?;

            axes.push(axis_len);
        }

        let n_pixels = if naxis == 0 { 0 } else { axes.iter().product() };
        let n_bytes = (bitpix.abs() as usize / 8) * gcount * (pcount + n_pixels);

        Ok(Self {
            bitpix,
            naxis,
            axes,
            n_pixels,
            n_bytes,
            bscale,
            bzero,
        })
    }
}

pub struct HDU {
    pub hdu_index: usize,
    pub n_blocks: usize,
    pub total_size: usize,
    pub data: Option<memmap2::Mmap>,
    pub header: FitsHeader,
    pub basic_info: BasicHDUInfo,
}

#[derive(Debug, Default)]
pub struct FitsHeader {
    pub cards: Vec<Card>,
}

#[derive(Debug)]
pub struct Card {
    pub key: String,
    pub value: Option<String>,
    pub comment: Option<String>,
}

impl FitsHeader {
    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.cards
            .iter()
            .find(|card| card.key == key)
            .and_then(|card| card.value.as_deref())
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get_value(key)?.parse::<i64>().ok()
    }

    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get_value(key)?.parse::<f64>().ok()
    }

    pub fn get<T: FromStr>(&self, key: &str) -> Option<T> {
        self.get_value(key)?.parse::<T>().ok()
    }
}

impl Card {
    pub fn from_bytes(card: &[u8; 80]) -> Self {
        let key_bytes = &card[0..8];
        let key = String::from_utf8_lossy(key_bytes).trim().to_string();

        let mut value = None;
        let mut comment = None;

        if card[8] == b'=' {
            // Bytes 9..80 contain the value and comment
            let rest = &card[9..80];

            // Split into exactly 2 parts on the first '/'
            let mut parts = rest.splitn(2, |&b| b == b'/');

            // Handle the value (everything before the '/')
            if let Some(val_bytes) = parts.next() {
                let mut val_str = String::from_utf8_lossy(val_bytes).trim().to_string();

                if !val_str.is_empty() {
                    // If it starts and ends with a single quote, it's a FITS string.
                    if val_str.starts_with('\'') && val_str.ends_with('\'') {
                        // Strip the quotes and any extra padding spaces inside them
                        val_str = val_str[1..val_str.len() - 1].trim().to_string();
                    }
                    // If it's not a string, replace any 'D' with 'E' so Rust can parse it as a float
                    else if val_str.contains('D') {
                        val_str = val_str.replace('D', "E");
                    }
                    // Convert "T" / "F" to Rust-parsable "true" / "false"
                    else if val_str == "T" {
                        val_str = "true".to_string();
                    } else if val_str == "F" {
                        val_str = "false".to_string();
                    }

                    value = Some(val_str);
                }
            }

            // Handle the comment (everything after the '/')
            if let Some(comm_bytes) = parts.next() {
                let comm_str = String::from_utf8_lossy(comm_bytes).trim().to_string();
                if !comm_str.is_empty() {
                    comment = Some(comm_str);
                }
            }
        } else {
            // If there is no '=', it's a HISTORY, COMMENT, or blank card.
            // Bytes 8..80 are all treated as the comment/content.
            let content = String::from_utf8_lossy(&card[8..80]).trim().to_string();
            if !content.is_empty() {
                comment = Some(content);
            }
        }

        Self {
            key,
            value,
            comment,
        }
    }
}
