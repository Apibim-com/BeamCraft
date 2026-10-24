# Application icons

The original `beamcraft-1024.png` and everything under `docs/brand/` remain unchanged.
The macOS composition places the official SVG's I-beam group, without changing its
paths or colours, on a contributor-original continuous-corner enamel tile. The
824-unit tile has 100-unit canvas insets, a muted teal finish, restrained rim,
directional lighting and soft shadows. The mark keeps clear space from the tile
edge so its silhouette remains distinct at small Dock sizes.

The Windows ICO and Linux PNG use the same composition. The ICO contains 16, 24,
32, 48, 64, 128 and 256 pixel representations. Windows/Linux runtime window icons
load the packaged PNG from `assets/icons/beamcraft-icon.png`; the Windows archive
also includes `BeamCraft.ico`.

Rebuild from any directory with Python 3.12, CairoSVG 2.8.2 (system Cairo required)
and Pillow 12.3.0:

```sh
python3 -m pip install CairoSVG==2.8.2 Pillow==12.3.0
./packaging/generate-icon.sh
```

The SVG tile and compositor are the editable sources. The generator reads
`docs/brand/beamcraft-logo.svg` without writing it. It supersamples at 2048px and
writes the macOS and Linux PNGs, the Windows ICO, and PNG-backed ICNS
representations for standard/Retina sizes, including 16, 32, 64, 128, 256, 512
and 1024px. Pinned renderer versions in the same environment produce identical
bytes; Cairo/zlib versions can affect encoding across operating systems.

`packaging/package.sh` installs the ICNS for Finder/Spotlight and the same
composition as `Contents/Resources/beamcraft-icon.png` for eframe's runtime Dock
icon. Native macOS acceptance still requires checking the running Dock, Finder
and Spotlight icons at normal and magnified sizes, including transparent corners
and small sizes. Attribution and retained trademark terms are in
`ATTRIBUTION.md`.
