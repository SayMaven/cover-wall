#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Cover Wall Studio — Pure Rust FLAC Cover Extractor & 4K Wallpaper Studio.

mod extractor;
mod i18n;
mod renderer;
mod theme;
mod ui;

use std::sync::Arc;

use eframe::egui;

const LOGO_SVG: &[u8] = include_bytes!("../assets/logo.svg");

/// Rasterize the embedded `assets/logo.svg` into unpremultiplied RGBA bytes.
pub fn render_logo_rgba(size: u32) -> Option<Vec<u8>> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(LOGO_SVG, &opt).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    let svg_size = tree.size();
    let sx = size as f32 / svg_size.width().max(1.0);
    let sy = size as f32 / svg_size.height().max(1.0);
    let transform = resvg::tiny_skia::Transform::from_scale(sx, sy);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut rgba = pixmap.take();
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as f32 / 255.0;
        if a > 0.0 && px[3] < 255 {
            px[0] = ((px[0] as f32 / a).round()).clamp(0.0, 255.0) as u8;
            px[1] = ((px[1] as f32 / a).round()).clamp(0.0, 255.0) as u8;
            px[2] = ((px[2] as f32 / a).round()).clamp(0.0, 255.0) as u8;
        }
    }
    Some(rgba)
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([920.0, 620.0])
        .with_title("Cover Wall Studio");

    if let Some(rgba) = render_logo_rgba(256) {
        viewport = viewport.with_icon(Arc::new(egui::IconData {
            rgba,
            width: 256,
            height: 256,
        }));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Cover Wall Studio",
        options,
        Box::new(|cc| Ok(Box::new(ui::CoverWallApp::new(cc)))),
    )
}
