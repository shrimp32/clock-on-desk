use crate::settings::{self, Settings, SettingsPatch};
use chrono::Timelike;
use tauri::menu::{CheckMenuItem, ContextMenu, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

const OPACITIES: [(&str, &str, f32); 5] = [
    ("opacity-0.25", "25%", 0.25),
    ("opacity-0.40", "40%", 0.4),
    ("opacity-0.55", "55%", 0.55),
    ("opacity-0.70", "70%", 0.7),
    ("opacity-0.90", "90%", 0.9),
];

const FONT_SCALES: [(&str, &str, f32); 4] = [
    ("font-0.80", "小", 0.8),
    ("font-1.00", "中", 1.0),
    ("font-1.30", "大", 1.3),
    ("font-1.60", "特大", 1.6),
];

const EPSILON: f32 = 0.02;

/// 右键菜单与菜单栏托盘共用同一套菜单结构
pub fn build(app: &AppHandle, settings: &Settings) -> tauri::Result<Menu<Wry>> {
    let opacity = Submenu::new(app, "透明度", true)?;
    for (id, label, value) in OPACITIES {
        let item = CheckMenuItem::with_id(
            app,
            id,
            label,
            true,
            (settings.bg_alpha - value).abs() < EPSILON,
            None::<&str>,
        )?;
        opacity.append(&item)?;
    }

    let font = Submenu::new(app, "字号", true)?;
    for (id, label, value) in FONT_SCALES {
        let item = CheckMenuItem::with_id(
            app,
            id,
            label,
            true,
            (settings.font_scale - value).abs() < EPSILON,
            None::<&str>,
        )?;
        font.append(&item)?;
    }

    let seconds = CheckMenuItem::with_id(
        app,
        "toggle-seconds",
        "显示秒",
        true,
        settings.show_seconds,
        None::<&str>,
    )?;
    let date = CheckMenuItem::with_id(
        app,
        "toggle-date",
        "显示日期",
        true,
        settings.show_date,
        None::<&str>,
    )?;
    let always_on_top = CheckMenuItem::with_id(
        app,
        "toggle-top",
        "始终置顶",
        true,
        settings.always_on_top,
        None::<&str>,
    )?;
    let all_spaces = CheckMenuItem::with_id(
        app,
        "toggle-spaces",
        "显示在所有桌面空间",
        true,
        settings.all_workspaces,
        None::<&str>,
    )?;

    let reset = MenuItem::with_id(app, "reset-position", "重置到右上角", true, None::<&str>)?;
    let window_visible = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(true))
        .unwrap_or(true);
    let toggle_visibility = MenuItem::with_id(
        app,
        "toggle-visibility",
        if window_visible { "隐藏时钟" } else { "显示时钟" },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, Some("CmdOrCtrl+Q"))?;

    Menu::with_items(
        app,
        &[
            &opacity,
            &font,
            &PredefinedMenuItem::separator(app)?,
            &seconds,
            &date,
            &PredefinedMenuItem::separator(app)?,
            &always_on_top,
            &all_spaces,
            &PredefinedMenuItem::separator(app)?,
            &reset,
            &toggle_visibility,
            &quit,
        ],
    )
}

pub fn setup_tray(app: &AppHandle, settings: &Settings) -> tauri::Result<()> {
    let menu = build(app, settings)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("桌面时钟")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    // 菜单栏常驻显示当前时间：既是全屏场景的兜底，也保证托盘一定可见
    start_tray_clock(app.clone());

    Ok(())
}

/// 在菜单栏托盘上持续刷新显示当前时间（HH:mm），与前端窗口是否可见无关
fn start_tray_clock(app: AppHandle) {
    std::thread::spawn(move || loop {
        if let Some(tray) = app.tray_by_id("main") {
            let now = chrono::Local::now();
            let title = format!("{:02}:{:02}", now.hour(), now.minute());
            let _ = tray.set_title(Some(&title));
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    });
}

/// 设置变化后重建托盘菜单，同步勾选状态
pub fn refresh_tray(app: &AppHandle, settings: &Settings) {
    let Some(tray) = app.tray_by_id("main") else { return };
    if let Ok(menu) = build(app, settings) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn toggle_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        crate::window::apply(&window, &settings::snapshot(app));
    }
}

#[tauri::command]
pub fn show_context_menu(window: tauri::Window, app: AppHandle) -> Result<(), String> {
    let current = settings::snapshot(&app);
    let menu = build(&app, &current).map_err(|err| err.to_string())?;
    menu.popup(window).map_err(|err| err.to_string())
}

pub fn handle(app: &AppHandle, id: &str) {
    match id {
        "quit" => app.exit(0),
        "toggle-visibility" => {
            toggle_window(app);
            refresh_tray(app, &settings::snapshot(app));
        }
        "reset-position" => settings::reset_position(app.clone()),
        "toggle-seconds" => toggle(app, |s| SettingsPatch {
            show_seconds: Some(!s.show_seconds),
            ..Default::default()
        }),
        "toggle-date" => toggle(app, |s| SettingsPatch {
            show_date: Some(!s.show_date),
            ..Default::default()
        }),
        "toggle-top" => toggle(app, |s| SettingsPatch {
            always_on_top: Some(!s.always_on_top),
            ..Default::default()
        }),
        "toggle-spaces" => toggle(app, |s| SettingsPatch {
            all_workspaces: Some(!s.all_workspaces),
            ..Default::default()
        }),
        other => {
            if let Some(value) = other
                .strip_prefix("opacity-")
                .and_then(|v| v.parse::<f32>().ok())
            {
                settings::apply_patch(
                    app,
                    SettingsPatch {
                        bg_alpha: Some(value),
                        ..Default::default()
                    },
                );
            } else if let Some(value) = other
                .strip_prefix("font-")
                .and_then(|v| v.parse::<f32>().ok())
            {
                settings::apply_patch(
                    app,
                    SettingsPatch {
                        font_scale: Some(value),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

fn toggle<F>(app: &AppHandle, make_patch: F)
where
    F: FnOnce(&Settings) -> SettingsPatch,
{
    let current = settings::snapshot(app);
    settings::apply_patch(app, make_patch(&current));
}
