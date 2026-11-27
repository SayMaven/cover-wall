//! Cover Wall Studio — Pure Rust FLAC Cover Extractor & 4K Wallpaper Studio.

mod extractor;
mod renderer;
mod theme;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([920.0, 620.0])
            .with_title("Cover Wall Studio"),
        ..Default::default()
    };

    eframe::run_native(
        "Cover Wall Studio",
        options,
        Box::new(|cc| Ok(Box::new(ui::CoverWallApp::new(cc)))),
    )
}
