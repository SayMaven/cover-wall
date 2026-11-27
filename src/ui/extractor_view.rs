//! FLAC cover extractor workspace view with split inspector and responsive thumbnail grid.

use eframe::egui::{self, ProgressBar, RichText, Vec2};

use crate::extractor::CoverOrigin;
use crate::ui::{AppMode, CoverWallApp};

pub fn show(app: &mut CoverWallApp, ctx: &egui::Context) {
    let t = app.language.strings();

    // Left Control Inspector (consistent with Studio view)
    egui::SidePanel::left("extractor_inspector")
        .resizable(false)
        .exact_width(320.0)
        .frame(
            egui::Frame::new()
                .fill(app.tokens.bg_panel.to_color32())
                .inner_margin(egui::Margin::same(16))
                .stroke(egui::Stroke::new(1.0, app.tokens.border_subtle.to_color32())),
        )
        .show(ctx, |ui| {
            ui.label(
                RichText::new(t.extract_heading)
                    .size(16.0)
                    .strong()
                    .color(app.tokens.text_primary.to_color32()),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(t.extract_subtitle)
                    .size(12.0)
                    .color(app.tokens.text_muted.to_color32()),
            );

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);

            // Music Folder
            ui.label(
                RichText::new(t.music_folder)
                    .size(11.5)
                    .strong()
                    .color(app.tokens.text_muted.to_color32()),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_sized([228.0, 26.0], egui::TextEdit::singleline(&mut app.extract_source));
                if ui.button("...").on_hover_text(t.browse).clicked()
                    && let Some(folder) = rfd::FileDialog::new().pick_folder()
                {
                    app.extract_source = folder.display().to_string();
                }
            });

            ui.add_space(10.0);

            // Output Covers Folder
            ui.label(
                RichText::new(t.output_covers_folder)
                    .size(11.5)
                    .strong()
                    .color(app.tokens.text_muted.to_color32()),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_sized([228.0, 26.0], egui::TextEdit::singleline(&mut app.extract_output));
                if ui.button("...").on_hover_text(t.browse).clicked()
                    && let Some(folder) = rfd::FileDialog::new().pick_folder()
                {
                    app.extract_output = folder.display().to_string();
                }
            });

            ui.add_space(16.0);

            // Primary Action
            let primary_btn = egui::Button::new(
                RichText::new(if app.extract_running {
                    t.extracting
                } else {
                    t.start_extraction
                })
                .strong()
                .color(app.tokens.text_primary.to_color32()),
            )
            .fill(app.tokens.accent.to_color32())
            .min_size(Vec2::new(288.0, 34.0));

            if ui.add_enabled(!app.extract_running, primary_btn).clicked() {
                app.start_extraction(ctx.clone());
            }

            if app.extract_running {
                ui.add_space(6.0);
                if ui
                    .add(egui::Button::new(t.cancel).min_size(Vec2::new(288.0, 28.0)))
                    .clicked()
                {
                    app.cancel_extraction();
                }
            }

            if let Some(progress) = &app.extract_progress {
                ui.add_space(12.0);
                let frac = if progress.total > 0 {
                    progress.processed as f32 / progress.total as f32
                } else {
                    0.0
                };
                ui.add(
                    ProgressBar::new(frac)
                        .text(format!("{} / {}", progress.processed, progress.total))
                        .animate(app.extract_running),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(&progress.current_album)
                        .size(11.5)
                        .color(app.tokens.text_muted.to_color32()),
                );
            }

            if let Some(summary) = &app.extract_summary {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                egui::Frame::new()
                    .fill(app.tokens.bg_elevated.to_color32())
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(12))
                    .stroke(egui::Stroke::new(1.0, app.tokens.border_subtle.to_color32()))
                    .show(ui, |ui| {
                        ui.columns(3, |cols| {
                            cols[0].vertical_centered(|ui| {
                                ui.label(RichText::new(summary.total_flacs.to_string()).size(18.0).strong());
                                ui.label(RichText::new("FLAC").size(11.0).color(app.tokens.text_muted.to_color32()));
                            });
                            cols[1].vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(summary.extracted_count.to_string())
                                        .size(18.0)
                                        .strong()
                                        .color(app.tokens.success.to_color32()),
                                );
                                ui.label(
                                    RichText::new(t.extracted_label)
                                        .size(11.0)
                                        .color(app.tokens.text_muted.to_color32()),
                                );
                            });
                            cols[2].vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(summary.missing_count.to_string())
                                        .size(18.0)
                                        .strong()
                                        .color(app.tokens.warning.to_color32()),
                                );
                                ui.label(
                                    RichText::new(t.missing_label)
                                        .size(11.0)
                                        .color(app.tokens.text_muted.to_color32()),
                                );
                            });
                        });
                    });

                if summary.extracted_count > 0 {
                    ui.add_space(10.0);
                    if ui
                        .add(egui::Button::new(t.open_in_studio).min_size(Vec2::new(288.0, 32.0)))
                        .clicked()
                    {
                        app.covers_dir = app.extract_output.clone();
                        app.mode = AppMode::Studio;
                        app.reload_covers(ctx.clone());
                    }
                }
            }
        });

    // Main Results Area (responsive thumbnail grid)
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(app.tokens.bg_canvas.to_color32())
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ctx, |ui| {
            if let Some(summary) = &app.extract_summary {
                let extracted_items: Vec<_> = summary
                    .items
                    .iter()
                    .filter(|i| i.origin != CoverOrigin::Missing)
                    .collect();
                let missing_items: Vec<_> = summary
                    .items
                    .iter()
                    .filter(|i| i.origin == CoverOrigin::Missing)
                    .collect();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !extracted_items.is_empty() {
                            let card_w = 164.0_f32;
                            let spacing = 12.0_f32;
                            let avail_w = ui.available_width();
                            let cols = ((avail_w + spacing) / (card_w + spacing)).floor().max(1.0) as usize;

                            for row in extracted_items.chunks(cols) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::splat(spacing);
                                    for item in row {
                                        egui::Frame::new()
                                            .fill(app.tokens.bg_panel.to_color32())
                                            .corner_radius(8.0)
                                            .inner_margin(egui::Margin::same(10))
                                            .stroke(egui::Stroke::new(
                                                1.0,
                                                app.tokens.border_subtle.to_color32(),
                                            ))
                                            .show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    ui.set_width(card_w - 20.0);
                                                    let thumb_size = Vec2::splat(144.0);
                                                    if let Some(path) = &item.output_path
                                                        && let Some(tex) = app
                                                            .extract_thumbs
                                                            .get(&path.to_string_lossy().into_owned())
                                                    {
                                                        let resp = ui.image((tex.id(), thumb_size));
                                                        ui.painter().rect_stroke(
                                                            resp.rect,
                                                            4.0,
                                                            egui::Stroke::new(
                                                                1.0,
                                                                app.tokens.border_subtle.to_color32(),
                                                            ),
                                                            egui::StrokeKind::Outside,
                                                        );
                                                    } else {
                                                        let (rect, _) = ui.allocate_exact_size(
                                                            thumb_size,
                                                            egui::Sense::hover(),
                                                        );
                                                        ui.painter().rect_filled(
                                                            rect,
                                                            4.0,
                                                            app.tokens.bg_elevated.to_color32(),
                                                        );
                                                    }

                                                    ui.add_space(6.0);
                                                    let title: String = if item.album.chars().count() > 16 {
                                                        let head: String =
                                                            item.album.chars().take(15).collect();
                                                        format!("{head}…")
                                                    } else {
                                                        item.album.clone()
                                                    };
                                                    let hover_info = match &item.file_name {
                                                        Some(fname) => format!("{}\n{fname}", item.album),
                                                        None => item.album.clone(),
                                                    };
                                                    ui.label(RichText::new(title).size(12.5).strong())
                                                        .on_hover_text(hover_info);

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
                                                    ui.label(
                                                        RichText::new(badge)
                                                            .monospace()
                                                            .size(10.5)
                                                            .color(color),
                                                    );
                                                });
                                            });
                                    }
                                });
                                ui.add_space(spacing);
                            }
                        }

                        if !missing_items.is_empty() {
                            ui.add_space(8.0);
                            egui::Frame::new()
                                .fill(app.tokens.bg_panel.to_color32())
                                .corner_radius(8.0)
                                .inner_margin(egui::Margin::same(14))
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    app.tokens.warning.to_color32(),
                                ))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} ({})",
                                            t.missing_label,
                                            missing_items.len()
                                        ))
                                        .strong()
                                        .color(app.tokens.warning.to_color32()),
                                    );
                                    ui.add_space(6.0);
                                    for item in missing_items {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&item.album).strong());
                                            ui.label(
                                                RichText::new(&item.detail)
                                                    .size(11.5)
                                                    .color(app.tokens.text_muted.to_color32()),
                                            );
                                        });
                                    }
                                });
                        }
                    });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        if app.extract_running {
                            ui.spinner();
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(t.extracting)
                                    .size(15.0)
                                    .color(app.tokens.text_primary.to_color32()),
                            );
                        } else {
                            ui.label(
                                RichText::new(t.extract_heading)
                                    .size(17.0)
                                    .strong()
                                    .color(app.tokens.text_primary.to_color32()),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(t.extract_empty_hint)
                                    .color(app.tokens.text_muted.to_color32()),
                            );
                        }
                    });
                });
            }
        });
}
