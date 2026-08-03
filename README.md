Simple FITS viewer built in Rust using egui/wgpu!

## Motivation
I am building this to learn Rust, it is not feature complete and probably never will be.

## Known limitations
* Does not support FITS tables
* Currently no astrometry can be performed, this is purely a visual tool
* No WCS support
* Requires a GPU supporting wGPU
* Only numeric headers are interpreted
* Only a single HDU can be loaded at one time
* No compressed file support
* Cubes can only be navigated using a flat slice index
