# Application icons

BeamCraft has one application icon on every platform: the official logo
`docs/brand/beamcraft-logo.svg`, rendered unchanged and clipped to a rounded square on
Apple's 1024-unit icon grid (an 824-unit tile, corner radius 185, with a 100-unit transparent
margin). The macOS Dock, Finder and Spotlight, the Windows taskbar, the Ubuntu launcher and the
running window all show this same picture.

| File | Use |
| --- | --- |
| `beamcraft-icon.png` | 1024 px master raster; the runtime window and Dock icon |
| `BeamCraft.icns` | macOS bundle icon, 16 to 1024 px including the Retina slots |
| `beamcraft-windows.ico` | Windows icon, 16 to 256 px; shipped as `BeamCraft.ico` |
| `beamcraft-linux-256.png` | Linux hicolor launcher icon |

All four are generated, never edited by hand. After a change to the official logo, rebuild them
on any OS with Pillow and an SVG renderer (`cairosvg`, `rsvg-convert` or ImageMagick):

```sh
python3 packaging/generate-icon.py
```

`beamcraft-1024.png` is the unmodified official raster. It is not an application icon: it is the
logo on the start window and start page. It and everything under `docs/brand/` are never
modified.

## Packaging

`packaging/package.sh` installs the ICNS for Finder/Spotlight and copies `beamcraft-icon.png`
into the bundle as `Contents/Resources/beamcraft-icon.png`, which the UI applies at run time to
the Dock (eframe shows the process icon while the app runs). Windows and Linux archives carry
`assets/icons/beamcraft-icon.png` for the window icon; the Windows archive also carries
`BeamCraft.ico`, the Linux archive `install-linux.sh` and the launcher PNG.

## Ubuntu application menu and Dock

After extracting the Linux release, run `bash install-linux.sh` from its folder. It installs
BeamCraft under `${XDG_DATA_HOME:-$HOME/.local/share}/beamcraft`, registers
`com.apibim.BeamCraft.desktop` in the user applications directory and installs the icon in the
user's `hicolor` icon theme; no sudo is needed. The window's Wayland app ID and X11 window class
match the desktop entry, so the running window groups with the launcher. Run the installer again
from a newer release to update it.

## Native acceptance still required

On macOS, package the app, open BeamCraft.app and compare the running Dock icon with stock apps
at normal and magnified sizes; quit and check Finder and Spotlight. On Windows, check the
taskbar; on Ubuntu, the Dock and Show Applications after `install-linux.sh`. Generated previews
do not establish native acceptance.
