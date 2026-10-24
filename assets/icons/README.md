# Windows and Linux icons

`beamcraft-linux-1024.png` is used for the Windows and Linux runtime window icon.
`beamcraft-windows.ico` contains 16, 24, 32, 48, 64, 128 and 256 pixel images
and is included in the Windows archive. Both assets follow the composition of the
latest macOS icon in the BeamCraft fork while leaving the official logo untouched.

The packaging script places the PNG at `assets/icons/beamcraft-icon.png`, where
the UI loads it next to the executable. The macOS bundle icon remains independent.
