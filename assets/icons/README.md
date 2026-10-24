# macOS application icon

The original `beamcraft-1024.png` and everything under `docs/brand/` remain unchanged.
The macOS composition places the official SVG's I-beam group, without changing its
paths or colours, on a contributor-original continuous-corner enamel tile. The
824-unit tile has 100-unit canvas insets, a subtle rim, directional lighting and
soft shadows. No grid competes with the mark at small Dock sizes.

`macos-tile.svg` is the editable design source of the composition;
`beamcraft-macos-1024.png` is its checked-in 1024px raster. Rebuild the ICNS from
the raster with the tools included with macOS:

```sh
./packaging/generate-icon.sh
```

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
sizes and matching running/closed icons.
