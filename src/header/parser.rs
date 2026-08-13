use anyhow::{Context, Result, bail};

use crate::constants::{FITS_BLOCKSIZE, FITS_CARDSIZE};
use crate::errors::FitsError;
use crate::header::{BasicHDUInfo, FitsHeader};
use std::fs;

#[deprecated(note = "use mparser::crawl instead")]
pub fn get_hdu<'a>(_data: &'a [u8], _header_n: usize) -> Result<&'a [u8]> {
    unimplemented!("Deprecated in favor of mparser::crawl");
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
