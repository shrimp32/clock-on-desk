mod menu;
#[cfg(target_os = "macos")]
mod macos;
mod settings;
mod window;

use chrono::{Datelike, Timelike};
use settings::{Settings, SettingsState};
use std::sync::Mutex;
use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;
use tauri_plugin_notification::NotificationExt;

#[tauri::command]
fn client_ready(label: String) {
    eprintln!("[client] ready: {label}");
}

/// 全局快捷键（Cmd/Ctrl+Shift+T）触发：若悬浮窗被隐藏则显示出来，
/// 并弹出一条系统通知显示当前时间（通知可以浮在其他 App 的全屏空间之上）
fn hotkey_show_time(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if !win.is_visible().unwrap_or(true) {
            let _ = win.show();
            crate::window::apply(&win, &settings::snapshot(app));
        }
    }

    let now = chrono::Local::now();
    let time = format!(
        "{:02}:{:02}:{:02}",
        now.hour(),
        now.minute(),
        now.second()
    );
    let wd = [
        "周日", "周一", "周二", "周三", "周四", "周五", "周六",
    ][now.weekday().num_days_from_sunday() as usize];
    let date = format!("{}年{}月{}日 {}", now.year(), now.month(), now.day(), wd);

    let _ = app
        .notification()
        .builder()
        .title("🕐 当前时间")
        .body(format!("{time}\n{date}"))
        .show();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 只允许一个时钟实例：重复启动时把已有窗口显示出来
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                crate::window::apply(&win, &settings::snapshot(app));
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin({
            let builder = tauri_plugin_global_shortcut::Builder::new().with_handler(
                |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        hotkey_show_time(app);
                    }
                },
            );
            builder
                .with_shortcuts(["cmd+shift+t"])
                .expect("注册全局快捷键失败")
                .build()
        })
        .manage(SettingsState(Mutex::new(Settings::default())))
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::update_settings,
            settings::reset_position,
            window::fit_window,
            window::hide_window,
            window::quit_app,
            menu::show_context_menu,
            client_ready,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // 不占 Dock、不进 Cmd+Tab，只留菜单栏图标（macOS）
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let loaded = settings::load(&handle);
            *app.state::<SettingsState>().0.lock().unwrap() = loaded.clone();

            if let Some(win) = app.get_webview_window("main") {
                window::apply(&win, &loaded);
                window::place_initial(&win, &loaded);

                #[cfg(debug_assertions)]
                {
                    for monitor in handle.available_monitors().unwrap_or_default() {
                        eprintln!(
                            "[geom] monitor name={:?} pos={:?} size={:?} scale={}",
                            monitor.name(),
                            monitor.position(),
                            monitor.size(),
                            monitor.scale_factor()
                        );
                    }
                    eprintln!(
                        "[geom] window size={:?} pos={:?} scale={}",
                        win.outer_size(),
                        win.outer_position(),
                        win.scale_factor().unwrap_or(-1.0)
                    );
                }
            }

            menu::setup_tray(&handle, &loaded)?;

            // tao 在显示窗口后会把 level 改回 floating，需延迟补刷
            window::schedule_overlay_refresh(handle.clone());

            window::start_position_saver(handle);

            Ok(())
        })
        .on_window_event(|win, event| match event {
            // 没有标题栏按钮，这里兜底：任何关闭请求都只隐藏窗口
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = win.hide();
            }
            WindowEvent::Moved(position) => window::note_position(win.app_handle(), *position),
            _ => {}
        })
        .on_menu_event(|app, event| menu::handle(app, event.id().as_ref()))
        .run(tauri::generate_context!())
        .expect("error while running clock-on-desk");
}
