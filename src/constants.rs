/*
Global constants
*/

pub const FITS_BLOCKSIZE: usize = 2880;
pub const FITS_CARDSIZE: usize = 80;
pub const END_CARD: &[u8; 80] = b"END                                                                             ";

pub const DEFAULT_FIND_DATA_MAX_RECURSIONS: usize = 128;

pub const SCALING_LINEAR_PERCENTILE_UPPER: f64 = 0.98;
pub const SCALING_LINEAR_PERCENTILE_LOWER: f64 = 0.02;
pub const SCALING_LOGARITHMIC_PERCENTILE_UPPER: f64 = 0.99;
pub const SCALING_LOGARITHMIC_PERCENTILE_LOWER: f64 = 0.01;
pub const SCALING_SQUAREROOT_PERCENTILE_UPPER: f64 = 0.98;
pub const SCALING_SQUAREROOT_PERCENTILE_LOWER: f64 = 0.02;
pub const SCALING_ASINH_PERCENTILE_UPPER: f64 = 0.98;
pub const SCALING_ASINH_PERCENTILE_LOWER: f64 = 0.02;