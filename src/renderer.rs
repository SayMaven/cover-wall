//! Parallel 4K wallpaper renderer with live preview support.

use std::fs;
use std::path::{Path, PathBuf};

use fast_image_resize::images::Image as FirImage;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{Rgba, RgbaImage, imageops};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use thiserror::Error;

const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "bmp"];

/// Supported wallpaper layout models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutStyle {
    Terasering,
    Diagonal,
    GridTipis,
    GridLebar,
}

impl LayoutStyle {
    pub const ALL: [Self; 4] = [
        Self::Terasering,
        Self::Diagonal,
        Self::GridTipis,
        Self::GridLebar,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Terasering => "Terasering",
            Self::Diagonal => "Diagonal",
            Self::GridTipis => "Grid Tipis",
            Self::GridLebar => "Grid Lebar",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Terasering => "terasering",
            Self::Diagonal => "diagonal",
            Self::GridTipis => "grid_tipis",
            Self::GridLebar => "grid_lebar",
        }
    }
}

/// Output resolution preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspectPreset {
    Landscape4K,
    Portrait4K,
    Ultrawide,
    Custom,
}

impl AspectPreset {
    pub const ALL: [Self; 4] = [
        Self::Landscape4K,
        Self::Portrait4K,
        Self::Ultrawide,
        Self::Custom,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Landscape4K => "4K 16:9 (3840x2160)",
            Self::Portrait4K => "4K 9:16 (2160x3840)",
            Self::Ultrawide => "Ultrawide (3440x1440)",
            Self::Custom => "Custom",
        }
    }

    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Landscape4K => (3840, 2160),
            Self::Portrait4K => (2160, 3840),
            Self::Ultrawide => (3440, 1440),
            Self::Custom => (3840, 2160),
        }
    }
}

/// Background rendering mode for all wallpaper layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundMode {
    Solid,
    BlurredCovers,
    Transparent,
}

impl BackgroundMode {
    pub const ALL: [Self; 3] = [Self::Solid, Self::BlurredCovers, Self::Transparent];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid",
            Self::BlurredCovers => "Blurred",
            Self::Transparent => "Transparent",
        }
    }
}

/// Curated OKLCH-derived background palette swatches (`name`, `[r, g, b]`).
pub const BG_PALETTE_SWATCHES: [(&str, [u8; 3]); 6] = [
    ("Obsidian", [14, 14, 18]),
    ("Midnight", [16, 22, 34]),
    ("Deep Plum", [26, 16, 30]),
    ("Forest Ink", [14, 24, 20]),
    ("Warm Slate", [28, 26, 24]),
    ("Chalk", [242, 242, 246]),
];

/// Parameters controlling wallpaper layout and rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderParams {
    pub style: LayoutStyle,
    pub width: u32,
    pub height: u32,
    pub cols: u32,
    pub gap: u32,
    pub angle: f32,
    pub corner_radius: u32,
    pub shadow_blur: f32,
    pub shuffle: bool,
    pub seed: u64,
    pub bg_mode: BackgroundMode,
    pub bg_color: [u8; 3],
    pub bg_tint_opacity: f32,
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            style: LayoutStyle::Terasering,
            width: 3840,
            height: 2160,
            cols: 9,
            gap: 14,
            angle: 12.0,
            corner_radius: 14,
            shadow_blur: 10.0,
            shuffle: false,
            seed: 42,
            bg_mode: BackgroundMode::Solid,
            bg_color: [14, 14, 18],
            bg_tint_opacity: 0.72,
        }
    }
}

/// A loaded square-cropped cover image ready for rendering.
#[derive(Debug, Clone)]
pub struct CoverImage {
    pub path: PathBuf,
    pub title: String,
    pub square: RgbaImage,
}

/// Errors returned by cover loading or wallpaper rendering.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("No covers selected or available")]
    EmptyCovers,
    #[error("Failed to create output directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to save wallpaper image: {0}")]
    ImageSave(#[from] image::ImageError),
}

/// Scan a directory for cover image paths sorted by filename.
pub fn scan_cover_paths(covers_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(covers_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    paths.sort();
    paths
}

/// Load and center-crop cover images to 1:1 squares in parallel.
pub fn load_covers_parallel(paths: &[PathBuf]) -> Vec<CoverImage> {
    paths
        .par_iter()
        .filter_map(|path| {
            let img = image::open(path).ok()?.to_rgba8();
            let (w, h) = img.dimensions();
            let side = w.min(h);
            if side == 0 {
                return None;
            }
            let x = (w - side) / 2;
            let y = (h - side) / 2;
            let square = imageops::crop_imm(&img, x, y, side, side).to_image();
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Cover")
                .to_owned();
            Some(CoverImage {
                path: path.clone(),
                title,
                square,
            })
        })
        .collect()
}

/// Resize an `RgbaImage` quickly using `fast_image_resize`.
pub fn resize_rgba(src: &RgbaImage, dst_w: u32, dst_h: u32) -> RgbaImage {
    let dst_w = dst_w.max(1);
    let dst_h = dst_h.max(1);
    if src.width() == dst_w && src.height() == dst_h {
        return src.clone();
    }

    let Ok(src_fir) = FirImage::from_vec_u8(src.width(), src.height(), src.as_raw().clone(), PixelType::U8x4)
    else {
        return imageops::resize(src, dst_w, dst_h, imageops::FilterType::Lanczos3);
    };

    let mut dst_fir = FirImage::new(dst_w, dst_h, PixelType::U8x4);
    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    if resizer.resize(&src_fir, &mut dst_fir, &options).is_err() {
        return imageops::resize(src, dst_w, dst_h, imageops::FilterType::Lanczos3);
    }

    RgbaImage::from_raw(dst_w, dst_h, dst_fir.into_vec())
        .unwrap_or_else(|| imageops::resize(src, dst_w, dst_h, imageops::FilterType::Lanczos3))
}

fn apply_rounded_corners(img: &mut RgbaImage, radius: u32) {
    if radius == 0 {
        return;
    }
    let (w, h) = img.dimensions();
    let r = (radius as f32).min(w as f32 * 0.5).min(h as f32 * 0.5);
    let r_sq = r * r;

    for (y, row) in img.rows_mut().enumerate() {
        let yf = y as f32 + 0.5;
        let dy = if yf < r {
            r - yf
        } else if yf > h as f32 - r {
            yf - (h as f32 - r)
        } else {
            continue;
        };

        for (x, px) in row.enumerate() {
            let xf = x as f32 + 0.5;
            let dx = if xf < r {
                r - xf
            } else if xf > w as f32 - r {
                xf - (w as f32 - r)
            } else {
                continue;
            };

            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r + 0.5 {
                px[3] = 0;
            } else if dist > r - 0.5 && dist * dist > r_sq - r {
                let cov = (r + 0.5 - dist).clamp(0.0, 1.0);
                px[3] = (px[3] as f32 * cov).round() as u8;
            }
        }
    }
}

fn make_shadow(size: u32, radius: u32, blur: f32, offset: (i32, i32)) -> (RgbaImage, i32) {
    let blur_i = blur.ceil().max(0.0) as i32;
    let pad = blur_i * 2;
    let canvas_size = size + (pad * 2) as u32;
    let mut shadow = RgbaImage::from_pixel(canvas_size, canvas_size, Rgba([0, 0, 0, 0]));

    let start_x = (pad + offset.0).max(0) as u32;
    let start_y = (pad + offset.1).max(0) as u32;
    let end_x = (start_x + size).min(canvas_size);
    let end_y = (start_y + size).min(canvas_size);

    for y in start_y..end_y {
        for x in start_x..end_x {
            shadow.put_pixel(x, y, Rgba([0, 0, 0, 150]));
        }
    }

    let mut rect = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 150]));
    apply_rounded_corners(&mut rect, radius);
    shadow.fill(0);
    imageops::overlay(&mut shadow, &rect, (pad + offset.0) as i64, (pad + offset.1) as i64);

    let blurred = if blur > 0.25 {
        imageops::blur(&shadow, blur)
    } else {
        shadow
    };
    (blurred, pad)
}

fn blend_rgba_clipped(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    imageops::overlay(dst, src, x as i64, y as i64);
}

fn create_blurred_background(covers: &[&RgbaImage], spec: &LayoutSpec, rows: u32) -> RgbaImage {
    let width = spec.width;
    let height = spec.height;
    let cols = spec.cols.max(1);
    let rows = rows.max(1);
    let tint_color = spec.bg_color;
    let mut bg = RgbaImage::from_pixel(width, height, Rgba([tint_color[0], tint_color[1], tint_color[2], 255]));
    let cell_w = width / cols + 1;
    let cell_h = height / rows + 1;

    let mut idx = 0_usize;
    for r in 0..rows {
        for c in 0..cols {
            let cover = covers[idx % covers.len()];
            let tile = resize_rgba(cover, cell_w, cell_h);
            imageops::overlay(&mut bg, &tile, (c * cell_w) as i64, (r * cell_h) as i64);
            idx += 1;
        }
    }

    let blur_sigma = (60.0 * spec.scale_factor).max(4.0);
    let mut blurred = imageops::blur(&bg, blur_sigma);
    let alpha = spec.bg_tint_opacity.clamp(0.0, 1.0);
    let tr = tint_color[0] as f32;
    let tg = tint_color[1] as f32;
    let tb = tint_color[2] as f32;
    for px in blurred.pixels_mut() {
        px[0] = (px[0] as f32 * (1.0 - alpha) + tr * alpha).round() as u8;
        px[1] = (px[1] as f32 * (1.0 - alpha) + tg * alpha).round() as u8;
        px[2] = (px[2] as f32 * (1.0 - alpha) + tb * alpha).round() as u8;
        px[3] = 255;
    }
    blurred
}

fn create_canvas_background(covers: &[&RgbaImage], spec: &LayoutSpec, rows: u32) -> RgbaImage {
    match spec.bg_mode {
        BackgroundMode::Solid => RgbaImage::from_pixel(
            spec.width,
            spec.height,
            Rgba([spec.bg_color[0], spec.bg_color[1], spec.bg_color[2], 255]),
        ),
        BackgroundMode::BlurredCovers => create_blurred_background(covers, spec, rows),
        BackgroundMode::Transparent => RgbaImage::from_pixel(spec.width, spec.height, Rgba([0, 0, 0, 0])),
    }
}

fn prepare_tiles(covers: &[&RgbaImage], tile_size: u32, corner_radius: u32) -> Vec<RgbaImage> {
    covers
        .par_iter()
        .map(|cover| {
            let mut tile = resize_rgba(cover, tile_size, tile_size);
            apply_rounded_corners(&mut tile, corner_radius);
            tile
        })
        .collect()
}

struct LayoutSpec {
    width: u32,
    height: u32,
    cols: u32,
    gap: u32,
    margin_x: u32,
    margin_y: u32,
    corner_radius: u32,
    shadow_blur: f32,
    scale_factor: f32,
    angle: f32,
    tile_size: u32,
    bg_mode: BackgroundMode,
    bg_color: [u8; 3],
    bg_tint_opacity: f32,
}

fn build_grid(covers: &[&RgbaImage], spec: &LayoutSpec) -> RgbaImage {
    let cols = spec.cols.max(1);
    let rows = (covers.len() as u32).div_ceil(cols).max(1);

    let avail_w = spec.width.saturating_sub(2 * spec.margin_x + (cols - 1) * spec.gap);
    let avail_h = spec.height.saturating_sub(2 * spec.margin_y + (rows - 1) * spec.gap);
    let tile_size = (avail_w / cols).min(avail_h / rows).max(8);

    let mut canvas = create_canvas_background(covers, spec, rows);
    let total_grid_h = rows * tile_size + (rows - 1) * spec.gap;
    let start_y = (spec.height as i32 - total_grid_h as i32) / 2;

    let tiles = prepare_tiles(covers, tile_size, spec.corner_radius);
    let (shadow, shadow_pad) = make_shadow(
        tile_size,
        spec.corner_radius,
        spec.shadow_blur,
        (0, (6.0 * spec.scale_factor).round() as i32),
    );

    for row_idx in 0..rows {
        let start = (row_idx * cols) as usize;
        let end = ((row_idx + 1) * cols).min(covers.len() as u32) as usize;
        if start >= end {
            break;
        }
        let row_count = (end - start) as u32;
        let row_w = row_count * tile_size + (row_count - 1) * spec.gap;
        let start_x = (spec.width as i32 - row_w as i32) / 2;
        let y = start_y + (row_idx * (tile_size + spec.gap)) as i32;

        for (col_idx, tile_idx) in (start..end).enumerate() {
            let x = start_x + (col_idx as u32 * (tile_size + spec.gap)) as i32;
            blend_rgba_clipped(&mut canvas, &shadow, x - shadow_pad, y - shadow_pad);
            blend_rgba_clipped(&mut canvas, &tiles[tile_idx], x, y);
        }
    }

    canvas
}

fn build_staggered_terrace(covers: &[&RgbaImage], spec: &LayoutSpec) -> RgbaImage {
    let cols = spec.cols.max(1);
    let step_x = ((spec.width + spec.gap) as f32 / cols as f32).ceil() as u32;
    let tile_size = step_x.saturating_sub(spec.gap).max(8);
    let step_y = tile_size + spec.gap;

    let mut rows = (spec.height as f32 / step_y as f32).ceil() as u32;
    let mut total_h = rows * tile_size + rows.saturating_sub(1) * spec.gap;
    let mut start_y = (spec.height as i32 - total_h as i32) / 2;

    // Avoid thin edge slivers/gaps at top and bottom when a row boundary lands near y=0
    if (tile_size as i32 + start_y) < (tile_size as f32 * 0.25) as i32 {
        rows += 1;
        total_h = rows * tile_size + rows.saturating_sub(1) * spec.gap;
        start_y = (spec.height as i32 - total_h as i32) / 2;
    }

    let total_w = cols * tile_size + cols.saturating_sub(1) * spec.gap;
    let base_x = (spec.width as i32 - total_w as i32) / 2;

    let mut canvas = create_canvas_background(covers, spec, rows);
    let tiles = prepare_tiles(covers, tile_size, spec.corner_radius);
    let (shadow, shadow_pad) = make_shadow(
        tile_size,
        spec.corner_radius,
        spec.shadow_blur,
        (0, (6.0 * spec.scale_factor).round() as i32),
    );

    let mut idx = 0_usize;
    for row_idx in 0..rows {
        let is_odd = row_idx % 2 == 1;
        let offset_x = if is_odd {
            base_x - (step_x as i32 / 2)
        } else {
            base_x
        };
        let row_cols = if is_odd { cols + 1 } else { cols };
        let y = start_y + (row_idx * step_y) as i32;

        for col_idx in 0..row_cols {
            let x = offset_x + (col_idx * step_x) as i32;
            let tile = &tiles[idx % tiles.len()];
            idx += 1;
            blend_rgba_clipped(&mut canvas, &shadow, x - shadow_pad, y - shadow_pad);
            blend_rgba_clipped(&mut canvas, tile, x, y);
        }
    }

    canvas
}

fn rotate_center_crop(board: &RgbaImage, target_w: u32, target_h: u32, angle_deg: f32) -> RgbaImage {
    let rad = (-angle_deg).to_radians();
    let cos_a = rad.cos();
    let sin_a = rad.sin();
    let bw = board.width() as f32;
    let bh = board.height() as f32;
    let bcx = bw * 0.5;
    let bcy = bh * 0.5;
    let tcx = target_w as f32 * 0.5;
    let tcy = target_h as f32 * 0.5;

    let mut out = RgbaImage::from_pixel(target_w, target_h, Rgba([0, 0, 0, 0]));
    let raw_src = board.as_raw();
    let src_w = board.width() as i32;
    let src_h = board.height() as i32;

    out.as_mut()
        .par_chunks_mut((target_w * 4) as usize)
        .enumerate()
        .for_each(|(ty, row_bytes)| {
            let dy = ty as f32 - tcy;
            for tx in 0..target_w {
                let dx = tx as f32 - tcx;
                let sx = bcx + dx * cos_a - dy * sin_a;
                let sy = bcy + dx * sin_a + dy * cos_a;

                let x0 = sx.floor() as i32;
                let y0 = sy.floor() as i32;
                if x0 < 0 || y0 < 0 || x0 + 1 >= src_w || y0 + 1 >= src_h {
                    continue;
                }

                let fx = sx - x0 as f32;
                let fy = sy - y0 as f32;
                let w00 = (1.0 - fx) * (1.0 - fy);
                let w10 = fx * (1.0 - fy);
                let w01 = (1.0 - fx) * fy;
                let w11 = fx * fy;

                let i00 = ((y0 * src_w + x0) * 4) as usize;
                let i10 = i00 + 4;
                let i01 = i00 + (src_w as usize * 4);
                let i11 = i01 + 4;

                let a00 = raw_src[i00 + 3] as f32 * w00;
                let a10 = raw_src[i10 + 3] as f32 * w10;
                let a01 = raw_src[i01 + 3] as f32 * w01;
                let a11 = raw_src[i11 + 3] as f32 * w11;
                let a_sum = a00 + a10 + a01 + a11;

                let dst_i = (tx * 4) as usize;
                if a_sum > 0.001 {
                    for c in 0..3 {
                        let val = (raw_src[i00 + c] as f32 * a00
                            + raw_src[i10 + c] as f32 * a10
                            + raw_src[i01 + c] as f32 * a01
                            + raw_src[i11 + c] as f32 * a11)
                            / a_sum;
                        row_bytes[dst_i + c] = val.round().clamp(0.0, 255.0) as u8;
                    }
                    row_bytes[dst_i + 3] = a_sum.round().clamp(0.0, 255.0) as u8;
                }
            }
        });

    out
}

fn build_diagonal_terrace(covers: &[&RgbaImage], spec: &LayoutSpec) -> RgbaImage {
    let tile_size = spec.tile_size.max(24);
    let diag = ((spec.width as f32).hypot(spec.height as f32) as u32) + tile_size * 2;
    let step = tile_size + spec.gap;
    let cols = (diag as f32 / step as f32).ceil() as u32 + 2;
    let rows = (diag as f32 / step as f32).ceil() as u32 + 2;

    let mut board = RgbaImage::from_pixel(diag, diag, Rgba([0, 0, 0, 0]));
    let tiles = prepare_tiles(covers, tile_size, spec.corner_radius);
    let (shadow, shadow_pad) = make_shadow(
        tile_size,
        spec.corner_radius,
        spec.shadow_blur,
        (0, (6.0 * spec.scale_factor).round() as i32),
    );

    let mut idx = 0_usize;
    for row_idx in 0..rows {
        let offset_x = if row_idx % 2 == 1 {
            -(step as i32 / 2)
        } else {
            0
        };
        let y = (row_idx * step) as i32 - (tile_size as i32 / 2);
        for col_idx in 0..cols {
            let x = offset_x + (col_idx * step) as i32 - (tile_size as i32 / 2);
            let tile = &tiles[idx % tiles.len()];
            idx += 1;
            blend_rgba_clipped(&mut board, &shadow, x - shadow_pad, y - shadow_pad);
            blend_rgba_clipped(&mut board, tile, x, y);
        }
    }

    let rotated = rotate_center_crop(&board, spec.width, spec.height, spec.angle);
    if spec.bg_mode == BackgroundMode::Transparent {
        return rotated;
    }
    let bg_rows = (spec.height as f32 / step as f32).ceil().max(1.0) as u32;
    let mut canvas = create_canvas_background(covers, spec, bg_rows);
    imageops::overlay(&mut canvas, &rotated, 0, 0);
    canvas
}

/// Render a wallpaper (full resolution or scaled preview) from loaded covers.
pub fn render_wallpaper(
    covers: &[CoverImage],
    params: &RenderParams,
    preview_max_dim: Option<u32>,
) -> Result<RgbaImage, RenderError> {
    if covers.is_empty() {
        return Err(RenderError::EmptyCovers);
    }

    let mut ordered: Vec<&RgbaImage> = covers.iter().map(|c| &c.square).collect();
    if params.shuffle {
        let mut rng = ChaCha8Rng::seed_from_u64(params.seed);
        ordered.shuffle(&mut rng);
    }

    let full_w = params.width.max(320);
    let full_h = params.height.max(320);
    let scale = match preview_max_dim {
        Some(max_dim) if full_w.max(full_h) > max_dim => max_dim as f32 / full_w.max(full_h) as f32,
        _ => 1.0,
    };

    let w = ((full_w as f32 * scale).round() as u32).max(120);
    let h = ((full_h as f32 * scale).round() as u32).max(120);
    let gap = ((params.gap as f32 * scale).round() as u32).max(1);
    let radius = (params.corner_radius as f32 * scale).round() as u32;
    let shadow_blur = (params.shadow_blur * scale).max(0.0);

    let img = match params.style {
        LayoutStyle::Terasering => build_staggered_terrace(
            &ordered,
            &LayoutSpec {
                width: w,
                height: h,
                cols: params.cols,
                gap,
                margin_x: 0,
                margin_y: 0,
                corner_radius: radius,
                shadow_blur,
                scale_factor: scale,
                angle: 0.0,
                tile_size: 0,
                bg_mode: params.bg_mode,
                bg_color: params.bg_color,
                bg_tint_opacity: params.bg_tint_opacity,
            },
        ),
        LayoutStyle::Diagonal => {
            let tile_size = (((full_w as f32 / params.cols.max(1) as f32) - params.gap as f32) * scale)
                .round()
                .max(24.0) as u32;
            build_diagonal_terrace(
                &ordered,
                &LayoutSpec {
                    width: w,
                    height: h,
                    cols: params.cols,
                    gap,
                    margin_x: 0,
                    margin_y: 0,
                    corner_radius: radius,
                    shadow_blur,
                    scale_factor: scale,
                    angle: params.angle,
                    tile_size,
                    bg_mode: params.bg_mode,
                    bg_color: params.bg_color,
                    bg_tint_opacity: params.bg_tint_opacity,
                },
            )
        }
        LayoutStyle::GridTipis => {
            let (mx, my) = if w >= h { (80, 60) } else { (60, 100) };
            build_grid(
                &ordered,
                &LayoutSpec {
                    width: w,
                    height: h,
                    cols: params.cols,
                    gap,
                    margin_x: (mx as f32 * scale).round() as u32,
                    margin_y: (my as f32 * scale).round() as u32,
                    corner_radius: radius,
                    shadow_blur,
                    scale_factor: scale,
                    angle: 0.0,
                    tile_size: 0,
                    bg_mode: params.bg_mode,
                    bg_color: params.bg_color,
                    bg_tint_opacity: params.bg_tint_opacity,
                },
            )
        }
        LayoutStyle::GridLebar => {
            let (mx, my) = if w >= h { (180, 120) } else { (120, 200) };
            build_grid(
                &ordered,
                &LayoutSpec {
                    width: w,
                    height: h,
                    cols: params.cols,
                    gap: gap.max((36.0 * scale).round() as u32),
                    margin_x: (mx as f32 * scale).round() as u32,
                    margin_y: (my as f32 * scale).round() as u32,
                    corner_radius: radius.max((20.0 * scale).round() as u32),
                    shadow_blur: shadow_blur.max(18.0 * scale),
                    scale_factor: scale,
                    angle: 0.0,
                    tile_size: 0,
                    bg_mode: params.bg_mode,
                    bg_color: params.bg_color,
                    bg_tint_opacity: params.bg_tint_opacity,
                },
            )
        }
    };

    Ok(img)
}

/// Resolve an output path without ever overwriting an existing file.
pub fn resolve_safe_output_path(dir: &Path, base_name: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{base_name}.{ext}"));
    if !first.exists() {
        return first;
    }
    let mut version = 2_u32;
    loop {
        let candidate = dir.join(format!("{base_name}_v{version}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        version += 1;
    }
}

/// Render and save a full-resolution PNG wallpaper safely.
pub fn export_wallpaper_png(
    covers: &[CoverImage],
    params: &RenderParams,
    output_dir: &Path,
    prefix: &str,
) -> Result<PathBuf, RenderError> {
    fs::create_dir_all(output_dir)?;
    let img = render_wallpaper(covers, params, None)?;
    let base_name = format!("{}_{}x{}_{}", prefix, params.width, params.height, params.style.slug());
    let out_path = resolve_safe_output_path(output_dir, &base_name, "png");
    img.save(&out_path)?;
    Ok(out_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_wallpaper_produces_exact_dimensions() {
        let dummy = CoverImage {
            path: PathBuf::from("a.png"),
            title: "A".to_owned(),
            square: RgbaImage::from_pixel(64, 64, Rgba([200, 100, 50, 255])),
        };
        let params = RenderParams {
            style: LayoutStyle::Terasering,
            width: 640,
            height: 360,
            cols: 4,
            gap: 8,
            ..Default::default()
        };
        let out = render_wallpaper(&[dummy], &params, None);
        assert!(out.is_ok());
        if let Ok(img) = out {
            assert_eq!(img.dimensions(), (640, 360));
        }
    }

    #[test]
    fn transparent_background_preserves_alpha_in_gaps() {
        let dummy = CoverImage {
            path: PathBuf::from("a.png"),
            title: "A".to_owned(),
            square: RgbaImage::from_pixel(64, 64, Rgba([200, 100, 50, 255])),
        };
        let params = RenderParams {
            style: LayoutStyle::GridLebar,
            width: 400,
            height: 300,
            cols: 2,
            gap: 24,
            bg_mode: BackgroundMode::Transparent,
            shadow_blur: 0.0,
            ..Default::default()
        };
        let out = render_wallpaper(&[dummy], &params, None);
        assert!(out.is_ok());
        if let Ok(img) = out {
            assert_eq!(img.get_pixel(0, 0)[3], 0);
        }
    }

    #[test]
    fn resolve_safe_output_path_avoids_overwriting() {
        let temp = std::env::temp_dir().join("cover_wall_test_safe_path");
        let _ = fs::create_dir_all(&temp);
        let first = temp.join("sample.png");
        let _ = fs::write(&first, b"test");
        let resolved = resolve_safe_output_path(&temp, "sample", "png");
        assert_ne!(resolved, first);
        let _ = fs::remove_dir_all(&temp);
    }
}
