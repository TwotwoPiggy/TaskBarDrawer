use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_DEFAULT};
use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

use crate::config::ShortcutItem;

fn to_wide_chars(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn path_to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

pub fn launch_item(item: &ShortcutItem) -> Result<(), String> {
    let path = &item.path;
    if !path.exists() {
        return Err(format!("目标文件不存在: {}", path.display()));
    }

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let is_script = ext == "bat" || ext == "cmd";

    if item.run_as_admin {
        // Admin elevation via ShellExecuteExW with verb "runas"
        let verb_wide = to_wide_chars("runas");
        let file_wide = path_to_wide(path);
        let param_wide = item.args.as_ref().map(|a| to_wide_chars(a));
        let dir_wide = path.parent().map(|p| path_to_wide(p));

        let mut exec_info = SHELLEXECUTEINFOW::default();
        exec_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        exec_info.fMask = SEE_MASK_DEFAULT;
        exec_info.hwnd = HWND::default();
        exec_info.lpVerb = PCWSTR(verb_wide.as_ptr());
        exec_info.lpFile = PCWSTR(file_wide.as_ptr());
        if let Some(param) = &param_wide {
            exec_info.lpParameters = PCWSTR(param.as_ptr());
        }
        if let Some(dir) = &dir_wide {
            exec_info.lpDirectory = PCWSTR(dir.as_ptr());
        }
        exec_info.nShow = if item.silent { SW_HIDE.0 as i32 } else { SW_SHOWNORMAL.0 as i32 };

        unsafe {
            ShellExecuteExW(&mut exec_info).map_err(|e| format!("管理员启动失败: {e}"))?;
        }
        return Ok(());
    }

    // Normal execution
    if item.silent && is_script {
        // Run completely silently without CMD console window popping up
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let working_dir = path.parent().unwrap_or(Path::new("."));
        let mut cmd = std::process::Command::new("cmd.exe");
        cmd.arg("/c")
            .arg(path)
            .current_dir(working_dir)
            .creation_flags(CREATE_NO_WINDOW);

        if let Some(args) = &item.args {
            for arg in args.split_whitespace() {
                cmd.arg(arg);
            }
        }

        cmd.spawn().map_err(|e| format!("静默启动失败: {e}"))?;
        return Ok(());
    }

    // Standard open via ShellExecuteExW (respects associations and arguments)
    let verb_wide = to_wide_chars("open");
    let file_wide = path_to_wide(path);
    let param_wide = item.args.as_ref().map(|a| to_wide_chars(a));
    let dir_wide = path.parent().map(|p| path_to_wide(p));

    let mut exec_info = SHELLEXECUTEINFOW::default();
    exec_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    exec_info.fMask = SEE_MASK_DEFAULT;
    exec_info.hwnd = HWND::default();
    exec_info.lpVerb = PCWSTR(verb_wide.as_ptr());
    exec_info.lpFile = PCWSTR(file_wide.as_ptr());
    if let Some(param) = &param_wide {
        exec_info.lpParameters = PCWSTR(param.as_ptr());
    }
    if let Some(dir) = &dir_wide {
        exec_info.lpDirectory = PCWSTR(dir.as_ptr());
    }
    exec_info.nShow = SW_SHOWNORMAL.0 as i32;

    unsafe {
        ShellExecuteExW(&mut exec_info).map_err(|e| format!("打开失败: {e}"))?;
    }

    Ok(())
}
