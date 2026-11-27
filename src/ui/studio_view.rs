//! Interactive Wallpaper Studio view with live preview and cover strip.

use eframe::egui::{self, RichText, Sense, Vec2};

use crate::renderer::{AspectPreset, BG_PALETTE_SWATCHES, BackgroundMode, LayoutStyle};
use crate::ui::{AppMode, CoverWallApp};

pub fn show(app: &mut CoverWallApp, ctx: &egui::Context) {
    let t = app.language.strings();

    // Left Inspector Panel
    egui::SidePanel::left("studio_inspector")
        .resizable(false)
        .exact_width(310.0)
        .frame(
            egui::Frame::new()
                .fill(app.tokens.bg_panel.to_color32())
                .inner_margin(egui::Margin::same(14))
                .stroke(egui::Stroke::new(1.0, app.tokens.border_subtle.to_color32())),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Covers Source Section
                ui.label(RichText::new(t.covers_source).size(11.5).strong().color(app.tokens.text_muted.to_color32()));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_sized([185.0, 26.0], egui::TextEdit::singleline(&mut app.covers_dir));
                    if ui.button("...").on_hover_text(t.browse_covers_folder).clicked()
                        && let Some(folder) = rfd::FileDialog::new().pick_folder()
                    {
                        app.covers_dir = folder.display().to_string();
                        app.reload_covers(ctx.clone());
                    }
                    if ui.button(t.load).clicked() {
                        app.reload_covers(ctx.clone());
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(t.all).clicked() {
                        for c in &mut app.covers {
                            c.selected = true;
                        }
                    }
                    if ui.button(t.none).clicked() {
                        for c in &mut app.covers {
                            c.selected = false;
                        }
                    }
                    ui.checkbox(&mut app.params.shuffle, t.shuffle);
                    if app.params.shuffle && ui.button(t.new_seed).clicked() {
                        app.params.seed = app.params.seed.wrapping_add(1);
                    }
                });
                if app.params.shuffle {
                    ui.horizontal(|ui| {
                        ui.label(t.seed);
                        ui.add(egui::DragValue::new(&mut app.params.seed).speed(1));
                    });
                }

                ui.separator();

                // Resolution Section
                ui.label(RichText::new(t.resolution).size(11.5).strong().color(app.tokens.text_muted.to_color32()));
                ui.add_space(4.0);
                egui::ComboBox::from_id_salt("aspect_preset_combo")
                    .selected_text(app.aspect_preset.label())
                    .width(275.0)
                    .show_ui(ui, |ui| {
                        for preset in AspectPreset::ALL {
                            if ui.selectable_value(&mut app.aspect_preset, preset, preset.label()).clicked()
                                && preset != AspectPreset::Custom
                            {
                                let (w, h) = preset.dimensions();
                                app.params.width = w;
                                app.params.height = h;
                                app.params.cols = if w < h { 4 } else if app.covers.len() >= 32 { 9 } else { 7 };
                            }
                        }
                    });

                if app.aspect_preset == AspectPreset::Custom {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("W:");
                        ui.add(egui::DragValue::new(&mut app.params.width).range(320..=7680));
                        ui.label("H:");
                        ui.add(egui::DragValue::new(&mut app.params.height).range(320..=7680));
                    });
                }

                ui.separator();

                // Layout Model Section
                ui.label(RichText::new(t.layout_model).size(11.5).strong().color(app.tokens.text_muted.to_color32()));
                ui.add_space(4.0);
                ui.columns(2, |cols| {
                    for (idx, style) in LayoutStyle::ALL.iter().enumerate() {
                        let col = &mut cols[idx % 2];
                        let active = app.params.style == *style;
                        if col.selectable_label(active, style.label()).clicked() {
                            app.apply_style_defaults(*style);
                        }
                    }
                });

                ui.add_space(8.0);
                ui.label(RichText::new(t.parameters).size(11.5).strong().color(app.tokens.text_muted.to_color32()));
                ui.add_space(4.0);

                ui.add(egui::Slider::new(&mut app.params.cols, 2..=16).text(t.columns));
                ui.add(egui::Slider::new(&mut app.params.gap, 0..=80).text(t.gap_px));
                if app.params.style == LayoutStyle::Diagonal {
                    ui.add(egui::Slider::new(&mut app.params.angle, -45.0..=45.0).text(t.angle_deg));
                }
                ui.add(egui::Slider::new(&mut app.params.corner_radius, 0..=48).text(t.radius_px));
                ui.add(egui::Slider::new(&mut app.params.shadow_blur, 0.0..=32.0).text(t.shadow));

                ui.separator();

                // Background Section
                ui.label(RichText::new(t.background).size(11.5).strong().color(app.tokens.text_muted.to_color32()));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for mode in BackgroundMode::ALL {
                        let label = match mode {
                            BackgroundMode::Solid => t.bg_solid,
                            BackgroundMode::BlurredCovers => t.bg_blurred,
                            BackgroundMode::Transparent => t.bg_transparent,
                        };
                        if ui.selectable_label(app.params.bg_mode == mode, label).clicked() {
                            app.params.bg_mode = mode;
                        }
                    }
                });

                if app.params.bg_mode != BackgroundMode::Transparent {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for (name, rgb) in BG_PALETTE_SWATCHES {
                            let c = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            let selected = app.params.bg_color == rgb;
                            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
                            if resp.clicked() {
                                app.params.bg_color = rgb;
                            }
                            ui.painter().rect_filled(rect, 4.0, c);
                            let stroke = if selected {
                                egui::Stroke::new(2.0, app.tokens.accent.to_color32())
                            } else {
                                egui::Stroke::new(1.0, app.tokens.border_subtle.to_color32())
                            };
                            ui.painter().rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Outside);
                            resp.on_hover_text(name);
                        }
                        ui.color_edit_button_srgb(&mut app.params.bg_color);
                    });
                }

                if app.params.bg_mode == BackgroundMode::BlurredCovers {
                    ui.add_space(4.0);
                    ui.add(egui::Slider::new(&mut app.params.bg_tint_opacity, 0.0..=1.0).text(t.tint_opacity));
                }

                ui.separator();

                // Export Section
                ui.label(RichText::new(t.export_section).size(11.5).strong().color(app.tokens.text_muted.to_color32()));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_sized([225.0, 26.0], egui::TextEdit::singleline(&mut app.output_dir));
                    if ui.button("...").clicked()
                        && let Some(folder) = rfd::FileDialog::new().pick_folder()
                    {
                        app.output_dir = folder.display().to_string();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t.prefix);
                    ui.add_sized([220.0, 24.0], egui::TextEdit::singleline(&mut app.export_prefix));
                });

                ui.add_space(8.0);
                let export_btn = egui::Button::new(
                    RichText::new(if app.exporting { t.exporting } else { t.export_4k_png })
                        .strong()
                        .color(app.tokens.text_primary.to_color32()),
                )
                .fill(app.tokens.accent.to_color32())
                .min_size(Vec2::new(280.0, 34.0));

                if ui.add_enabled(!app.exporting && !app.covers.is_empty(), export_btn).clicked() {
                    app.start_export(ctx.clone(), false);
                }
                ui.add_space(4.0);
                if ui
                    .add_enabled(
                        !app.exporting && !app.covers.is_empty(),
                        egui::Button::new(t.export_both).min_size(Vec2::new(280.0, 28.0)),
                    )
                    .clicked()
                {
                    app.start_export(ctx.clone(), true);
                }
            });
        });

    // Bottom Cover Strip
    if !app.covers.is_empty() {
        egui::TopBottomPanel::bottom("cover_strip")
            .exact_height(if app.show_cover_strip { 126.0 } else { 32.0 })
            .frame(
                egui::Frame::new()
                    .fill(app.tokens.bg_panel.to_color32())
                    .inner_margin(egui::Margin::symmetric(14, 6))
                    .stroke(egui::Stroke::new(1.0, app.tokens.border_subtle.to_color32())),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label = if app.show_cover_strip {
                        t.hide_cover_library
                    } else {
                        t.show_cover_library
                    };
                    if ui.button(label).clicked() {
                        app.show_cover_strip = !app.show_cover_strip;
                    }
                    ui.label(
                        RichText::new(t.cover_strip_hint)
                            .size(12.0)
                            .color(app.tokens.text_muted.to_color32()),
                    );
                });

                if app.show_cover_strip {
                    ui.add_space(4.0);
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for cover in &mut app.covers {
                                let size = Vec2::new(68.0, 82.0);
                                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                                if resp.clicked() {
                                    cover.selected = !cover.selected;
                                }

                                let border_color = if cover.selected {
                                    app.tokens.accent.to_color32()
                                } else {
                                    app.tokens.border_subtle.to_color32()
                                };

                                let img_rect = egui::Rect::from_min_size(
                                    rect.min + Vec2::new(2.0, 0.0),
                                    Vec2::splat(64.0),
                                );
                                if let Some(tex) = &cover.thumb {
                                    let tint = if cover.selected {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgba_unmultiplied(120, 120, 120, 120)
                                    };
                                    ui.painter().image(
                                        tex.id(),
                                        img_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        tint,
                                    );
                                }
                                ui.painter().rect_stroke(
                                    img_rect,
                                    4.0,
                                    egui::Stroke::new(if cover.selected { 2.0 } else { 1.0 }, border_color),
                                    egui::StrokeKind::Outside,
                                );

                                let title_rect = egui::Rect::from_min_max(
                                    egui::pos2(rect.min.x, img_rect.max.y + 2.0),
                                    rect.max,
                                );
                                let short_title: String = if cover.data.title.chars().count() > 7 {
                                    let head: String = cover.data.title.chars().take(6).collect();
                                    format!("{head}…")
                                } else {
                                    cover.data.title.clone()
                                };
                                ui.painter().with_clip_rect(title_rect).text(
                                    title_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    short_title,
                                    egui::FontId::proportional(10.5),
                                    app.tokens.text_muted.to_color32(),
                                );

                                resp.on_hover_text(&cover.data.title);
                            }
                        });
                    });
                }
            });
    }

    // Trigger background preview render if parameters/selection changed
    app.trigger_preview_if_needed(ctx);

    // Central Live Preview Canvas
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(app.tokens.bg_canvas.to_color32())
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ctx, |ui| {
            if app.covers_loading {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(t.loading_covers);
                    });
                });
                return;
            }

            if app.covers.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(t.no_covers_loaded)
                                .size(18.0)
                                .strong()
                                .color(app.tokens.text_primary.to_color32()),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(t.no_covers_hint)
                                .color(app.tokens.text_muted.to_color32()),
                        );
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            if ui.button(t.browse_covers_folder).clicked()
                                && let Some(folder) = rfd::FileDialog::new().pick_folder()
                            {
                                app.covers_dir = folder.display().to_string();
                                app.reload_covers(ctx.clone());
                            }
                            if ui.button(t.go_to_extract).clicked() {
                                app.mode = AppMode::Extractor;
                            }
                        });
                    });
                });
                return;
            }

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} — {} ({} × {})",
                        t.live_preview,
                        app.params.style.label(),
                        app.params.width,
                        app.params.height
                    ))
                    .size(12.5)
                    .color(app.tokens.text_muted.to_color32()),
                );
                if app.preview_rendering {
                    ui.add_space(8.0);
                    ui.spinner();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_portrait = app.aspect_preset == AspectPreset::Portrait4K
                        || (app.params.width == 2160 && app.params.height == 3840);
                    if ui.selectable_label(is_portrait, "9:16").clicked() {
                        app.aspect_preset = AspectPreset::Portrait4K;
                        app.params.width = 2160;
                        app.params.height = 3840;
                        app.params.cols = 4;
                    }
                    let is_landscape = app.aspect_preset == AspectPreset::Landscape4K
                        || (app.params.width == 3840 && app.params.height == 2160);
                    if ui.selectable_label(is_landscape, "16:9").clicked() {
                        app.aspect_preset = AspectPreset::Landscape4K;
                        app.params.width = 3840;
                        app.params.height = 2160;
                        app.params.cols = if app.covers.len() >= 32 { 9 } else { 7 };
                    }
                });
            });
            ui.add_space(8.0);

            let avail = ui.available_size();
            if let Some(tex) = &app.preview_texture {
                let tex_size = tex.size_vec2();
                let aspect = tex_size.x / tex_size.y.max(1.0);
                let mut draw_w = avail.x;
                let mut draw_h = draw_w / aspect;
                if draw_h > avail.y {
                    draw_h = avail.y;
                    draw_w = draw_h * aspect;
                }
                let size = Vec2::new(draw_w.max(64.0), draw_h.max(64.0));
                ui.centered_and_justified(|ui| {
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    if app.params.bg_mode == BackgroundMode::Transparent {
                        let cell = 14.0_f32;
                        let c1 = egui::Color32::from_rgb(28, 28, 34);
                        let c2 = egui::Color32::from_rgb(40, 40, 48);
                        ui.painter().rect_filled(rect, 0.0, c1);
                        let cols = (rect.width() / cell).ceil() as i32;
                        let rows = (rect.height() / cell).ceil() as i32;
                        for r in 0..rows {
                            for c in 0..cols {
                                if (r + c) % 2 == 1 {
                                    let min = egui::pos2(rect.min.x + c as f32 * cell, rect.min.y + r as f32 * cell);
                                    let max = egui::pos2(
                                        (min.x + cell).min(rect.max.x),
                                        (min.y + cell).min(rect.max.y),
                                    );
                                    ui.painter().rect_filled(egui::Rect::from_min_max(min, max), 0.0, c2);
                                }
                            }
                        }
                    }
                    ui.painter().image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    ui.painter().rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(1.0, app.tokens.border_subtle.to_color32()),
                        egui::StrokeKind::Outside,
                    );
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(t.select_one_to_preview)
                            .color(app.tokens.text_muted.to_color32()),
                    );
                });
            }
        });
}
