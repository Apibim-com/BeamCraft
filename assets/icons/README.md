# Application icons

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

The Windows ICO and Linux PNG use the same composition. The ICO contains 16, 24,
32, 48, 64, 128 and 256 pixel representations. Windows/Linux runtime window icons
load the packaged PNG from `assets/icons/beamcraft-icon.png`; the Windows archive
also includes `BeamCraft.ico`. The committed Windows/Linux rasters are one-off
renders of the same composition from `macos-tile.svg` with pinned CairoSVG 2.8.2
and Pillow 12.3.0; the rendering script is not part of the project and their
provenance is recorded in `ATTRIBUTION.md`.

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

## Ubuntu application menu and Dock

After extracting the Linux release, run `bash install-linux.sh` from its folder.
It installs BeamCraft under `${XDG_DATA_HOME:-$HOME/.local/share}/beamcraft`,
registers `com.apibim.BeamCraft.desktop` in the user applications directory and
installs the application icon in the user's `hicolor` icon theme. No sudo is needed.
Open Ubuntu's Show Applications, search for BeamCraft and launch it. Right-click
its Dock icon and choose Pin to Dash to keep the launcher there. The Linux window's
application ID and X11 window class match the desktop entry, so the running window
groups with that launcher.
Run the installer again from a newer extracted release to update it.

The Linux screenshot workflow launches the installed desktop entry in Ubuntu's
GNOME Shell session and captures both the Dock and Show Applications.
