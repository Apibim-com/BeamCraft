#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Build every platform's application icon from the official logo; run from any directory.

The official `docs/brand/beamcraft-logo.svg` is rendered unchanged and only clipped to a
rounded square on Apple's 1024-unit icon grid (an 824-unit tile with a 100-unit transparent
margin). That one picture is written as the master raster and resampled into the macOS ICNS,
the Windows ICO and the Linux hicolor launcher PNG, so every platform shows the same icon.

Needs Pillow and one SVG renderer: the `cairosvg` module, `rsvg-convert` or ImageMagick's
`magick`.
"""
from io import BytesIO
from pathlib import Path
import shutil
import struct
import subprocess

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / 'assets/icons'
CANVAS = 1024
TILE = 824  # Apple's icon grid: the tile leaves a 100-unit margin on each side.
RADIUS = 185  # Corner radius of the 824-unit tile, as on stock macOS icons.
SCALE = 4  # Supersampling keeps the rounded corners clean at small sizes.


def render_logo(side):
    """The official logo as a `side` x `side` RGBA image."""
    svg = ROOT / 'docs/brand/beamcraft-logo.svg'
    try:
        import cairosvg
        png = cairosvg.svg2png(url=str(svg), output_width=side, output_height=side)
    except ImportError:
        if shutil.which('rsvg-convert'):
            command = ['rsvg-convert', '-w', str(side), '-h', str(side), str(svg)]
        elif shutil.which('magick'):
            command = ['magick', '-background', 'none', '-density', str(96 * side // CANVAS),
                       f'rsvg:{svg}', '-resize', f'{side}x{side}!', 'png:-']
        else:
            raise SystemExit('No SVG renderer: install cairosvg, rsvg-convert or ImageMagick.')
        png = subprocess.run(command, check=True, capture_output=True).stdout
    image = Image.open(BytesIO(png)).convert('RGBA')
    if image.size != (side, side):
        raise SystemExit(f'The renderer produced {image.size}, not {side} x {side}.')
    return image


def main():
    tile = TILE * SCALE
    logo = render_logo(tile)
    mask = Image.new('L', (tile, tile), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, tile - 1, tile - 1), radius=RADIUS * SCALE, fill=255)
    logo.putalpha(mask)
    canvas = Image.new('RGBA', (CANVAS * SCALE, CANVAS * SCALE), (0, 0, 0, 0))
    margin = (CANVAS - TILE) // 2 * SCALE
    canvas.alpha_composite(logo, (margin, margin))
    icon = canvas.resize((CANVAS, CANVAS), Image.Resampling.LANCZOS)

    def sized(side):
        return icon.resize((side, side), Image.Resampling.LANCZOS)

    icon.save(ICONS / 'beamcraft-icon.png', optimize=True)
    sized(256).save(ICONS / 'beamcraft-linux-256.png', optimize=True)
    icon.save(ICONS / 'beamcraft-windows.ico', format='ICO',
              sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    # PNG-backed ICNS representations, including the Retina slots.
    chunks = []
    for kind, side in ((b'icp4', 16), (b'icp5', 32), (b'icp6', 64),
                       (b'ic07', 128), (b'ic08', 256), (b'ic09', 512),
                       (b'ic10', 1024), (b'ic11', 32), (b'ic12', 64),
                       (b'ic13', 256), (b'ic14', 512)):
        png = BytesIO()
        sized(side).save(png, format='PNG', optimize=True)
        payload = png.getvalue()
        chunks.append(kind + struct.pack('>I', len(payload) + 8) + payload)
    body = b''.join(chunks)
    (ICONS / 'BeamCraft.icns').write_bytes(b'icns' + struct.pack('>I', len(body) + 8) + body)


if __name__ == '__main__':
    main()
