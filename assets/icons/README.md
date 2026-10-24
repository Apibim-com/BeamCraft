# Application icons

The original `beamcraft-1024.png` and everything under `docs/brand/` remain unchanged.
The macOS composition places the official SVG's I-beam group, without changing its
paths or colours, on a contributor-original continuous-corner enamel tile. The
824-unit tile has 100-unit canvas insets, a subtle rim, directional lighting and
soft shadows. No grid competes with the mark at small Dock sizes.

Rebuild from any directory with Python 3.12, CairoSVG 2.8.2 (system Cairo required)
and Pillow 12.3.0:

```sh
python3 -m pip install CairoSVG==2.8.2 Pillow==12.3.0
./packaging/generate-icon.sh
python3 -m unittest discover -s packaging -p 'test_*.py'
```

The Windows ICO and Linux PNG use that same composition. The ICO contains 16, 24,
32, 48, 64, 128 and 256 pixel representations. The Windows archive includes
`BeamCraft.ico`; Windows/Linux runtime window icons load the PNG from
`assets/icons/beamcraft-icon.png` in the package. The SVG tile and compositor are
the editable sources. The generator reads
`docs/brand/beamcraft-logo.svg` without writing it. It supersamples at 2048px and
writes `beamcraft-macos-1024.png`, `beamcraft-linux-1024.png`,
`beamcraft-windows.ico`, plus PNG-backed ICNS representations for all
standard/Retina sizes, including 16, 32, 64, 128, 256, 512 and 1024px. Pinned
renderer versions in the same environment produce identical bytes; Cairo/zlib
versions can affect encoding across operating systems.

`packaging/package.sh` installs the ICNS for Finder/Spotlight and the same
composition as `Contents/Resources/beamcraft-icon.png` for eframe's runtime Dock
icon. Windows/Linux and the unbundled fallback are unchanged. Attribution and
retained trademark terms are in `ATTRIBUTION.md`.

## Native acceptance still required

On macOS, build the distribution binaries and run the packaging command described
in `packaging/package.sh`. Extract the archive, open BeamCraft.app and check the
running Dock icon alongside stock apps at normal and magnified sizes. Quit and
check the bundle icon in Finder and Spotlight. Capture actual screenshots of all
three surfaces, including the running Dock; generated previews do not establish
native acceptance. Verify transparent corners, comparable tile scale, clean small
sizes and matching running/closed icons. Do not mark issue #59 complete before
this check and the required workspace checks pass.
