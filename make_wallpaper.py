"""Generate 4K (16:9 and 9:16) album cover wallpapers in multiple layout styles."""

from __future__ import annotations

import argparse
import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageOps

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".bmp"}
ALL_STYLES = ("terasering", "diagonal", "grid_tipis", "grid_lebar")


def load_covers(covers_dir: Path, shuffle: bool = False, seed: int | None = None) -> list[Image.Image]:
    """Load and center-crop all cover images to 1:1 squares."""
    paths = sorted(
        p for p in covers_dir.iterdir()
        if p.is_file() and p.suffix.lower() in IMAGE_EXTENSIONS
    )
    if shuffle:
        rng = random.Random(seed)
        rng.shuffle(paths)

    images: list[Image.Image] = []
    for path in paths:
        with Image.open(path) as img:
            rgb = img.convert("RGB")
            side = min(rgb.size)
            square = ImageOps.fit(rgb, (side, side), method=Image.Resampling.LANCZOS)
            images.append(square)
    return images


def create_blurred_background(
    covers: list[Image.Image],
    width: int,
    height: int,
    cols: int,
    rows: int,
) -> Image.Image:
    """Create a dark blurred mosaic background from the covers."""
    bg = Image.new("RGB", (width, height), (14, 14, 18))
    cell_w = width // cols + 1
    cell_h = height // rows + 1

    idx = 0
    for r in range(rows):
        for c in range(cols):
            cover = covers[idx % len(covers)]
            tile = ImageOps.fit(cover, (cell_w, cell_h), method=Image.Resampling.BILINEAR)
            bg.paste(tile, (c * cell_w, r * cell_h))
            idx += 1

    bg = bg.filter(ImageFilter.GaussianBlur(radius=60))
    overlay = Image.new("RGBA", (width, height), (10, 10, 14, 185))
    return Image.alpha_composite(bg.convert("RGBA"), overlay)


def round_corners(img: Image.Image, radius: int) -> Image.Image:
    """Apply rounded corners to an RGB/RGBA image."""
    rgba = img.convert("RGBA")
    mask = Image.new("L", rgba.size, 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, rgba.width, rgba.height), radius=radius, fill=255)
    rgba.putalpha(mask)
    return rgba


def make_shadow(size: int, radius: int, blur: int = 12, offset: tuple[int, int] = (0, 6)) -> Image.Image:
    """Create a soft drop shadow layer for a square tile."""
    pad = blur * 2
    shadow = Image.new("RGBA", (size + pad * 2, size + pad * 2), (0, 0, 0, 0))
    draw = ImageDraw.Draw(shadow)
    draw.rounded_rectangle(
        (pad + offset[0], pad + offset[1], pad + size + offset[0], pad + size + offset[1]),
        radius=radius,
        fill=(0, 0, 0, 150),
    )
    return shadow.filter(ImageFilter.GaussianBlur(radius=blur))


def build_grid(
    covers: list[Image.Image],
    width: int,
    height: int,
    cols: int,
    gap: int = 14,
    margin_x: int = 80,
    margin_y: int = 60,
    corner_radius: int = 12,
    shadow_blur: int = 10,
) -> Image.Image:
    """Straight grid with configurable gap and centered last row."""
    rows = (len(covers) + cols - 1) // cols
    max_tile_w = (width - 2 * margin_x - (cols - 1) * gap) // cols
    max_tile_h = (height - 2 * margin_y - (rows - 1) * gap) // rows
    tile_size = min(max_tile_w, max_tile_h)

    canvas = create_blurred_background(covers, width, height, cols, rows)
    total_grid_h = rows * tile_size + (rows - 1) * gap
    start_y = (height - total_grid_h) // 2

    shadow_pad = shadow_blur * 2
    shadow_template = make_shadow(tile_size, corner_radius, blur=shadow_blur)

    for row_idx in range(rows):
        row_items = covers[row_idx * cols : (row_idx + 1) * cols]
        row_count = len(row_items)
        row_w = row_count * tile_size + (row_count - 1) * gap
        start_x = (width - row_w) // 2
        y = start_y + row_idx * (tile_size + gap)

        for col_idx, cover in enumerate(row_items):
            x = start_x + col_idx * (tile_size + gap)
            resized = cover.resize((tile_size, tile_size), Image.Resampling.LANCZOS)
            rounded = round_corners(resized, corner_radius)
            canvas.alpha_composite(shadow_template, (x - shadow_pad, y - shadow_pad))
            canvas.alpha_composite(rounded, (x, y))

    return canvas.convert("RGB")


def build_staggered_terrace(
    covers: list[Image.Image],
    width: int,
    height: int,
    cols: int,
    gap: int = 14,
    corner_radius: int = 14,
) -> Image.Image:
    """Staggered brick/terrace layout with thin gaps filling the full frame."""
    step_x = math.ceil((width + gap) / cols)
    tile_size = step_x - gap
    step_y = tile_size + gap

    rows = math.ceil(height / step_y)
    total_h = rows * tile_size + (rows - 1) * gap
    start_y = (height - total_h) // 2
    # Avoid thin edge slivers/gaps at top and bottom when a row boundary lands near y=0
    if tile_size + start_y < tile_size * 0.25:
        rows += 1
        total_h = rows * tile_size + (rows - 1) * gap
        start_y = (height - total_h) // 2

    total_w = cols * tile_size + (cols - 1) * gap
    base_x = (width - total_w) // 2

    canvas = Image.new("RGBA", (width, height), (14, 14, 18, 255))

    shadow_blur = 10
    shadow_pad = shadow_blur * 2
    shadow_template = make_shadow(tile_size, corner_radius, blur=shadow_blur)

    idx = 0
    for row_idx in range(rows):
        offset_x = base_x - (step_x // 2) if (row_idx % 2 == 1) else base_x
        row_cols = cols + 1 if (row_idx % 2 == 1) else cols
        y = start_y + row_idx * step_y

        for col_idx in range(row_cols):
            x = offset_x + col_idx * step_x
            cover = covers[idx % len(covers)]
            idx += 1

            resized = cover.resize((tile_size, tile_size), Image.Resampling.LANCZOS)
            rounded = round_corners(resized, corner_radius)

            sx, sy = x - shadow_pad, y - shadow_pad
            if sx + shadow_template.width > 0 and sx < width and sy + shadow_template.height > 0 and sy < height:
                canvas.alpha_composite(shadow_template, (max(0, sx), max(0, sy)), (max(0, -sx), max(0, -sy)))
            if x + tile_size > 0 and x < width and y + tile_size > 0 and y < height:
                canvas.alpha_composite(rounded, (max(0, x), max(0, y)), (max(0, -x), max(0, -y)))

    return canvas.convert("RGB")


def build_diagonal_terrace(
    covers: list[Image.Image],
    width: int,
    height: int,
    tile_size: int,
    gap: int = 14,
    angle: float = 12.0,
    corner_radius: int = 14,
) -> Image.Image:
    """Tilted staggered mosaic (diagonal terrace) filling the full 4K canvas."""
    diag = int(math.hypot(width, height)) + tile_size * 2
    step = tile_size + gap
    cols = math.ceil(diag / step) + 2
    rows = math.ceil(diag / step) + 2

    board = Image.new("RGBA", (diag, diag), (14, 14, 18, 255))
    shadow_blur = 10
    shadow_pad = shadow_blur * 2
    shadow_template = make_shadow(tile_size, corner_radius, blur=shadow_blur)

    idx = 0
    for row_idx in range(rows):
        offset_x = -(step // 2) if (row_idx % 2 == 1) else 0
        y = row_idx * step - tile_size // 2
        for col_idx in range(cols):
            x = offset_x + col_idx * step - tile_size // 2
            cover = covers[idx % len(covers)]
            idx += 1

            resized = cover.resize((tile_size, tile_size), Image.Resampling.LANCZOS)
            rounded = round_corners(resized, corner_radius)

            sx, sy = x - shadow_pad, y - shadow_pad
            if sx + shadow_template.width > 0 and sx < diag and sy + shadow_template.height > 0 and sy < diag:
                board.alpha_composite(shadow_template, (max(0, sx), max(0, sy)), (max(0, -sx), max(0, -sy)))
            if x + tile_size > 0 and x < diag and y + tile_size > 0 and y < diag:
                board.alpha_composite(rounded, (max(0, x), max(0, y)), (max(0, -x), max(0, -y)))

    rotated = board.rotate(angle, resample=Image.Resampling.BICUBIC, expand=False)
    cx, cy = rotated.width // 2, rotated.height // 2
    left = cx - width // 2
    top = cy - height // 2
    cropped = rotated.crop((left, top, left + width, top + height))
    return cropped.convert("RGB")


def render_style(style: str, covers: list[Image.Image], gap: int, angle: float) -> list[tuple[str, Image.Image]]:
    """Render 16:9 and 9:16 4K wallpapers for the requested style."""
    cols_16_9 = 9 if len(covers) >= 32 else 7
    if style == "terasering":
        return [
            ("16x9_terasering", build_staggered_terrace(covers, 3840, 2160, cols=cols_16_9, gap=gap)),
            ("9x16_terasering", build_staggered_terrace(covers, 2160, 3840, cols=4, gap=gap)),
        ]
    if style == "diagonal":
        return [
            ("16x9_diagonal", build_diagonal_terrace(covers, 3840, 2160, tile_size=520, gap=gap, angle=angle)),
            ("9x16_diagonal", build_diagonal_terrace(covers, 2160, 3840, tile_size=500, gap=gap, angle=angle)),
        ]
    if style == "grid_tipis":
        return [
            ("16x9_grid_tipis", build_grid(covers, 3840, 2160, cols=cols_16_9, gap=gap, margin_x=80, margin_y=60, corner_radius=12)),
            ("9x16_grid_tipis", build_grid(covers, 2160, 3840, cols=4, gap=gap, margin_x=60, margin_y=100, corner_radius=12)),
        ]
    if style == "grid_lebar":
        return [
            ("16x9_grid_lebar", build_grid(covers, 3840, 2160, cols=cols_16_9, gap=36, margin_x=180, margin_y=120, corner_radius=20, shadow_blur=18)),
            ("9x16_grid_lebar", build_grid(covers, 2160, 3840, cols=4, gap=36, margin_x=120, margin_y=200, corner_radius=22, shadow_blur=18)),
        ]
    raise ValueError(f"Unknown style: {style}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Create 4K 16:9 and 9:16 album cover wallpapers.")
    parser.add_argument("-i", "--input", type=Path, default=Path("covers"), help="Covers directory (default: ./covers)")
    parser.add_argument("-o", "--output", type=Path, default=Path("wallpapers"), help="Output directory (default: ./wallpapers)")
    parser.add_argument("-p", "--prefix", default="wallpaper", help="Output filename prefix (default: wallpaper)")
    parser.add_argument(
        "-s",
        "--style",
        nargs="+",
        choices=[*ALL_STYLES, "all"],
        default=["all"],
        help="Layout style(s) to generate (default: all)",
    )
    parser.add_argument("--gap", type=int, default=14, help="Gap in pixels for terasering/diagonal/grid_tipis (default: 14)")
    parser.add_argument("--angle", type=float, default=12.0, help="Tilt angle in degrees for diagonal style (default: 12)")
    parser.add_argument("--shuffle", action="store_true", help="Randomize cover order")
    parser.add_argument("--seed", type=int, default=None, help="Random seed when using --shuffle")
    args = parser.parse_args()

    covers = load_covers(args.input, shuffle=args.shuffle, seed=args.seed)
    if not covers:
        print(f"No cover images found in {args.input}")
        return 1

    args.output.mkdir(parents=True, exist_ok=True)
    selected_styles = list(ALL_STYLES) if "all" in args.style else args.style

    for style in selected_styles:
        for suffix, image in render_style(style, covers, gap=args.gap, angle=args.angle):
            out_path = args.output / f"{args.prefix}_{suffix}.png"
            image.save(out_path, format="PNG", optimize=True)
            print(f"Saved: {out_path.resolve()} ({image.size[0]}x{image.size[1]})")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
