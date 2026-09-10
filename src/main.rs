#![windows_subsystem = "windows"]

mod app;
mod config;
mod font_utils;
mod icon_extractor;
mod launcher;
mod logger;
mod tray_manager;
mod win_utils;

use app::DrawerApp;
use egui::ViewportBuilder;
use logger::app_log;
use win_utils::{apply_win11_dwm_attributes, calculate_popup_position, calculate_popup_position_for_hwnd};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, FindWindowW, GetWindowLongW, SetForegroundWindow, SetWindowLongW,
    SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_SHOWWINDOW, WS_EX_APPWINDOW,
    WS_EX_TOOLWINDOW,
};

fn main() -> eframe::Result<()> {
    logger::init();
    app_log!("Application starting up...");

    let window_width = 380.0;
    let window_height = 480.0;
    let (target_x, target_y) = calculate_popup_position(window_width, window_height);
    app_log!("Calculated initial popup position: ({}, {}) for size: {}x{}", target_x, target_y, window_width, window_height);

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Taskbar Drawer")
            .with_inner_size([window_width, window_height])
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_position([target_x, target_y])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Taskbar Drawer",
        options,
        Box::new(|cc| {
            app_log!("eframe App creation callback invoked");

            let mut hwnd_raw = 0isize;
            unsafe {
                let title = windows::core::w!("Taskbar Drawer");
                if let Ok(hwnd) = FindWindowW(None, title) {
                    if !hwnd.is_invalid() {
                        hwnd_raw = hwnd.0 as isize;
                        app_log!("Found HWND: {:?}, setting WS_EX_TOOLWINDOW and docking above tray", hwnd);
                        apply_win11_dwm_attributes(hwnd.0 as isize, true);

                        // Pure flyout popup: lives exclusively in system tray, does NOT show on the Windows taskbar
                        let mut ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                        ex_style &= !(WS_EX_APPWINDOW.0 as i32);
                        ex_style |= WS_EX_TOOLWINDOW.0 as i32;
                        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);

                        let (x, y, w, h) = calculate_popup_position_for_hwnd(hwnd, window_width, window_height);
                        let _ = SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            x,
                            y,
                            w,
                            h,
                            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                        );

                        let _ = BringWindowToTop(hwnd);
                        let _ = SetForegroundWindow(hwnd);
                    } else {
                        app_log!("FindWindowW returned invalid HWND");
                    }
                } else {
                    app_log!("FindWindowW failed to find window with title 'Taskbar Drawer'");
                }
            }

            // Setup system Chinese fonts (Microsoft YaHei) and symbol fonts
            font_utils::setup_custom_fonts(&cc.egui_ctx);
            font_utils::apply_fluent_dark_theme(&cc.egui_ctx);
            app_log!("Fonts and Fluent dark theme initialization completed");

            Ok(Box::new(DrawerApp::new(cc, hwnd_raw)))
        }),
    )
}
