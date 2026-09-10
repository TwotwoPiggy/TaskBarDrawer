use std::sync::Arc;
use egui::{Color32, CornerRadius, Stroke};

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Microsoft YaHei for Chinese
    let yahei_candidates = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simhei.ttf",
    ];

    let mut loaded_chinese = false;
    for font_path in yahei_candidates {
        if let Ok(font_data) = std::fs::read(font_path) {
            fonts.font_data.insert(
                "system_chinese".to_owned(),
                Arc::new(egui::FontData::from_owned(font_data)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "system_chinese".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "system_chinese".to_owned());
            loaded_chinese = true;
            break;
        }
    }

    // 2. Segoe UI Symbol / Emoji for system glyphs (⚙, 🔍, ✕, ➕, 🛡️, etc.)
    let symbol_candidates = [
        "C:\\Windows\\Fonts\\seguisym.ttf",
        "C:\\Windows\\Fonts\\seguiemj.ttf",
    ];
    for sym_path in symbol_candidates {
        if let Ok(sym_data) = std::fs::read(sym_path) {
            fonts.font_data.insert(
                "system_symbol".to_owned(),
                Arc::new(egui::FontData::from_owned(sym_data)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("system_symbol".to_owned());
            break;
        }
    }

    if loaded_chinese {
        ctx.set_fonts(fonts);
    }
}

pub fn apply_fluent_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(Color32::from_rgb(240, 243, 248));
    visuals.window_fill = Color32::from_rgb(26, 30, 36);
    visuals.panel_fill = Color32::from_rgb(26, 30, 36);
    visuals.extreme_bg_color = Color32::from_rgba_unmultiplied(18, 22, 28, 220);

    // Inactive widgets (buttons, inputs)
    visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 12);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18));
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(220, 225, 235));

    // Hovered widgets
    visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 24);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 45));
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    // Active (pressed) widgets
    visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 36);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0, 150, 255));
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    // Noninteractive widgets
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 8);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 12));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(160, 168, 180));

    // Selection
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 120, 215, 160);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(0, 150, 255));

    visuals.window_corner_radius = CornerRadius::same(12);

    ctx.set_visuals(visuals);
}

