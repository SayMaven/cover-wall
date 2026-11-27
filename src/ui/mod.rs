//! Top-level egui application state and navigation.

pub mod extractor_view;
pub mod studio_view;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use eframe::egui::{self, ColorImage, RichText, TextureHandle, TextureOptions};
use image::RgbaImage;

use crate::extractor::{ExtractConfig, ExtractProgress, ExtractSummary, run_extraction};
use crate::renderer::{
    AspectPreset, CoverImage, LayoutStyle, RenderParams, export_wallpaper_png,
    load_covers_parallel, render_wallpaper, resize_rgba, scan_cover_paths,
};
use crate::theme::ThemeTokens;

/// Active workspace mode in the top bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Studio,
    Extractor,
}

/// A cover item displayed in the studio library strip.
pub struct StudioCover {
    pub data: CoverImage,
    pub selected: bool,
    pub thumb: Option<TextureHandle>,
}

enum WorkerEvent {
    ExtractProgress(ExtractProgress),
    ExtractDone(Result<ExtractSummary, String>),
    CoversLoaded(Vec<CoverImage>),
    PreviewReady {
        generation: u64,
        image: RgbaImage,
    },
    ExportDone(Result<Vec<PathBuf>, String>),
}

/// Main application state for Cover Wall Studio.
pub struct CoverWallApp {
    pub tokens: ThemeTokens,
    pub mode: AppMode,
    pub banner: Option<(bool, String)>, // (is_error, message)
    pub logo_texture: Option<TextureHandle>,

    // Extractor state
    pub extract_source: String,
    pub extract_output: String,
    pub extract_running: bool,
    pub extract_progress: Option<ExtractProgress>,
    pub extract_summary: Option<ExtractSummary>,
    extract_cancel: Arc<AtomicBool>,

    // Studio state
    pub covers_dir: String,
    pub output_dir: String,
    pub export_prefix: String,
    pub covers: Vec<StudioCover>,
    pub covers_loading: bool,
    pub aspect_preset: AspectPreset,
    pub params: RenderParams,
    pub preview_texture: Option<TextureHandle>,
    pub preview_rendering: bool,
    pub exporting: bool,
    pub show_cover_strip: bool,
    preview_generation: u64,
    last_preview_params: Option<(RenderParams, Vec<PathBuf>)>,

    rx: Receiver<WorkerEvent>,
    tx: mpsc::Sender<WorkerEvent>,
}

impl CoverWallApp {
    /// Initialize the application with OKLCH styling and default paths.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let tokens = ThemeTokens::default();
        tokens.apply_to_egui(&cc.egui_ctx);
        tokens.install_cjk_font_fallback(&cc.egui_ctx);

        let (tx, rx) = mpsc::channel();

        let default_covers = if PathBuf::from("covers/Takahashi Yu").exists() {
            "covers/Takahashi Yu".to_owned()
        } else {
            "covers".to_owned()
        };
        let default_wallpapers = if PathBuf::from("wallpapers/Takahashi Yu").exists() {
            "wallpapers/Takahashi Yu".to_owned()
        } else {
            "wallpapers".to_owned()
        };

        let logo_texture = crate::render_logo_rgba(64).map(|rgba| {
            let color_img = ColorImage::from_rgba_unmultiplied([64, 64], &rgba);
            cc.egui_ctx
                .load_texture("app_logo", color_img, TextureOptions::LINEAR)
        });

        let mut app = Self {
            tokens,
            mode: AppMode::Studio,
            banner: None,
            logo_texture,
            extract_source: String::new(),
            extract_output: default_covers.clone(),
            extract_running: false,
            extract_progress: None,
            extract_summary: None,
            extract_cancel: Arc::new(AtomicBool::new(false)),
            covers_dir: default_covers,
            output_dir: default_wallpapers,
            export_prefix: "wallpaper".to_owned(),
            covers: Vec::new(),
            covers_loading: false,
            aspect_preset: AspectPreset::Landscape4K,
            params: RenderParams::default(),
            preview_texture: None,
            preview_rendering: false,
            exporting: false,
            show_cover_strip: true,
            preview_generation: 0,
            last_preview_params: None,
            rx,
            tx,
        };

        if PathBuf::from(&app.covers_dir).exists() {
            app.reload_covers(cc.egui_ctx.clone());
        }

        app
    }

    pub fn reload_covers(&mut self, ctx: egui::Context) {
        let dir = PathBuf::from(&self.covers_dir);
        if !dir.exists() {
            self.banner = Some((true, format!("Covers folder not found: {}", dir.display())));
            return;
        }
        self.covers_loading = true;
        self.banner = None;
        let tx = self.tx.clone();
        thread::spawn(move || {
            let paths = scan_cover_paths(&dir);
            let loaded = load_covers_parallel(&paths);
            let _ = tx.send(WorkerEvent::CoversLoaded(loaded));
            ctx.request_repaint();
        });
    }

    pub fn start_extraction(&mut self, ctx: egui::Context) {
        if self.extract_running {
            return;
        }
        let source_dir = PathBuf::from(self.extract_source.trim());
        let output_dir = PathBuf::from(self.extract_output.trim());
        if self.extract_source.trim().is_empty() {
            self.banner = Some((true, "Please select a source music folder first.".to_owned()));
            return;
        }

        self.extract_running = true;
        self.extract_progress = None;
        self.extract_summary = None;
        self.banner = None;
        self.extract_cancel.store(false, Ordering::Relaxed);

        let cancel = Arc::clone(&self.extract_cancel);
        let tx = self.tx.clone();
        let config = ExtractConfig {
            source_dir,
            output_dir,
        };

        thread::spawn(move || {
            let result = run_extraction(&config, &cancel, |progress| {
                let _ = tx.send(WorkerEvent::ExtractProgress(progress));
                ctx.request_repaint();
            })
            .map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::ExtractDone(result));
            ctx.request_repaint();
        });
    }

    pub fn cancel_extraction(&self) {
        self.extract_cancel.store(true, Ordering::Relaxed);
    }

    pub fn selected_covers(&self) -> Vec<CoverImage> {
        self.covers
            .iter()
            .filter(|c| c.selected)
            .map(|c| c.data.clone())
            .collect()
    }

    pub fn trigger_preview_if_needed(&mut self, ctx: &egui::Context) {
        if self.covers_loading || self.covers.is_empty() {
            return;
        }
        let selected_paths: Vec<PathBuf> = self
            .covers
            .iter()
            .filter(|c| c.selected)
            .map(|c| c.data.path.clone())
            .collect();

        if selected_paths.is_empty() {
            self.preview_texture = None;
            self.last_preview_params = None;
            return;
        }

        let current_key = (self.params.clone(), selected_paths);
        if self.last_preview_params.as_ref() == Some(&current_key) {
            return;
        }
        self.last_preview_params = Some(current_key);

        self.preview_generation += 1;
        let generation = self.preview_generation;
        self.preview_rendering = true;

        let covers = self.selected_covers();
        let params = self.params.clone();
        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();

        thread::spawn(move || {
            if let Ok(img) = render_wallpaper(&covers, &params, Some(1920)) {
                let _ = tx.send(WorkerEvent::PreviewReady { generation, image: img });
                ctx_clone.request_repaint();
            }
        });
    }

    pub fn start_export(&mut self, ctx: egui::Context, both_orientations: bool) {
        if self.exporting {
            return;
        }
        let covers = self.selected_covers();
        if covers.is_empty() {
            self.banner = Some((true, "Select at least one cover before exporting.".to_owned()));
            return;
        }

        self.exporting = true;
        self.banner = None;
        let base_params = self.params.clone();
        let out_dir = PathBuf::from(self.output_dir.trim());
        let prefix = if self.export_prefix.trim().is_empty() {
            "wallpaper".to_owned()
        } else {
            self.export_prefix.trim().to_owned()
        };
        let tx = self.tx.clone();

        thread::spawn(move || {
            let mut saved = Vec::new();
            let result: Result<Vec<PathBuf>, String> = (|| {
                if both_orientations {
                    let mut landscape = base_params.clone();
                    landscape.width = 3840;
                    landscape.height = 2160;
                    saved.push(
                        export_wallpaper_png(&covers, &landscape, &out_dir, &prefix)
                            .map_err(|e| e.to_string())?,
                    );

                    let mut portrait = base_params;
                    portrait.width = 2160;
                    portrait.height = 3840;
                    portrait.cols = portrait.cols.min(4);
                    saved.push(
                        export_wallpaper_png(&covers, &portrait, &out_dir, &prefix)
                            .map_err(|e| e.to_string())?,
                    );
                } else {
                    saved.push(
                        export_wallpaper_png(&covers, &base_params, &out_dir, &prefix)
                            .map_err(|e| e.to_string())?,
                    );
                }
                Ok(saved)
            })();

            let _ = tx.send(WorkerEvent::ExportDone(result));
            ctx.request_repaint();
        });
    }

    pub fn apply_style_defaults(&mut self, style: LayoutStyle) {
        self.params.style = style;
        match style {
            LayoutStyle::Terasering => {
                self.params.gap = 14;
                self.params.corner_radius = 14;
                self.params.shadow_blur = 10.0;
            }
            LayoutStyle::Diagonal => {
                self.params.gap = 14;
                self.params.angle = 12.0;
                self.params.corner_radius = 14;
                self.params.shadow_blur = 10.0;
            }
            LayoutStyle::GridTipis => {
                self.params.gap = 14;
                self.params.corner_radius = 12;
                self.params.shadow_blur = 10.0;
            }
            LayoutStyle::GridLebar => {
                self.params.gap = 36;
                self.params.corner_radius = 20;
                self.params.shadow_blur = 18.0;
            }
        }
    }

    fn poll_worker_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                WorkerEvent::ExtractProgress(p) => {
                    self.extract_progress = Some(p);
                }
                WorkerEvent::ExtractDone(res) => {
                    self.extract_running = false;
                    match res {
                        Ok(summary) => {
                            let msg = if summary.cancelled {
                                format!(
                                    "Extraction cancelled ({} covers saved).",
                                    summary.extracted_count
                                )
                            } else {
                                format!(
                                    "Extracted {} unique album covers ({} missing).",
                                    summary.extracted_count, summary.missing_count
                                )
                            };
                            self.banner = Some((false, msg));
                            self.extract_summary = Some(summary);
                        }
                        Err(err) => {
                            self.banner = Some((true, err));
                        }
                    }
                }
                WorkerEvent::CoversLoaded(loaded) => {
                    self.covers_loading = false;
                    self.covers = loaded
                        .into_iter()
                        .map(|data| {
                            let thumb_img = resize_rgba(&data.square, 96, 96);
                            let color_img = ColorImage::from_rgba_unmultiplied(
                                [thumb_img.width() as usize, thumb_img.height() as usize],
                                thumb_img.as_raw(),
                            );
                            let thumb = ctx.load_texture(
                                format!("thumb_{}", data.path.display()),
                                color_img,
                                TextureOptions::LINEAR,
                            );
                            StudioCover {
                                data,
                                selected: true,
                                thumb: Some(thumb),
                            }
                        })
                        .collect();

                    if self.covers.len() >= 32 && self.params.width >= self.params.height {
                        self.params.cols = 9;
                    } else if self.params.width < self.params.height {
                        self.params.cols = 4;
                    }
                    self.last_preview_params = None;
                }
                WorkerEvent::PreviewReady { generation, image } => {
                    if generation == self.preview_generation {
                        self.preview_rendering = false;
                        let color_img = ColorImage::from_rgba_unmultiplied(
                            [image.width() as usize, image.height() as usize],
                            image.as_raw(),
                        );
                        self.preview_texture = Some(ctx.load_texture(
                            "wallpaper_preview",
                            color_img,
                            TextureOptions::LINEAR,
                        ));
                    }
                }
                WorkerEvent::ExportDone(res) => {
                    self.exporting = false;
                    match res {
                        Ok(paths) => {
                            let names: Vec<String> = paths
                                .iter()
                                .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
                                .map(ToOwned::to_owned)
                                .collect();
                            self.banner = Some((false, format!("Exported: {}", names.join(", "))));
                        }
                        Err(err) => {
                            self.banner = Some((true, err));
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for CoverWallApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker_events(ctx);

        // Top Mode Bar
        egui::TopBottomPanel::top("top_bar")
            .frame(
                egui::Frame::new()
                    .fill(self.tokens.bg_panel.to_color32())
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .stroke(egui::Stroke::new(1.0, self.tokens.border_subtle.to_color32())),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(logo) = &self.logo_texture {
                        ui.image((logo.id(), egui::vec2(24.0, 24.0)));
                    }
                    ui.label(
                        RichText::new("Cover Wall Studio")
                            .size(17.0)
                            .strong()
                            .color(self.tokens.text_primary.to_color32()),
                    );
                    ui.add_space(16.0);

                    let studio_active = self.mode == AppMode::Studio;
                    if ui.selectable_label(studio_active, "Wallpaper Studio").clicked() {
                        self.mode = AppMode::Studio;
                    }
                    let extract_active = self.mode == AppMode::Extractor;
                    if ui.selectable_label(extract_active, "Extract Covers").clicked() {
                        self.mode = AppMode::Extractor;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let selected_count = self.covers.iter().filter(|c| c.selected).count();
                        let status = if self.exporting {
                            "Exporting 4K...".to_owned()
                        } else if self.extract_running {
                            "Extracting FLACs...".to_owned()
                        } else if self.covers_loading {
                            "Loading covers...".to_owned()
                        } else {
                            format!("{selected_count} / {} covers active", self.covers.len())
                        };
                        ui.label(
                            RichText::new(status)
                                .size(12.5)
                                .color(self.tokens.text_muted.to_color32()),
                        );
                    });
                });
            });

        // Status / Error Banner
        if let Some((is_err, msg)) = self.banner.clone() {
            let accent = if is_err {
                self.tokens.danger.to_color32()
            } else {
                self.tokens.success.to_color32()
            };
            egui::TopBottomPanel::top("status_banner")
                .frame(
                    egui::Frame::new()
                        .fill(accent.gamma_multiply(0.16))
                        .inner_margin(egui::Margin::symmetric(16, 8))
                        .stroke(egui::Stroke::new(1.0, accent)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(msg).color(self.tokens.text_primary.to_color32()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Dismiss").clicked() {
                                self.banner = None;
                            }
                        });
                    });
                });
        }

        match self.mode {
            AppMode::Studio => studio_view::show(self, ctx),
            AppMode::Extractor => extractor_view::show(self, ctx),
        }
    }
}
