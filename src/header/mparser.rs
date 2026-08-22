use crate::constants::{END_CARD, FITS_BLOCKSIZE, FITS_CARDSIZE};
use crate::header::{BasicHDUInfo, Card, FitsHeader, HDU};
use anyhow::{Context, Result};
use indexmap::IndexMap;
use log::debug;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub fn crawl(file: &mut File) -> Result<IndexMap<usize, HDU>> {
    let mut buf = [0u8; FITS_BLOCKSIZE];
    let mut hdus: IndexMap<usize, HDU> = IndexMap::new();

    let mut cards: Vec<Card> = Vec::new();
    let mut current_hdu_idx: usize = 0;

    loop {
        if file.read(&mut buf).context("Error reading file")? == 0 {
            break;
        }

        debug!("Reading HDU {} block", current_hdu_idx);

        for card_slice in buf.chunks_exact(FITS_CARDSIZE) {
            let card_array: &[u8; FITS_CARDSIZE] = card_slice
                .try_into()
                .expect("FITS block was not perfectly divisible by CARDSIZE");

            cards.push(Card::from_bytes(card_array));

            if card_array == END_CARD {
                debug!("Hit END card for HDU {}", current_hdu_idx);

                let header = FitsHeader { cards };

                let basic_info = BasicHDUInfo::from_header(&header)?;

                let data_n_blocks = calculate_data_n_blocks(&basic_info);
                let data_byte_length = data_n_blocks * FITS_BLOCKSIZE;

                let data_offset = file.stream_position()?;

                let mmap_data = if data_byte_length > 0 {
                    let mmap = unsafe {
                        memmap2::MmapOptions::new()
                            .offset(data_offset)
                            .len(data_byte_length)
                            .map(&*file)?
                    };
                    Some(mmap)
                } else {
                    None
                };

                let hdu = HDU {
                    hdu_index: current_hdu_idx,
                    n_blocks: data_n_blocks,
                    total_size: data_byte_length,
                    data: mmap_data,
                    header,
                    basic_info,
                };

                hdus.insert(current_hdu_idx, hdu);

                // Seek the file pointer past the data payload to the start of the next HDU
                if data_byte_length > 0 {
                    file.seek(SeekFrom::Current(data_byte_length as i64))?;
                }

                current_hdu_idx += 1;
                cards = Vec::new(); // Re-instantiate a fresh vector for the next HDU

                break;
            }
        }
    }

    Ok(hdus)
}

fn calculate_data_n_blocks(basic_info: &BasicHDUInfo) -> usize {
    let n_bytes = basic_info.n_bytes;
    n_bytes.div_ceil(FITS_BLOCKSIZE)
}
