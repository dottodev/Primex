"""Generate launcher icons from the supplied PrimeX logo (docs/assets primex logo.jfif).

Usage: python scripts/make_icons.py <source-image>
Reproducible: deterministic resize with Lanczos, fixed dark background.
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw

DARK = (26, 26, 28, 255)  # matches the logo's own background

ROOT = Path(__file__).resolve().parents[1]
RES = ROOT / "app" / "src" / "main" / "res"

DENSITIES = {
    "mdpi": 48,
    "hdpi": 72,
    "xhdpi": 96,
    "xxhdpi": 144,
    "xxxhdpi": 192,
}
FG_DENSITIES = {
    "mdpi": 108,
    "hdpi": 162,
    "xhdpi": 216,
    "xxhdpi": 324,
    "xxxhdpi": 432,
}


def artwork_bbox(source: Image.Image) -> tuple[int, int, int, int]:
    """Bounding box of pixels brighter than the uniform dark background."""
    gray = source.convert("L")
    mask = gray.point(lambda value: 255 if value > 70 else 0)
    box = mask.getbbox()
    if box is None:
        raise SystemExit("no bright artwork found in source image")
    return box


def fit(image: Image.Image, canvas: int, fraction: float) -> Image.Image:
    target = max(1, int(canvas * fraction))
    ratio = min(target / image.width, target / image.height)
    size = (max(1, int(image.width * ratio)), max(1, int(image.height * ratio)))
    return image.resize(size, Image.Resampling.LANCZOS)


def centered(canvas: int, image: Image.Image, bg: tuple[int, int, int, int]) -> Image.Image:
    out = Image.new("RGBA", (canvas, canvas), bg)
    out.alpha_composite(image, ((canvas - image.width) // 2, (canvas - image.height) // 2))
    return out


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    source_path = Path(sys.argv[1])
    source = Image.open(source_path).convert("RGBA")
    box = artwork_bbox(source)
    pad = int(min(source.size) * 0.02)
    box = (
        max(0, box[0] - pad),
        max(0, box[1] - pad),
        min(source.width, box[2] + pad),
        min(source.height, box[3] + pad),
    )
    artwork = source.crop(box)

    # Adaptive foreground: keep the white artwork, make everything else
    # transparent so the system-supplied (dark) background shows through.
    gray = artwork.convert("L")
    alpha = gray.point(lambda value: min(255, max(0, int((value - 40) * 255 / 170))))
    white = Image.new("RGBA", artwork.size, (255, 255, 255, 0))
    white.putalpha(alpha)

    for density, size in DENSITIES.items():
        folder = RES / f"mipmap-{density}"
        folder.mkdir(parents=True, exist_ok=True)
        legacy = centered(size, fit(artwork, size, 0.86), DARK).convert("RGB")
        legacy.save(folder / "ic_launcher.png", optimize=True)

        round_icon = legacy.copy()
        mask = Image.new("L", (size, size), 0)
        ImageDraw.Draw(mask).ellipse((0, 0, size - 1, size - 1), fill=255)
        round_icon.putalpha(mask)
        round_icon.save(folder / "ic_launcher_round.png", optimize=True)

    for density, size in FG_DENSITIES.items():
        folder = RES / f"mipmap-{density}"
        folder.mkdir(parents=True, exist_ok=True)
        foreground = centered(size, fit(white, size, 0.60), (0, 0, 0, 0))
        foreground.save(folder / "ic_launcher_foreground.png", optimize=True)

    (RES / "values" / "ic_launcher_background.xml").write_text(
        '<?xml version="1.0" encoding="utf-8"?>\n'
        '<resources>\n'
        '    <color name="ic_launcher_background">#1A1A1C</color>\n'
        '</resources>\n',
        encoding="utf-8",
    )

    (RES / "mipmap-anydpi-v26").mkdir(parents=True, exist_ok=True)
    (RES / "mipmap-anydpi-v26" / "ic_launcher.xml").write_text(
        '<?xml version="1.0" encoding="utf-8"?>\n'
        '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
        '    <background android:drawable="@color/ic_launcher_background" />\n'
        '    <foreground android:drawable="@mipmap/ic_launcher_foreground" />\n'
        '</adaptive-icon>\n',
        encoding="utf-8",
    )
    (RES / "mipmap-anydpi-v26" / "ic_launcher_round.xml").write_text(
        (RES / "mipmap-anydpi-v26" / "ic_launcher.xml").read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    print(f"icons generated from artwork bbox {box}")


if __name__ == "__main__":
    main()
