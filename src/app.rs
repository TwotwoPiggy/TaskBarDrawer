use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use egui::{
    vec2, Align, Align2, Color32, CornerRadius, FontId, Layout, Margin, Rect, Sense, Stroke,
    StrokeKind, TextureHandle, Ui,
};

use crate::config::{AppConfig, LayoutMode, ShortcutItem};
use crate::icon_extractor::extract_file_icon;
use crate::launcher::launch_item;
use crate::logger::app_log;

pub struct DrawerApp {
    pub config: AppConfig,
    pub icon_cache: HashMap<PathBuf, Option<TextureHandle>>,
    pub search_query: String,
    pub dragged_index: Option<usize>,
    pub drag_target_index: Option<usize>,
    pub is_reordering: bool,
    pub just_dropped: bool,
    pub status_message: Option<(String, Instant)>,
    pub show_settings: bool,
    pub launch_time: Instant,
    pub has_gained_focus: bool,
    pub last_focus_state: Option<bool>,
    pub frame_count: u64,
    pub is_visible: bool,
    pub hwnd_raw: isize,
    #[allow(dead_code)]
    pub tray_manager: Option<crate::tray_manager::TrayManager>,
}

impl DrawerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, hwnd_raw: isize) -> Self {
        let config = AppConfig::load();
        app_log!("DrawerApp initialized. Loaded {} items from config. auto_exit_on_blur={}, auto_exit_on_launch={}", 
            config.items.len(), config.auto_exit_on_blur, config.auto_exit_on_launch);

        let tray_manager = if hwnd_raw != 0 {
            match crate::tray_manager::TrayManager::new(hwnd_raw, cc.egui_ctx.clone()) {
                Ok(t) => Some(t),
                Err(e) => {
                    app_log!("TrayManager init error: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            config,
            icon_cache: HashMap::new(),
            search_query: String::new(),
            dragged_index: None,
            drag_target_index: None,
            is_reordering: false,
            just_dropped: false,
            status_message: None,
            show_settings: false,
            launch_time: Instant::now(),
            has_gained_focus: false,
            last_focus_state: None,
            frame_count: 0,
            is_visible: true,
            hwnd_raw,
            tray_manager,
        }
    }

    fn get_or_load_icon(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        if let Some(cached) = self.icon_cache.get(path) {
            return cached.clone();
        }

        let loaded = if let Some(raw) = extract_file_icon(path) {
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [raw.width, raw.height],
                &raw.rgba,
            );
            Some(ctx.load_texture(
                path.to_string_lossy(),
                color_image,
                egui::TextureOptions::LINEAR,
            ))
        } else {
            None
        };

        self.icon_cache.insert(path.to_path_buf(), loaded.clone());
        loaded
    }

    pub fn hide_to_tray(&mut self, ctx: &egui::Context) {
        app_log!("Hiding drawer to tray");
        crate::tray_manager::record_window_hide();
        self.is_visible = false;
        if self.hwnd_raw != 0 {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                    windows::Win32::Foundation::HWND(self.hwnd_raw as _),
                    windows::Win32::UI::WindowsAndMessaging::SW_HIDE,
                );
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn set_status(&mut self, msg: String) {
        app_log!("Status: {}", msg);
        self.status_message = Some((msg, Instant::now()));
    }

    fn handle_external_drop(&mut self, ctx: &egui::Context) {
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            self.just_dropped = true;
        }
        for file in dropped_files {
            if let Some(path) = file.path {
                app_log!("External file dropped into window: {:?}", path);
                if self.config.add_item_from_path(&path) {
                    self.set_status(format!("已添加: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                }
            }
        }
    }

    fn render_header(&mut self, ui: &mut Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            // Search box
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 搜索快捷方式或脚本...")
                    .desired_width(170.0),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Close button (hides to tray)
                if ui.button("✕").on_hover_text("收起抽屉 (Esc)").clicked() {
                    app_log!("Close button clicked");
                    self.hide_to_tray(ui.ctx());
                }

                // Settings button
                let settings_btn = ui.button(if self.show_settings { "▲ 设置" } else { "⚙" });
                if settings_btn.clicked() {
                    self.show_settings = !self.show_settings;
                    app_log!("Toggled settings view: {}", self.show_settings);
                }

                // Add button
                if ui.button("➕ 添加").clicked() {
                    app_log!("Opening file picker dialog");
                    if let Some(file) = rfd::FileDialog::new()
                        .set_title("选择要添加的脚本或程序")
                        .add_filter("脚本与程序", &["bat", "cmd", "exe", "ps1", "lnk", "*"])
                        .pick_file()
                    {
                        app_log!("Selected file from picker: {:?}", file);
                        if self.config.add_item_from_path(&file) {
                            self.set_status(format!("已添加: {}", file.file_name().unwrap_or_default().to_string_lossy()));
                        }
                    }
                    self.launch_time = Instant::now();
                    self.has_gained_focus = false;
                }

                // Layout toggle button
                let (layout_icon, next_layout) = match self.config.layout {
                    LayoutMode::Grid => ("☰ 列表", LayoutMode::List),
                    LayoutMode::List => ("⊞ 网格", LayoutMode::Grid),
                };
                if ui.button(layout_icon).clicked() {
                    self.config.layout = next_layout;
                    let _ = self.config.save();
                    app_log!("Switched layout mode to {:?}", next_layout);
                }
            });
        });

        // Settings panel toggle
        if self.show_settings {
            ui.separator();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.config.auto_exit_on_launch, "运行后自动关闭 (0 待机内存)").changed() {
                    let _ = self.config.save();
                    app_log!("Updated auto_exit_on_launch: {}", self.config.auto_exit_on_launch);
                }
                if ui.checkbox(&mut self.config.auto_exit_on_blur, "失焦后自动关闭").changed() {
                    let _ = self.config.save();
                    app_log!("Updated auto_exit_on_blur: {}", self.config.auto_exit_on_blur);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("📂 打开 shortcuts 快捷文件夹").clicked() {
                    let dir = AppConfig::shortcuts_dir();
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::process::Command::new("explorer.exe").arg(&dir).spawn();
                    app_log!("Opened shortcuts directory in explorer");
                }
                if ui.button("🔄 重新扫描").clicked() {
                    self.config.scan_shortcuts_folder();
                    self.set_status("已完成扫描".to_string());
                }
            });
        }
    }

    fn render_grid_item(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        index: usize,
        item: &ShortcutItem,
        should_exit: &mut bool,
    ) {
        let card_size = vec2(88.0, 92.0);
        let (rect, response) = ui.allocate_exact_size(card_size, Sense::click_and_drag());

        let is_hovered = response.hovered();
        let is_dragging_this = self.dragged_index == Some(index);
        let is_drop_target = self.drag_target_index == Some(index);

        let bg_color = if is_dragging_this {
            Color32::from_rgba_premultiplied(40, 80, 140, 90)
        } else if is_drop_target {
            Color32::from_rgba_premultiplied(30, 144, 255, 60)
        } else if is_hovered {
            Color32::from_rgba_premultiplied(255, 255, 255, 28)
        } else {
            Color32::from_rgba_premultiplied(255, 255, 255, 8)
        };

        let stroke = if is_drop_target {
            Stroke::new(2.0, Color32::from_rgb(0, 191, 255))
        } else if is_hovered {
            Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 70))
        } else {
            Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 14))
        };

        ui.painter().rect(rect, CornerRadius::same(8), bg_color, stroke, StrokeKind::Inside);

        // Render Icon
        let icon_rect = Rect::from_center_size(
            rect.center() - vec2(0.0, 14.0),
            vec2(40.0, 40.0),
        );

        if let Some(texture) = self.get_or_load_icon(ctx, &item.path) {
            ui.painter().image(
                texture.id(),
                icon_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            let letter = item.name.chars().next().unwrap_or('?').to_uppercase().to_string();
            ui.painter().rect_filled(
                icon_rect,
                CornerRadius::same(8),
                Color32::from_rgb(45, 85, 125),
            );
            ui.painter().text(
                icon_rect.center(),
                Align2::CENTER_CENTER,
                letter,
                FontId::proportional(20.0),
                Color32::WHITE,
            );
        }

        // Render Badges
        if item.run_as_admin {
            let badge_pos = icon_rect.right_bottom() - vec2(2.0, 2.0);
            ui.painter().text(
                badge_pos,
                Align2::RIGHT_BOTTOM,
                "🛡️",
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        } else if item.silent {
            let badge_pos = icon_rect.right_bottom() - vec2(2.0, 2.0);
            ui.painter().text(
                badge_pos,
                Align2::RIGHT_BOTTOM,
                "🤫",
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        }

        // Render Title
        let text_rect = Rect::from_min_max(
            rect.min + vec2(4.0, 60.0),
            rect.max - vec2(4.0, 4.0),
        );
        let mut truncated = item.name.clone();
        if truncated.chars().count() > 7 {
            truncated = format!("{}...", truncated.chars().take(6).collect::<String>());
        }
        ui.painter().text(
            text_rect.center_top(),
            Align2::CENTER_TOP,
            truncated,
            FontId::proportional(12.0),
            Color32::from_rgb(230, 230, 230),
        );

        // Drag tracking
        if response.drag_started() {
            self.dragged_index = Some(index);
            self.is_reordering = true;
        }
        if response.dragged() {
            self.is_reordering = true;
        }
        if self.dragged_index.is_some() && self.dragged_index != Some(index) && is_hovered {
            self.drag_target_index = Some(index);
        }

        // Click actions: only if not dragging, not just dropped, and not currently dragged
        if response.clicked()
            && !self.just_dropped
            && !self.is_reordering
            && self.dragged_index.is_none()
            && !response.dragged()
        {
            app_log!("Item clicked: {:?}", item.name);
            if let Err(e) = launch_item(item) {
                self.set_status(e);
            } else if self.config.auto_exit_on_launch {
                app_log!("auto_exit_on_launch is true, requesting window close");
                *should_exit = true;
            }
        }

        // Context Menu
        response.context_menu(|ui| {
            ui.label(format!("📁 {}", item.name));
            ui.separator();
            if ui.button("⚡ 运行").clicked() {
                let _ = launch_item(item);
                if self.config.auto_exit_on_launch {
                    *should_exit = true;
                }
                ui.close_menu();
            }
            if ui.button("🛡️ 以管理员身份运行").clicked() {
                let mut elevated = item.clone();
                elevated.run_as_admin = true;
                let _ = launch_item(&elevated);
                if self.config.auto_exit_on_launch {
                    *should_exit = true;
                }
                ui.close_menu();
            }
            if ui.button("📂 在资源管理器中定位").clicked() {
                let _ = std::process::Command::new("explorer.exe")
                    .args(["/select,", &item.path.to_string_lossy()])
                    .spawn();
                ui.close_menu();
            }
            ui.separator();
            let mut admin = item.run_as_admin;
            if ui.checkbox(&mut admin, "默认以管理员运行").changed() {
                self.config.items[index].run_as_admin = admin;
                let _ = self.config.save();
            }
            let mut silent = item.silent;
            if ui.checkbox(&mut silent, "静默后台执行 (隐藏黑框)").changed() {
                self.config.items[index].silent = silent;
                let _ = self.config.save();
            }
            ui.separator();
            if ui.button("🗑 移除快捷方式").clicked() {
                self.config.remove_item(index);
                ui.close_menu();
            }
        });
    }

    fn render_list_item(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        index: usize,
        item: &ShortcutItem,
        should_exit: &mut bool,
    ) {
        let row_size = vec2(ui.available_width(), 38.0);
        let (rect, response) = ui.allocate_exact_size(row_size, Sense::click_and_drag());

        let is_hovered = response.hovered();
        let is_dragging_this = self.dragged_index == Some(index);
        let is_drop_target = self.drag_target_index == Some(index);

        let bg_color = if is_dragging_this {
            Color32::from_rgba_premultiplied(40, 80, 140, 90)
        } else if is_drop_target {
            Color32::from_rgba_premultiplied(30, 144, 255, 60)
        } else if is_hovered {
            Color32::from_rgba_premultiplied(255, 255, 255, 22)
        } else {
            Color32::from_rgba_premultiplied(255, 255, 255, 6)
        };

        let stroke = if is_drop_target {
            Stroke::new(2.0, Color32::from_rgb(0, 191, 255))
        } else {
            Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 12))
        };

        ui.painter().rect(rect, CornerRadius::same(6), bg_color, stroke, StrokeKind::Inside);

        // Icon
        let icon_rect = Rect::from_min_size(rect.min + vec2(8.0, 7.0), vec2(24.0, 24.0));
        if let Some(texture) = self.get_or_load_icon(ctx, &item.path) {
            ui.painter().image(
                texture.id(),
                icon_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            let letter = item.name.chars().next().unwrap_or('?').to_uppercase().to_string();
            ui.painter().rect_filled(
                icon_rect,
                CornerRadius::same(4),
                Color32::from_rgb(45, 85, 125),
            );
            ui.painter().text(
                icon_rect.center(),
                Align2::CENTER_CENTER,
                letter,
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        }

        // Text
        let text_pos = rect.min + vec2(40.0, 10.0);
        ui.painter().text(
            text_pos,
            Align2::LEFT_TOP,
            &item.name,
            FontId::proportional(13.0),
            Color32::from_rgb(235, 235, 235),
        );

        // Right side badges
        let mut right_cursor = rect.max.x - 10.0;
        if item.run_as_admin {
            ui.painter().text(
                egui::pos2(right_cursor, rect.center().y),
                Align2::RIGHT_CENTER,
                "🛡️ 管理员",
                FontId::proportional(11.0),
                Color32::from_rgb(255, 193, 7),
            );
            right_cursor -= 70.0;
        }
        if item.silent {
            ui.painter().text(
                egui::pos2(right_cursor, rect.center().y),
                Align2::RIGHT_CENTER,
                "🤫 静默",
                FontId::proportional(11.0),
                Color32::from_rgb(144, 202, 249),
            );
        }

        // Drag tracking
        if response.drag_started() {
            self.dragged_index = Some(index);
            self.is_reordering = true;
        }
        if response.dragged() {
            self.is_reordering = true;
        }
        if self.dragged_index.is_some() && self.dragged_index != Some(index) && is_hovered {
            self.drag_target_index = Some(index);
        }

        // Click actions: only if not dragging, not just dropped, and not currently dragged
        if response.clicked()
            && !self.just_dropped
            && !self.is_reordering
            && self.dragged_index.is_none()
            && !response.dragged()
        {
            app_log!("Item clicked: {:?}", item.name);
            if let Err(e) = launch_item(item) {
                self.set_status(e);
            } else if self.config.auto_exit_on_launch {
                app_log!("auto_exit_on_launch is true, requesting window close");
                *should_exit = true;
            }
        }

        // Context menu
        response.context_menu(|ui| {
            ui.label(format!("📁 {}", item.name));
            ui.separator();
            if ui.button("⚡ 运行").clicked() {
                let _ = launch_item(item);
                if self.config.auto_exit_on_launch {
                    *should_exit = true;
                }
                ui.close_menu();
            }
            if ui.button("🛡️ 以管理员身份运行").clicked() {
                let mut elevated = item.clone();
                elevated.run_as_admin = true;
                let _ = launch_item(&elevated);
                if self.config.auto_exit_on_launch {
                    *should_exit = true;
                }
                ui.close_menu();
            }
            if ui.button("📂 在资源管理器中定位").clicked() {
                let _ = std::process::Command::new("explorer.exe")
                    .args(["/select,", &item.path.to_string_lossy()])
                    .spawn();
                ui.close_menu();
            }
            ui.separator();
            let mut admin = item.run_as_admin;
            if ui.checkbox(&mut admin, "默认以管理员运行").changed() {
                self.config.items[index].run_as_admin = admin;
                let _ = self.config.save();
            }
            let mut silent = item.silent;
            if ui.checkbox(&mut silent, "静默后台执行 (隐藏黑框)").changed() {
                self.config.items[index].silent = silent;
                let _ = self.config.save();
            }
            ui.separator();
            if ui.button("🗑 移除快捷方式").clicked() {
                self.config.remove_item(index);
                ui.close_menu();
            }
        });
    }
}

impl eframe::App for DrawerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;

        // Sync visibility from OS HWND
        if self.hwnd_raw != 0 {
            let os_visible = unsafe {
                windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(
                    windows::Win32::Foundation::HWND(self.hwnd_raw as _),
                )
                .as_bool()
            };
            if !self.is_visible && os_visible {
                app_log!("Window transitioned to visible from tray");
                self.launch_time = Instant::now();
                self.has_gained_focus = false;
            }
            self.is_visible = os_visible;
        }

        // If window is currently hidden in tray, keep polling with low CPU and avoid rendering
        if !self.is_visible {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
            return;
        }

        // Press Escape to hide drawer to tray
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app_log!("Escape key pressed");
            self.hide_to_tray(ctx);
            return;
        }

        // Handle external drag-and-drop
        self.handle_external_drop(ctx);

        // Check window focus state using Win32 GetForegroundWindow and egui input focus
        let is_fg = if self.hwnd_raw != 0 {
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow()
                    == windows::Win32::Foundation::HWND(self.hwnd_raw as _)
            }
        } else {
            ctx.input(|i| i.focused)
        };

        let is_focused = is_fg || ctx.input(|i| i.focused);
        if self.last_focus_state != Some(is_focused) {
            app_log!("Window focus state changed: is_focused={}, is_fg={}, egui_focused={} (elapsed={}ms)", 
                is_focused, is_fg, ctx.input(|i| i.focused), self.launch_time.elapsed().as_millis());
            self.last_focus_state = Some(is_focused);
        }

        if is_focused {
            self.has_gained_focus = true;
        }

        // Clear just_dropped flag when pointer is no longer released
        if !ctx.input(|i| i.pointer.any_released()) && ctx.input(|i| i.raw.dropped_files.is_empty()) {
            self.just_dropped = false;
        }

        // Check if cursor is physically inside the drawer window
        let is_cursor_inside = if self.hwnd_raw != 0 {
            let mut pt = windows::Win32::Foundation::POINT::default();
            let mut rect = windows::Win32::Foundation::RECT::default();
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
                let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(
                    windows::Win32::Foundation::HWND(self.hwnd_raw as _),
                    &mut rect,
                );
            }
            pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.bottom
        } else {
            true
        };

        let is_mouse_down = ctx.input(|i| i.pointer.any_down());
        let is_dragging = self.dragged_index.is_some() || self.is_reordering || ctx.input(|i| i.pointer.is_decidedly_dragging());
        let is_hovering_files = ctx.input(|i| !i.raw.hovered_files.is_empty());

        let is_active_interaction = is_mouse_down || is_dragging || is_hovering_files || is_cursor_inside;

        // Check blur exit if configured (hides to tray)
        if self.config.auto_exit_on_blur && !self.show_settings {
            // Actively poll every 50ms while visible so clicking outside is detected immediately
            ctx.request_repaint_after(std::time::Duration::from_millis(50));

            let elapsed = self.launch_time.elapsed().as_millis();
            let should_blur = !is_active_interaction && !is_fg && (
                (self.has_gained_focus && elapsed > 200) || (elapsed > 600)
            );

            if should_blur {
                app_log!("Auto hide on blur triggered! (has_gained_focus={}, elapsed={}ms, is_fg={}). Hiding to tray.",
                    self.has_gained_focus, elapsed, is_fg);
                self.hide_to_tray(ctx);
                return;
            }
        }

        // If mouse button is released anywhere, finalize internal drag & drop
        if ctx.input(|i| i.pointer.any_released()) {
            if self.is_reordering {
                if let (Some(from), Some(to)) = (self.dragged_index, self.drag_target_index) {
                    app_log!("Internal drag & drop completed: moved item from {} to {}", from, to);
                    self.config.move_item(from, to);
                }
                self.just_dropped = true;
                self.is_reordering = false;
            }
            self.dragged_index = None;
            self.drag_target_index = None;
        }

        // Draw modern Fluent Acrylic-like frame
        let panel_frame = egui::Frame {
            inner_margin: Margin::same(12),
            outer_margin: Margin::ZERO,
            corner_radius: CornerRadius::same(12),
            shadow: egui::Shadow {
                offset: [0, 6],
                blur: 16,
                spread: 0,
                color: Color32::from_rgba_premultiplied(0, 0, 0, 140),
            },
            fill: Color32::from_rgb(26, 30, 36),
            stroke: Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 22)),
        };

        let mut should_exit = false;

        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            // Header
            self.render_header(ui, ctx);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Empty state or Items
            let query = self.search_query.trim().to_lowercase();
            let matching_indices: Vec<usize> = self
                .config
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if query.is_empty() {
                        true
                    } else {
                        item.name.to_lowercase().contains(&query)
                            || item.path.to_string_lossy().to_lowercase().contains(&query)
                    }
                })
                .map(|(idx, _)| idx)
                .collect();

            if self.config.items.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(
                        egui::RichText::new("📥 拖入任意 .bat / .cmd / .exe / 快捷方式")
                            .color(Color32::from_rgb(180, 180, 180))
                            .size(14.0),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("直接从文件资源管理器或桌面拖放文件至此处")
                            .color(Color32::from_rgb(120, 120, 120))
                            .size(12.0),
                    );
                    ui.add_space(20.0);
                });
            } else if matching_indices.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label("未找到匹配的快捷方式");
                    ui.add_space(30.0);
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.config.layout {
                            LayoutMode::Grid => {
                                let available_width = ui.available_width();
                                let cols = ((available_width + 8.0) / (88.0 + 8.0)).floor().max(1.0) as usize;

                                egui::Grid::new("shortcuts_grid")
                                    .spacing(vec2(8.0, 8.0))
                                    .show(ui, |ui| {
                                        for (col_count, &idx) in matching_indices.iter().enumerate() {
                                            let item = self.config.items[idx].clone();
                                            self.render_grid_item(
                                                ui,
                                                ctx,
                                                idx,
                                                &item,
                                                &mut should_exit,
                                            );
                                            if (col_count + 1) % cols == 0 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                            }
                            LayoutMode::List => {
                                ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                                for &idx in &matching_indices {
                                    let item = self.config.items[idx].clone();
                                    self.render_list_item(
                                        ui,
                                        ctx,
                                        idx,
                                        &item,
                                        &mut should_exit,
                                    );
                                }
                            }
                        }
                    });
            }

            // Status message toast
            if let Some((msg, created_at)) = &self.status_message {
                if created_at.elapsed().as_secs() < 3 {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(msg)
                            .color(Color32::from_rgb(100, 200, 100))
                            .size(11.0),
                    );
                }
            }
        });

        if should_exit {
            app_log!("Shortcut launched, hiding drawer to tray");
            self.hide_to_tray(ctx);
        }
    }
}
