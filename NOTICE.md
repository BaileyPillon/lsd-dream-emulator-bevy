# Attribution and reference inputs

LSD: Dream Emulator and its original content belong to their respective owners.
This is an independent community port with no claim of official affiliation.
The MIT license covers new project code; it does not license original game
content, BIOS images or third-party dependencies.

Reference research comes from
[LSD: Dream Emulator Decompilation](https://github.com/BaileyPillon/lsd-dream-emulator-decompilation),
which retains [cawfee/lsd_decompiled](https://github.com/cawfee/lsd_decompiled)
history and attribution and acknowledges
[FirecatFG/lsddecomp](https://github.com/FirecatFG/lsddecomp) toolchain research.
This new repository has its own Git history. The initial Rust implementation
uses observed behavior and format facts; it does not import the C source tree.

[Bevy](https://github.com/bevyengine/bevy) is MIT/Apache-2.0 licensed.
Dependency licenses remain their own; Cargo.lock records the selected versions.

The SPU Gaussian hardware coefficient table in `lsd-core` was transcribed from
[psxe](https://github.com/allkern/psxe/blob/8459c63ed78c703f46e3c51daa65a2fdf96d02e6/psx/dev/spu.c),
Copyright (c) 2024 Allkern (Lisandro Alarcón), under the MIT license. Its license
is retained in [licenses/psxe-MIT.txt](licenses/psxe-MIT.txt). The Rust decoder,
pitch arithmetic and interpolation implementation are new project code.
