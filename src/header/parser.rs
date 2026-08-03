use anyhow::{Context, Result, bail};

use crate::constants::{FITS_BLOCKSIZE, FITS_CARDSIZE};
use crate::errors::FitsError;
use crate::header::{BasicFitsInfo, FitsHeader};
use std::fs;

pub fn get_hdu<'a>(data: &'a [u8], header_n: usize) -> Result<&'a [u8]> {
    let mut current_ptr = 0;

    for i in 0..=header_n {
        if current_ptr >= data.len() {
            bail!("Data pointer out of bounds for remaining data (could not find specified hdu)");
        }

        let remaining = &data[current_ptr..];

        let info = BasicFitsInfo::from_block(remaining)?;

        let hdu_end_block_idx = find_end_block(remaining)?;
        let header_size = (hdu_end_block_idx + 1) * FITS_BLOCKSIZE;
        let header_bytes = &remaining[..header_size];

        let bitpix: isize = info
            .bitpix
            .value
            .context(format!("Could not find BITPIX header in hdu {}", header_n))?
            as isize;
        let naxis: usize = info.naxis;

        let num_pixels: usize = match naxis {
            0 => 0,
            _ => info.axes.iter().product(),
        };

        let pcount = find_card(header_bytes, "PCOUNT")
            .and_then(|c| FitsHeader::new(c).ok()?.value)
            .unwrap_or(0.0) as usize;

        let gcount = find_card(header_bytes, "GCOUNT")
            .and_then(|c| FitsHeader::new(c).ok()?.value)
            .unwrap_or(1.0) as usize;

        let bytes_per_element = (bitpix.abs() as usize) / 8;
        let data_size_raw = bytes_per_element * gcount * (pcount + num_pixels);

        let data_size_padded =
            ((data_size_raw + FITS_BLOCKSIZE - 1) / FITS_BLOCKSIZE) * FITS_BLOCKSIZE;

        if i == header_n {
            let total_requested_size = header_size + data_size_raw;
            if current_ptr + total_requested_size > data.len() {
                bail!(
                    "Data pointer out of bounds for remaining data (could not find specified hdu)"
                );
            }

            return Ok(&data[current_ptr..current_ptr + total_requested_size]);
        }

        current_ptr += header_size + data_size_padded;
    }

    bail!("Could not find specified header");
}

pub fn find_end_block(data_slice: &[u8]) -> Result<usize, FitsError> {
    for (i, block) in data_slice.chunks_exact(FITS_BLOCKSIZE).enumerate() {
        if find_card(block, "END").is_some() {
            return Ok(i);
        }
    }
    Err(FitsError::NoEndBlock)
}

pub fn find_card<'a>(block: &'a [u8], name: &str) -> Option<&'a [u8]> {
    let name_b = name.as_bytes();
    block.chunks(FITS_CARDSIZE).find(|chunk| {
        let keyword = &chunk[0..8];
        keyword.starts_with(name_b) && keyword[name_b.len()..].iter().all(|&b| b == b' ')
    })
}

pub fn read_file(filepath: &str) -> Result<Vec<u8>, FitsError> {
    let data = fs::read(filepath)?;
    if !check_file_validity(&data) {
        return Err(FitsError::InvalidFile);
    }
    Ok(data)
}

fn check_file_validity(data: &[u8]) -> bool {
    data.len() >= 8 && &data[0..8] == b"SIMPLE  "
}
