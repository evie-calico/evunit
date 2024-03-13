# Changelog

## 1.3.4 (2024-3-13)

New:
- `stack` option added for pushing values to the stack.
- Memory assignment can now be done to arbitrary integer values.

Changed:
- Strings containing non-ASCII characters will generate errors.

## 1.3.3 (2024-3-10)

Fixed:
- Dump not correctly showing contents of memory

## 1.3.2 (2024-3-3)

New:
- Added emulation of HRAM

Fixed:
- `--dump-dir` flag now creates the directory instead of failing.

## 1.3.1 (2023-10-10)

- Enabled additional Clippy warnings.
- Move library usage example from `src/bin/` to `examples/`.
- Deprecated `read_symfile` in favor of `open_symfile`

Updated dependencies:
- clap: 3.2.17 -> 4.2.1
- toml: 0.5.9 -> 0.8.1


## 1.3.0 (2023-6-11)

Improvements to library interface.

## 1.2.0 (2023-6-7)

Added a Rust library interface.

## 1.1.1 (2023-1-29)

Fixed:
- `crash` and `exit` now accept a single label once again.

## 1.1.0 (2023-1-19)

New:
- Input files may now use `-` to read from standard input.
- Added `caller` and `exit` configuration options.
- `crash` now accepts an array of addresses.
- Memory can be assigned to and tested in unit tests.

## 1.0.0 (2022-10-2)

Initial release
