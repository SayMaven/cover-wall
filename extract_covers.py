"""Extract unique album covers from FLAC files using FFmpeg/FFprobe."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

INVALID_FILENAME_CHARS = re.compile(r'[<>:"/\\|?*\x00-\x1f]')

CODEC_TO_EXT = {
    "mjpeg": ".jpg",
    "jpeg": ".jpg",
    "png": ".png",
    "bmp": ".bmp",
    "webp": ".webp",
}


def sanitize_filename(name: str) -> str:
    """Sanitize a string so it is safe as a Windows filename."""
    cleaned = INVALID_FILENAME_CHARS.sub("_", name).strip().rstrip(".")
    return cleaned or "Unknown Album"


def probe_flac(flac_path: Path) -> dict:
    """Return ffprobe JSON metadata (format + streams) for a FLAC file."""
    cmd = [
        "ffprobe",
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        str(flac_path),
    ]
    res = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", check=False)
    if res.returncode != 0 or not res.stdout.strip():
        return {}
    return json.loads(res.stdout)


def get_album_name(probe_data: dict, flac_path: Path, root_dir: Path) -> str:
    """Extract case-insensitive ALBUM tag, falling back to folder/file name."""
    tags = probe_data.get("format", {}).get("tags", {})
    for key, value in tags.items():
        if key.lower() == "album" and str(value).strip():
            return str(value).strip()

    # Fallback if ALBUM tag is missing
    rel = flac_path.relative_to(root_dir)
    if len(rel.parts) > 1:
        parent = flac_path.parent
        if re.match(r"^(cd|disc)\s*\d+$", parent.name, re.IGNORECASE) and parent.parent != root_dir:
            return parent.parent.name
        return parent.name
    return flac_path.stem


def get_embedded_cover_stream(probe_data: dict) -> tuple[int | None, str]:
    """Return (stream_index, extension) for the first embedded image stream."""
    for stream in probe_data.get("streams", []):
        if stream.get("codec_type") == "video":
            codec = str(stream.get("codec_name", "")).lower()
            ext = CODEC_TO_EXT.get(codec, ".jpg")
            return stream.get("index"), ext
    return None, ".jpg"


def find_folder_cover(flac_path: Path, root_dir: Path) -> Path | None:
    """Look for an existing cover.* or folder.* image in the album folder."""
    search_dirs = [flac_path.parent]
    if flac_path.parent != root_dir and flac_path.parent.parent != root_dir:
        search_dirs.append(flac_path.parent.parent)

    for folder in search_dirs:
        if not folder.exists():
            continue
        for candidate in folder.iterdir():
            if candidate.is_file() and candidate.stem.lower() in {"cover", "folder", "front"}:
                if candidate.suffix.lower() in {".jpg", ".jpeg", ".png", ".webp", ".bmp"}:
                    return candidate
    return None


def extract_embedded_cover(flac_path: Path, stream_index: int, out_path: Path) -> bool:
    """Extract embedded cover stream from a FLAC file using ffmpeg."""
    cmd = [
        "ffmpeg",
        "-y",
        "-v",
        "error",
        "-i",
        str(flac_path),
        "-map",
        f"0:{stream_index}",
        "-an",
        "-vcodec",
        "copy",
        str(out_path),
    ]
    res = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", check=False)
    return res.returncode == 0 and out_path.exists() and out_path.stat().st_size > 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Extract album covers from FLAC files.")
    parser.add_argument("source", type=Path, help="Root music directory containing FLAC files")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=Path("covers"),
        help="Output directory for extracted covers (default: ./covers)",
    )
    args = parser.parse_args()

    source_dir: Path = args.source
    output_dir: Path = args.output

    if not source_dir.exists():
        print(f"Source directory not found: {source_dir}", file=sys.stderr)
        return 1

    output_dir.mkdir(parents=True, exist_ok=True)

    flac_files = sorted(source_dir.rglob("*.flac"))
    if not flac_files:
        print(f"No .flac files found in {source_dir}")
        return 0

    extracted_albums: dict[str, Path] = {}
    missing_covers: list[str] = []

    for flac_path in flac_files:
        probe_data = probe_flac(flac_path)
        album_name = get_album_name(probe_data, flac_path, source_dir)
        safe_name = sanitize_filename(album_name)

        if safe_name.lower() in extracted_albums:
            continue

        stream_idx, ext = get_embedded_cover_stream(probe_data)
        if stream_idx is not None:
            out_path = output_dir / f"{safe_name}{ext}"
            if extract_embedded_cover(flac_path, stream_idx, out_path):
                extracted_albums[safe_name.lower()] = out_path
                print(f"[EMBEDDED] {album_name} -> {out_path.name}")
                continue

        folder_cover = find_folder_cover(flac_path, source_dir)
        if folder_cover is not None:
            out_path = output_dir / f"{safe_name}{folder_cover.suffix.lower()}"
            shutil.copy2(folder_cover, out_path)
            extracted_albums[safe_name.lower()] = out_path
            print(f"[FOLDER]   {album_name} -> {out_path.name}")
            continue

        missing_covers.append(f"{album_name} ({flac_path})")

    print(f"\nDone: {len(extracted_albums)} album covers saved to {output_dir.resolve()}")
    if missing_covers:
        print(f"No cover found for {len(missing_covers)} item(s):")
        for item in missing_covers:
            print(f"  - {item}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
