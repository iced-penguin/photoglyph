# photoglyph

Photoglyph is for photographers who want to describe and organize their photos without changing the image files themselves. It treats metadata as a separate, editable layer: export it to an XMP sidecar beside the image, then inspect or update that sidecar while leaving the original untouched. Because the sidecar is an ordinary file, it can be kept, backed up, and moved along with the photo.

## Requirements

- ExifTool
- Rust >= 1.85

To install ExifTool, for example on macOS: `brew install exiftool`

## Features

- `export`: Exports image metadata to a companion `.xmp` sidecar. Existing sidecars are not overwritten unless forced.
- `show`: Displays the default comment, common properties, all XMP properties, or a selected property. Missing properties are omitted; empty properties are shown without a value. `--verbose` uses a more detailed output format.
- `set`: Sets a property value.
- `clear`: Sets a property to an empty value while keeping it present.
- `edit`: Opens the sidecar in the editor configured by `$VISUAL` or `$EDITOR`.

The `show`, `set`, `clear`, and `edit` commands accept an `.xmp` file or a basename (for example, `photo` resolves to `photo.xmp`). Image file paths are not accepted; use `pg export` to create a sidecar first.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for environment setup, build, run, and test instructions.
