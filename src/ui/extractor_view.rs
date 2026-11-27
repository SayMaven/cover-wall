//! FLAC cover extractor workspace view.

use eframe::egui::{self, ProgressBar, RichText};

use crate::extractor::CoverOrigin;
use crate::ui::{AppMode, CoverWallApp};

pub fn show(app: &mut CoverWallApp, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(app.tokens.bg_canvas.to_color32())
                .inner_margin(egui::Margin::same(24)),
        )
        .show(ctx, |ui| {
            ui.set_max_width(860.0);

            ui.label(
                RichText::new("Extract Album Covers from FLACs")
                    .size(20.0)
                    .strong()
                    .color(app.tokens.text_primary.to_color32()),
            );
            ui.label(
                RichText::new(
                    "Scans FLAC tags and embedded artwork directly in Rust without FFmpeg.",
                )
                .size(13.0)
                .color(app.tokens.text_muted.to_color32()),
            );
            ui.add_space(16.0);

            egui::Frame::new()
                .fill(app.tokens.bg_panel.to_color32())
                .corner_radius(8.0)
                .inner_margin(egui::Margin::same(16))
                .stroke(egui::Stroke::new(1.0, app.tokens.border_subtle.to_color32()))
                .show(ui, |ui| {
                    egui::Grid::new("extract_paths_grid")
                        .num_columns(3)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Music Folder (FLAC)");
                            ui.add_sized(
                                [440.0, 26.0],
                                egui::TextEdit::singleline(&mut app.extract_source)
                                    .hint_text("e.g. E:\\Music Lossless\\Japanese\\Artists\\Takahashi yu"),
                            );
                            if ui.button("Browse...").clicked()
                                && let Some(folder) = rfd::FileDialog::new().pick_folder()
                            {
                                app.extract_source = folder.display().to_string();
                            }
                            ui.end_row();

                            ui.label("Output Covers Folder");
                            ui.add_sized(
                                [440.0, 26.0],
                                egui::TextEdit::singleline(&mut app.extract_output),
                            );
                            if ui.button("Browse...").clicked()
                                && let Some(folder) = rfd::FileDialog::new().pick_folder()
                            {
                                app.extract_output = folder.display().to_string();
                            }
                            ui.end_row();
                        });

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let primary_btn = egui::Button::new(
                            RichText::new(if app.extract_running {
                                "Extracting..."
                            } else {
                                "Start Extraction"
                            })
                            .strong()
                            .color(app.tokens.text_primary.to_color32()),
                        )
                        .fill(app.tokens.accent.to_color32());

                        if ui
                            .add_enabled(!app.extract_running, primary_btn)
                            .clicked()
                        {
                            app.start_extraction(ctx.clone());
                        }

                        if app.extract_running && ui.button("Cancel").clicked() {
                            app.cancel_extraction();
                        }

                        if let Some(summary) = &app.extract_summary
                            && summary.extracted_count > 0
                            && ui.button("Open Output in Wallpaper Studio").clicked()
                        {
                            app.covers_dir = app.extract_output.clone();
                            app.mode = AppMode::Studio;
                            app.reload_covers(ctx.clone());
                        }
                    });

                    if let Some(progress) = &app.extract_progress {
                        ui.add_space(12.0);
                        let frac = if progress.total > 0 {
                            progress.processed as f32 / progress.total as f32
                        } else {
                            0.0
                        };
                        ui.add(
                            ProgressBar::new(frac)
                                .text(format!(
                                    "{} / {} — {}",
                                    progress.processed, progress.total, progress.current_album
                                ))
                                .animate(app.extract_running),
                        );
                    }
                });

            ui.add_space(16.0);

            if let Some(summary) = &app.extract_summary {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Results ({} FLAC files scanned)", summary.total_flacs))
                            .strong(),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!("Extracted: {}", summary.extracted_count))
                            .color(app.tokens.success.to_color32()),
                    );
                    ui.label(
                        RichText::new(format!("Missing: {}", summary.missing_count))
                            .color(app.tokens.warning.to_color32()),
                    );
                });
                ui.add_space(8.0);

                egui::Frame::new()
                    .fill(app.tokens.bg_panel.to_color32())
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(12))
                    .stroke(egui::Stroke::new(1.0, app.tokens.border_subtle.to_color32()))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .show(ui, |ui| {
                                for item in &summary.items {
                                    ui.horizontal(|ui| {
                                        let (badge, color) = match item.origin {
                                            CoverOrigin::Embedded => {
                                                ("EMBEDDED", app.tokens.success.to_color32())
                                            }
                                            CoverOrigin::Folder => {
                                                ("FOLDER", app.tokens.accent.to_color32())
                                            }
                                            CoverOrigin::Missing => {
                                                ("MISSING", app.tokens.warning.to_color32())
                                            }
                                        };
                                        ui.label(RichText::new(badge).monospace().size(11.0).color(color));
                                        ui.label(RichText::new(&item.album).strong());
                                        if let Some(fname) = &item.file_name {
                                            ui.label(
                                                RichText::new(format!("-> {fname}"))
                                                    .color(app.tokens.text_muted.to_color32()),
                                            )
                                            .on_hover_text(&item.detail);
                                        } else {
                                            ui.label(
                                                RichText::new(&item.detail)
                                                    .size(11.5)
                                                    .color(app.tokens.text_muted.to_color32()),
                                            );
                                        }
                                    });
                                }
                            });
                    });
            } else if !app.extract_running {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(
                        RichText::new("Select a music folder above to scan for unique album covers.")
                            .color(app.tokens.text_muted.to_color32()),
                    );
                });
            }
        });
}
