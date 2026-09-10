use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init() {
    let mut path = PathBuf::from("taskbar-drawer.log");
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            path = dir.join("taskbar-drawer.log");
        }
    }
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    {
        let _ = writeln!(
            file,
            "=== Taskbar Drawer Log Started at {:?} ===",
            SystemTime::now()
        );
    }
    if let Ok(mut guard) = LOG_PATH.lock() {
        *guard = Some(path);
    }
}

pub fn log_msg(msg: &str) {
    if let Ok(guard) = LOG_PATH.lock() {
        if let Some(path) = guard.as_ref() {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(file, "{}", msg);
            }
        }
    }
}

macro_rules! app_log {
    ($($arg:tt)*) => {
        $crate::logger::log_msg(&format!($($arg)*))
    };
}
pub(crate) use app_log;
