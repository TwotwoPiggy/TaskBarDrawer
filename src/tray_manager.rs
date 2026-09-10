use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetWindowLongW, IsWindowVisible, SetForegroundWindow,
    SetWindowLongW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED,
    SWP_SHOWWINDOW, SW_HIDE, SW_RESTORE, SW_SHOW, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use crate::config::AppConfig;
use crate::logger::app_log;
use crate::win_utils::calculate_popup_position_for_hwnd;

static LAST_HIDE_MILLIS: AtomicU64 = AtomicU64::new(0);
static AUTO_SHOW_ON_TRAY_HOVER: AtomicBool = AtomicBool::new(true);

pub fn set_auto_show_on_tray_hover(enabled: bool) {
    AUTO_SHOW_ON_TRAY_HOVER.store(enabled, Ordering::Relaxed);
}

pub fn get_auto_show_on_tray_hover() -> bool {
    AUTO_SHOW_ON_TRAY_HOVER.load(Ordering::Relaxed)
}

pub fn record_window_hide() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    LAST_HIDE_MILLIS.store(now, Ordering::SeqCst);
}

pub fn clear_recently_hidden() {
    LAST_HIDE_MILLIS.store(0, Ordering::SeqCst);
}

pub fn was_recently_hidden(within_ms: u64) -> bool {
    let last = LAST_HIDE_MILLIS.load(Ordering::SeqCst);
    if last == 0 {
        return false;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    now.saturating_sub(last) < within_ms
}

pub struct TrayManager {
    _tray_icon: TrayIcon,
}

fn create_stylish_tray_icon() -> Icon {
    let width = 32;
    let height = 32;
    let mut rgba = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            // Draw a distinctive drawer box icon (32x32)
            if (3..=28).contains(&x) && (5..=26).contains(&y) {
                // Outer border
                if x == 3 || x == 28 || y == 5 || y == 26 || y == 15 {
                    rgba[idx] = 255;
                    rgba[idx + 1] = 255;
                    rgba[idx + 2] = 255;
                    rgba[idx + 3] = 250;
                } else if (12..=19).contains(&x) && (y == 9 || y == 10 || y == 20 || y == 21) {
                    // Golden handles
                    rgba[idx] = 255;
                    rgba[idx + 1] = 200;
                    rgba[idx + 2] = 0;
                    rgba[idx + 3] = 255;
                } else {
                    // Cyan/Blue modern accent fill
                    rgba[idx] = 0;
                    rgba[idx + 1] = 150;
                    rgba[idx + 2] = 255;
                    rgba[idx + 3] = 230;
                }
            }
        }
    }

    Icon::from_rgba(rgba, width as u32, height as u32).expect("Valid icon data")
}

pub fn show_window_by_hwnd(hwnd_raw: isize, egui_ctx: &egui::Context) {
    clear_recently_hidden();
    let hwnd = HWND(hwnd_raw as _);
    unsafe {
        let (target_x, target_y, w, h) = calculate_popup_position_for_hwnd(hwnd, 380.0, 480.0);
        app_log!("Showing popup flyout at ({}, {}) size {}x{} for HWND {}", target_x, target_y, w, h, hwnd_raw);

        // 1. Restore window from minimized state if needed
        let _ = ShowWindow(hwnd, SW_RESTORE);

        // 2. Remove taskbar button and enforce pure toolwindow flyout (never shows on taskbar)
        let mut ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        ex_style &= !(WS_EX_APPWINDOW.0 as i32);
        ex_style |= WS_EX_TOOLWINDOW.0 as i32;
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);

        // 3. Set position right above the system tray and make visible
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            target_x,
            target_y,
            w,
            h,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );

        // 4. Win32 show, bring to top and activate foreground
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }

    // 5. Notify eframe / egui that the viewport is visible and focused
    egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    egui_ctx.request_repaint();
}

pub fn toggle_window_by_hwnd(hwnd_raw: isize, egui_ctx: &egui::Context) {
    let hwnd = HWND(hwnd_raw as _);
    unsafe {
        let is_visible = IsWindowVisible(hwnd).as_bool();
        app_log!("toggle_window_by_hwnd: is_visible={}, was_recently_hidden={}", is_visible, was_recently_hidden(450));

        // If the window was hidden within the last 450ms (e.g. blur triggered when clicking tray),
        // the user's intent was to close it! Keep it hidden.
        if was_recently_hidden(450) {
            app_log!("Window was recently hidden by blur on tray click. Keeping it hidden.");
            return;
        }

        if is_visible {
            app_log!("Window is visible -> Hiding to tray via toggle");
            let _ = ShowWindow(hwnd, SW_HIDE);
            record_window_hide();
            egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else {
            app_log!("Window is hidden -> Showing drawer via toggle");
            show_window_by_hwnd(hwnd_raw, egui_ctx);
        }
    }
}

impl TrayManager {
    pub fn new(hwnd_raw: isize, egui_ctx: egui::Context) -> Result<Self, String> {
        let tray_menu = Menu::new();
        let item_open = MenuItem::new("⚡ 打开快捷抽屉", true, None);
        let item_folder = MenuItem::new("📂 打开 shortcuts 目录", true, None);
        let item_exit = MenuItem::new("❌ 退出程序", true, None);

        let open_item_id = item_open.id().clone();
        let folder_item_id = item_folder.id().clone();
        let exit_item_id = item_exit.id().clone();

        let _ = tray_menu.append(&item_open);
        let _ = tray_menu.append(&item_folder);
        let _ = tray_menu.append(&item_exit);

        let icon = create_stylish_tray_icon();
        // Left-click NEVER shows the context menu (left click pops up/toggles drawer window).
        // Right-click ONLY shows the context menu.
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("快捷抽屉 (Taskbar Drawer)")
            .with_icon(icon)
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(true)
            .build()
            .map_err(|e| format!("Failed to create tray icon: {e}"))?;

        // 1. Direct Tray Icon click event handler: ONLY left click toggles the flyout drawer
        let ctx_clone1 = egui_ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    app_log!("TrayIconEvent callback: Left click triggered -> toggling drawer window!");
                    toggle_window_by_hwnd(hwnd_raw, &ctx_clone1);
                }
            }
        }));

        // 2. Direct Menu click event handler
        let ctx_clone2 = egui_ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            app_log!("MenuEvent callback triggered: {:?}", event.id);
            if event.id == open_item_id {
                app_log!("Tray Menu: '打开快捷抽屉' clicked!");
                show_window_by_hwnd(hwnd_raw, &ctx_clone2);
            } else if event.id == folder_item_id {
                app_log!("Tray Menu: '打开 shortcuts 目录' clicked!");
                let dir = AppConfig::shortcuts_dir();
                let _ = std::process::Command::new("explorer.exe").arg(&dir).spawn();
            } else if event.id == exit_item_id {
                app_log!("Tray Menu: '退出程序' clicked! Terminating.");
                std::process::exit(0);
            }
        }));

        // 3. Background Drag-to-Tray auto-popup monitor thread
        let ctx_clone3 = egui_ctx;
        std::thread::Builder::new()
            .name("tray-drag-monitor".into())
            .spawn(move || {
                run_tray_drag_monitor(hwnd_raw, ctx_clone3);
            })
            .ok();

        app_log!("TrayManager successfully created tray icon and event handlers for HWND {}", hwnd_raw);

        Ok(Self {
            _tray_icon: tray_icon,
        })
    }
}

fn run_tray_drag_monitor(hwnd_raw: isize, egui_ctx: egui::Context) {
    let hwnd = HWND(hwnd_raw as _);
    let mut hover_start: Option<std::time::Instant> = None;

    loop {
        // 1. Check if left mouse button is pressed anywhere across Windows
        let is_lbutton_down = unsafe {
            (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                windows::Win32::UI::Input::KeyboardAndMouse::VK_LBUTTON.0 as i32,
            ) as u16 & 0x8000) != 0
        };

        let sleep_ms = if is_lbutton_down { 40 } else { 100 };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

        // If disabled in settings, skip
        if !get_auto_show_on_tray_hover() {
            hover_start = None;
            continue;
        }

        if !is_lbutton_down {
            hover_start = None;
            continue;
        }

        // 2. If window is already visible, no need to auto-show
        let is_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
        if is_visible {
            hover_start = None;
            continue;
        }

        // 3. If window was recently hidden (e.g. within 600ms), don't bounce open
        if was_recently_hidden(600) {
            hover_start = None;
            continue;
        }

        // 4. Check if cursor is in tray area
        let mut pt = windows::Win32::Foundation::POINT::default();
        let got_pos = unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) }.is_ok();
        if !got_pos || !crate::win_utils::is_cursor_in_tray_area(pt) {
            hover_start = None;
            continue;
        }

        // 5. Cursor is hovering over tray area with mouse button held down! Check dwell time
        let now = std::time::Instant::now();
        match hover_start {
            None => {
                hover_start = Some(now);
            }
            Some(start) => {
                if start.elapsed() >= std::time::Duration::from_millis(200) {
                    app_log!("Tray drag-hover detected at ({}, {}) for >200ms! Auto-showing drawer window.", pt.x, pt.y);
                    show_window_by_hwnd(hwnd_raw, &egui_ctx);
                    hover_start = None;
                }
            }
        }
    }
}
