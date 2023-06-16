# sameboy-sys

Rust bindings for the SameBoy core

## Compiling

You will need `cppp` on your PATH in order to compile SameBoy.

### Compiling `cppp`

1. Clone the source from `https://github.com/BR903/cppp`
2. If on Windows, or using MinGW:
    - Rename `unixisms.c` to `unixisms-nix.c`
    - Rename `unixisms-win32.c` to `unixisms.c`
    - Make sure you specify your cross compiler as `CC` in your
      make command
3. Call `make` to build.
4. The `cppp` binary should be in your current directory. Install
   it somewhere convenient or add the `cppp` dir to your PATH.

### Compiling `sameboy-sys`

1. Ensure `cppp` is on your PATH.
2. Call `cargo build` to build. 
