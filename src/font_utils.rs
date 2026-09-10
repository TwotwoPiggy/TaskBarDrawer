use std::sync::Arc;

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let font_candidates = [
        "C:\\Windows\\Fonts\\msyh.ttc", // Microsoft YaHei
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simhei.ttf", // SimHei
        "C:\\Windows\\Fonts\\simsun.ttc", // SimSun
    ];

    for font_path in font_candidates {
        if let Ok(font_data) = std::fs::read(font_path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "system_chinese".to_owned(),
                Arc::new(egui::FontData::from_owned(font_data)),
            );

            // Prioritize system font for proportional and monospace
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

            ctx.set_fonts(fonts);
            return;
        }
    }
}
