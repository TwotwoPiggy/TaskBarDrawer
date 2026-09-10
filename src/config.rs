use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutMode {
    Grid,
    List,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutItem {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub args: Option<String>,
    #[serde(default)]
    pub custom_icon: Option<PathBuf>,
    #[serde(default)]
    pub run_as_admin: bool,
    #[serde(default)]
    pub silent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub layout: LayoutMode,
    #[serde(default = "default_true")]
    pub auto_exit_on_launch: bool,
    #[serde(default = "default_true")]
    pub auto_exit_on_blur: bool,
    #[serde(default = "default_true")]
    pub auto_show_on_tray_hover: bool,
    pub items: Vec<ShortcutItem>,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            layout: LayoutMode::Grid,
            auto_exit_on_launch: true,
            auto_exit_on_blur: true,
            auto_show_on_tray_hover: true,
            items: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(dir) = exe_path.parent() {
                return dir.to_path_buf();
            }
        }
        PathBuf::from(".")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("shortcuts.json")
    }

    pub fn shortcuts_dir() -> PathBuf {
        Self::config_dir().join("shortcuts")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let mut config = if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                serde_json::from_str::<AppConfig>(&content).unwrap_or_default()
            } else {
                Self::default()
            }
        } else {
            Self::default()
        };

        // Automatically scan shortcuts/ folder if present
        config.scan_shortcuts_folder();
        crate::tray_manager::set_auto_show_on_tray_hover(config.auto_show_on_tray_hover);
        let _ = config.save();
        config
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    pub fn scan_shortcuts_folder(&mut self) {
        let dir = Self::shortcuts_dir();
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
            return;
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let ext = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if matches!(ext.as_str(), "bat" | "cmd" | "ps1" | "exe" | "lnk" | "url") {
                        self.add_item_from_path(&path);
                    }
                }
            }
        }
    }

    pub fn add_item_from_path(&mut self, file_path: &Path) -> bool {
        // Normalize canonical path if possible, or use absolute path
        let target_path = file_path.canonicalize().unwrap_or_else(|_| file_path.to_path_buf());

        if self.items.iter().any(|item| {
            item.path == target_path || item.path == file_path
        }) {
            return false;
        }

        let file_stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Shortcut")
            .to_string();

        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let is_bat = ext == "bat" || ext == "cmd";
        let run_as_admin = file_stem.to_lowercase().contains("admin");
        let silent = file_stem.to_lowercase().contains("silent") || file_stem.to_lowercase().contains("hide");

        let id = format!(
            "{}-{}",
            file_stem,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        self.items.push(ShortcutItem {
            id,
            name: file_stem,
            path: file_path.to_path_buf(),
            args: None,
            custom_icon: None,
            run_as_admin,
            silent: if is_bat { silent } else { false },
        });

        let _ = self.save();
        true
    }

    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            let _ = self.save();
        }
    }

    pub fn move_item(&mut self, from: usize, to: usize) {
        if from < self.items.len() && to < self.items.len() && from != to {
            let item = self.items.remove(from);
            self.items.insert(to, item);
            let _ = self.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_move_items() {
        let mut config = AppConfig::default();
        let path1 = PathBuf::from("C:\\test1.bat");
        let path2 = PathBuf::from("C:\\test2.bat");

        assert!(config.add_item_from_path(&path1));
        assert!(config.add_item_from_path(&path2));
        assert!(!config.add_item_from_path(&path1)); // Duplicate check

        assert_eq!(config.items.len(), 2);
        assert_eq!(config.items[0].name, "test1");
        assert_eq!(config.items[1].name, "test2");

        // Test move / drag reorder
        config.move_item(0, 1);
        assert_eq!(config.items[0].name, "test2");
        assert_eq!(config.items[1].name, "test1");
    }

    #[test]
    fn test_admin_and_silent_detection() {
        let mut config = AppConfig::default();
        let admin_path = PathBuf::from("C:\\setup_admin.bat");
        let silent_path = PathBuf::from("C:\\clean_silent.bat");

        config.add_item_from_path(&admin_path);
        config.add_item_from_path(&silent_path);

        assert!(config.items[0].run_as_admin);
        assert!(!config.items[0].silent);

        assert!(!config.items[1].run_as_admin);
        assert!(config.items[1].silent);
    }

    #[test]
    fn test_layout_mode_serialization() {
        let mut config = AppConfig::default();
        config.layout = LayoutMode::List;
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.layout, LayoutMode::List);
    }
}

