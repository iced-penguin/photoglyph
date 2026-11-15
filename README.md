# photoglyph

Photoglyph is for photographers who want to describe and organize their photos without changing the image files themselves. It treats metadata as a separate, editable layer: export it to an XMP sidecar beside the image, then inspect or update that sidecar while leaving the original untouched. Because the sidecar is an ordinary file, it can be kept, backed up, and moved along with the photo.

## Quick start

With ExifTool installed, export a RAW photo's metadata, inspect it, and add a description:

```sh
pg export photo.NEF
pg show photo
pg set photo "A quiet morning by the lake"
```

`pg --help` lists the commands, and `pg <command> --help` shows details for a specific command. Export accepts only the RAW extensions listed below, detected from the filename extension alone; file contents are not inspected before ExifTool runs. Metadata is written to the `.xmp` sidecar; the image file is not modified.

## Requirements

- [ExifTool](https://exiftool.org/)
- Rust 1.99.1 or later is required to install photoglyph with Cargo.

## Features

- `export`: Exports metadata from a supported RAW image to a companion `.xmp` sidecar. Existing sidecars are not overwritten unless `--force` is specified.
- `show`: Displays the default comment, common properties, all XMP properties, or a selected property. Missing properties are omitted; empty properties are shown without a value. `--verbose` uses a more detailed output format.
- `set`: Sets a property value.
- `clear`: Sets a property to an empty value while keeping it present.
- `edit`: Opens the sidecar in the editor configured by `$VISUAL` or `$EDITOR`.

### Export extensions

Export eligibility is determined solely from the input filename extension, case-insensitively. Photoglyph does not inspect file contents to determine whether a file is RAW; ExifTool reports any unreadable or unsupported content errors.
Files sharing a basename are allowed, including multiple supported RAW files; exporting any of them writes to the same `<basename>.xmp` path. An existing sidecar is preserved unless `--force` is specified.

| Format or manufacturer | Extensions |
| --- | --- |
| Adobe DNG | `dng` |
| Canon | `cr2`, `cr3`, `crw` |
| Nikon | `nef`, `nrw` |
| Sony | `arw`, `arq`, `sr2`, `srf` |
| Fujifilm | `raf` |
| Olympus / OM System | `orf` |
| Panasonic | `rw2` |
| Leica | `rwl` |
| Pentax | `pef` |
| Hasselblad | `3fr`, `fff` |
| Phase One | `iiq` |
| Leaf | `mos` |
| Mamiya | `mef` |
| Minolta | `mrw` |
| Samsung | `srw` |
| Sigma | `x3f` |
| Kodak | `dcr`, `k25`, `kdc` |
| Epson | `erf` |
| GoPro | `gpr` |
| Generic / legacy | `raw` |

The `show`, `set`, `clear`, and `edit` commands accept an `.xmp` file or a basename (for example, `photo` resolves to `photo.xmp`). They work directly with the sidecar and do not require a source image or verify which image produced it. Direct image paths remain invalid targets for these commands.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for environment setup, build, run, and test instructions.
