use thiserror::Error;

use crate::constants::FITS_CARDSIZE;

#[derive(Error, Debug)]
pub enum FitsError {
    #[error("Invalid card length: expected {FITS_CARDSIZE}, got {0}")]
    InvalidCardLength(usize),

    #[error("Invalid header card: missing '=' separator")]
    MissingEqualSign,

    #[error("Failed to parse header value: {0}")]
    ParseError(#[from] std::num::ParseFloatError),

    #[error("Invalid UTF-8 sequence")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Unexpected end of file")]
    UnexpectedEof,

    #[error("Invalid BITPIX value: {0} . Allowed values are 8, 16, 32, -32, 64, -64")]
    InvalidBitpix(i32),

    #[error("Missing BITPIX value")]
    MissingBitpix,

    #[error("Invalid data length")]
    InvalidLength,

    #[error("Error parsing fits file: the file does not start with 'SIMPLE  '")]
    InvalidFile,

    #[error("Error parsing fits file: attempted to find invalid block index")]
    InvalidBlock,

    #[error("Error parsing fits file: could not file END block")]
    NoEndBlock,

    #[error("Error reading fits file: could not read file {0}")]
    ReadError(#[from] std::io::Error),

    #[error("3D Datacubes are not supported")]
    NAxis3Error,
}
