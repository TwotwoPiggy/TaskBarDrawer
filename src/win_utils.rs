use windows::Win32::Foundation::{BOOL, HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABM_GETTASKBARPOS, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetSystemMetrics, GetWindowRect, SM_CXSCREEN, SM_CYSCREEN,
};

pub fn apply_win11_dwm_attributes(hwnd_raw: isize, is_dark: bool) {
    let hwnd = HWND(hwnd_raw as _);
    unsafe {
        // Apply Win11 rounded corners (DWMWCP_ROUND = 2)
        let corner_pref = DWM_WINDOW_CORNER_PREFERENCE(2);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner_pref as *const _ as *const _,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );

        // Apply dark mode theme if requested
        let dark_val: BOOL = BOOL(if is_dark { 1 } else { 0 });
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_val as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
    }
}

#[allow(dead_code)]
pub fn is_window_focused(hwnd_raw: isize) -> bool {
    let hwnd = HWND(hwnd_raw as _);
    unsafe {
        let foreground = GetForegroundWindow();
        foreground == hwnd
    }
}

pub fn calculate_popup_position(win_width: f32, win_height: f32) -> (f32, f32) {
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as f32;
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as f32;

    let mut appbar_data = APPBARDATA::default();
    appbar_data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    let success = unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut appbar_data) };

    let margin = 12.0;

    if success != 0 {
        let taskbar_rect: RECT = appbar_data.rc;
        match appbar_data.uEdge {
            ABE_BOTTOM => {
                // Taskbar at bottom: place window in bottom-right directly above taskbar
                let taskbar_top = taskbar_rect.top as f32;
                let target_x = screen_w - win_width - margin;
                let target_y = taskbar_top - win_height - margin;
                return (target_x, target_y.max(margin));
            }
            ABE_TOP => {
                let taskbar_bottom = taskbar_rect.bottom as f32;
                let target_x = screen_w - win_width - margin;
                let target_y = taskbar_bottom + margin;
                return (target_x, target_y);
            }
            ABE_LEFT => {
                let taskbar_right = taskbar_rect.right as f32;
                let target_x = taskbar_right + margin;
                let target_y = screen_h - win_height - margin;
                return (target_x, target_y);
            }
            ABE_RIGHT => {
                let taskbar_left = taskbar_rect.left as f32;
                let target_x = taskbar_left - win_width - margin;
                let target_y = screen_h - win_height - margin;
                return (target_x, target_y);
            }
            _ => {}
        }
    }

    // Default fallback: bottom right near system tray
    let target_x = screen_w - win_width - margin;
    let target_y = (screen_h - win_height - 60.0).max(margin);
    (target_x, target_y)
}

pub fn calculate_popup_position_for_hwnd(hwnd: HWND, fallback_w: f32, fallback_h: f32) -> (i32, i32, i32, i32) {
    let mut win_rect = RECT::default();
    let (mut w, mut h) = (fallback_w as i32, fallback_h as i32);
    if unsafe { GetWindowRect(hwnd, &mut win_rect) }.is_ok() {
        let rw = win_rect.right - win_rect.left;
        let rh = win_rect.bottom - win_rect.top;
        if rw > 100 && rh > 100 {
            w = rw;
            h = rh;
        }
    }

    let (x, y) = calculate_popup_position(w as f32, h as f32);
    (x as i32, y as i32, w, h)
}
