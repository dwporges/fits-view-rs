Simple FITS viewer built in Rust using egui/wgpu!

## Arguments
-f, --fname <path> (required)
Path to FITS file

-s, --scaling-method <int> (optional)
Which image scaling method to default to:
0 => Linear
1 => Logarithmic
2 => Squareroot
3 => ASINH

--hdu <int> (optional)
Which HDU to read

--slice <int> (optional)
Which slice of a multi-dimensional data to default to

--use-glow (optional)
Use Glow backend instead of WGPU. This should not be used unless testing


## Motivation
I am building this to learn Rust, it is not feature complete and probably never will be.

## Known limitations
* Does not support FITS tables
* Currently no astrometry can be performed, this is purely a visual tool
* No WCS support
* Requires a GPU supporting wGPU
* ~~Only numeric headers are interpreted~~
* ~~Only a single HDU can be loaded at one time~~
* No compressed file support
* Cubes can only be navigated using a flat slice index
