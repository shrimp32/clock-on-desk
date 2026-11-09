use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// 上次退出时的窗口位置（逻辑点），None 表示用默认右上角
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub font_scale: f32,
    pub bg_alpha: f32,
    pub show_seconds: bool,
    pub show_date: bool,
    pub always_on_top: bool,
    pub all_workspaces: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            font_scale: 1.0,
            bg_alpha: 0.45,
            show_seconds: true,
            show_date: true,
            always_on_top: true,
            // 桌面时钟的预期是“永远可见”：跨所有桌面空间，其他 App 全屏时也不被遮挡
            all_workspaces: true,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub font_scale: Option<f32>,
    pub bg_alpha: Option<f32>,
    pub show_seconds: Option<bool>,
    pub show_date: Option<bool>,
    pub always_on_top: Option<bool>,
    pub all_workspaces: Option<bool>,
}

impl Settings {
    fn merge(&mut self, patch: SettingsPatch) {
        if let Some(v) = patch.font_scale {
            self.font_scale = v.clamp(0.7, 2.0);
        }
        if let Some(v) = patch.bg_alpha {
            self.bg_alpha = v.clamp(0.2, 0.95);
        }
        if let Some(v) = patch.show_seconds {
            self.show_seconds = v;
        }
        if let Some(v) = patch.show_date {
            self.show_date = v;
        }
        if let Some(v) = patch.always_on_top {
            self.always_on_top = v;
        }
        if let Some(v) = patch.all_workspaces {
            self.all_workspaces = v;
        }
    }
}

/// 全局共享的设置状态
pub struct SettingsState(pub Mutex<Settings>);

fn file_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(FILE_NAME))
}

pub fn load(app: &AppHandle) -> Settings {
    file_path(app)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) {
    let Some(path) = file_path(app) else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, text);
    }
}

pub fn snapshot(app: &AppHandle) -> Settings {
    app.state::<SettingsState>()
        .0
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

/// 统一入口：合并 patch → 落盘 → 应用到窗口 → 广播事件 → 刷新托盘勾选态
pub fn apply_patch(app: &AppHandle, patch: SettingsPatch) -> Settings {
    let updated = {
        let state = app.state::<SettingsState>();
        let merged = match state.0.lock() {
            Ok(mut guard) => {
                guard.merge(patch);
                guard.clone()
            }
            Err(_) => Settings::default(),
        };
        merged
    };

    save(app, &updated);

    if let Some(window) = app.get_webview_window("main") {
        crate::window::apply(&window, &updated);
    }

    let _ = app.emit("settings-changed", &updated);
    crate::menu::refresh_tray(app, &updated);

    updated
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.0.lock().map(|s| s.clone()).unwrap_or_default()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, patch: SettingsPatch) -> Settings {
    apply_patch(&app, patch)
}

#[tauri::command]
pub fn reset_position(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        crate::window::place_top_right(&window);
    }
}
