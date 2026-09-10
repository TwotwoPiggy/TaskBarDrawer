use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDIBits, GetDC, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HGDIOBJ,
};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

pub struct ExtractedIcon {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

fn path_to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

pub fn extract_file_icon(file_path: &Path) -> Option<ExtractedIcon> {
    let wide_path = path_to_wide(file_path);
    let mut sfi = SHFILEINFOW::default();

    let res = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide_path.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi as *mut _ as *mut _),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };

    if res == 0 || sfi.hIcon.is_invalid() {
        return None;
    }

    let hicon: HICON = sfi.hIcon;
    let icon_data = hicon_to_rgba(hicon);
    let _ = unsafe { DestroyIcon(hicon) };

    icon_data
}

fn hicon_to_rgba(hicon: HICON) -> Option<ExtractedIcon> {
    unsafe {
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(hicon, &mut icon_info).is_err() {
            return None;
        }

        let hbm_color = icon_info.hbmColor;
        let hbm_mask = icon_info.hbmMask;

        let mut bm = BITMAP::default();
        if GetObjectW(
            HGDIOBJ(hbm_color.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        ) == 0
        {
            if !hbm_color.is_invalid() {
                let _ = DeleteObject(hbm_color);
            }
            if !hbm_mask.is_invalid() {
                let _ = DeleteObject(hbm_mask);
            }
            return None;
        }

        let width = bm.bmWidth as usize;
        let height = bm.bmHeight as usize;
        if width == 0 || height == 0 {
            if !hbm_color.is_invalid() {
                let _ = DeleteObject(hbm_color);
            }
            if !hbm_mask.is_invalid() {
                let _ = DeleteObject(hbm_mask);
            }
            return None;
        }

        let hdc: HDC = GetDC(HWND::default());
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bgra_buffer = vec![0u8; width * height * 4];
        let lines = GetDIBits(
            hdc,
            hbm_color,
            0,
            height as u32,
            Some(bgra_buffer.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let _ = ReleaseDC(HWND::default(), hdc);
        if !hbm_color.is_invalid() {
            let _ = DeleteObject(hbm_color);
        }
        if !hbm_mask.is_invalid() {
            let _ = DeleteObject(hbm_mask);
        }

        if lines == 0 {
            return None;
        }

        // Convert BGRA to RGBA and check if alpha is present
        let mut has_alpha = false;
        for chunk in bgra_buffer.chunks_exact(4) {
            if chunk[3] > 0 {
                has_alpha = true;
                break;
            }
        }

        let mut rgba_buffer = vec![0u8; width * height * 4];
        for (i, chunk) in bgra_buffer.chunks_exact(4).enumerate() {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = if has_alpha { chunk[3] } else { 255 };
            let offset = i * 4;
            rgba_buffer[offset] = r;
            rgba_buffer[offset + 1] = g;
            rgba_buffer[offset + 2] = b;
            rgba_buffer[offset + 3] = a;
        }

        Some(ExtractedIcon {
            width,
            height,
            rgba: rgba_buffer,
        })
    }
}
