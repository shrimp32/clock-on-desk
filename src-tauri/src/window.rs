//! 窗口几何：全部使用"逻辑点"(logical points) 计算。
//!
//! 注意：Tauri 的 `Monitor::size()` 返回的是物理像素，而 `Monitor::position()`
//! 与 NSWindow 的 frame 原点是逻辑坐标，两者混用会让窗口跑到屏幕外。这里统一
//! 换算成逻辑点，窗口尺寸直接用 CSS 像素（与逻辑点 1:1）。

use crate::settings::{self, Settings, SettingsState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewWindow};

/// 拖动过程中位置会高频变化，用脏标记 + 后台线程节流落盘
static POSITION_DIRTY: AtomicBool = AtomicBool::new(false);

const MARGIN: f64 = 24.0;

pub fn apply(win: &WebviewWindow, settings: &Settings) {
    let _ = win.set_always_on_top(settings.always_on_top);
    #[cfg(target_os = "macos")]
    {
        let _ = win.set_visible_on_all_workspaces(settings.all_workspaces);
        crate::macos::apply_overlay(win, settings.always_on_top, settings.all_workspaces);
    }
}

/// tao 会在窗口显示流程里把 level 重置回 floating(5)，必须在显示之后补刷几次，
/// 否则浮层压不住普通窗口/全屏 App。
pub fn schedule_overlay_refresh(app: AppHandle) {
    #[cfg(target_os = "macos")]
    thread::spawn(move || {
        for delay in [150u64, 600, 1500, 3000] {
            thread::sleep(Duration::from_millis(delay));
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(win) = handle.get_webview_window("main") {
                    let current = settings::snapshot(&handle);
                    crate::macos::apply_overlay(
                        &win,
                        current.always_on_top,
                        current.all_workspaces,
                    );
                }
            });
        }
    });
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// (x, y, width, height)，单位为逻辑点
fn monitor_rect(monitor: &tauri::Monitor) -> (f64, f64, f64, f64) {
    let scale = monitor.scale_factor();
    let pos = monitor.position();
    let size = monitor.size();
    (
        pos.x as f64 / scale,
        pos.y as f64 / scale,
        size.width as f64 / scale,
        size.height as f64 / scale,
    )
}

fn window_scale(win: &WebviewWindow) -> f64 {
    win.scale_factor().unwrap_or(1.0).max(0.1)
}

/// 窗口当前的逻辑点尺寸（等于内容 CSS 尺寸）
fn window_logical_size(win: &WebviewWindow) -> (f64, f64) {
    let scale = window_scale(win);
    match win.outer_size() {
        Ok(size) => (size.width as f64 / scale, size.height as f64 / scale),
        Err(_) => (240.0, 100.0),
    }
}

fn window_logical_position(win: &WebviewWindow) -> Option<(f64, f64)> {
    let scale = window_scale(win);
    let pos = win.outer_position().ok()?;
    Some((pos.x as f64 / scale, pos.y as f64 / scale))
}

fn monitor_at(win: &WebviewWindow, x: f64, y: f64) -> Option<tauri::Monitor> {
    win.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .find(|monitor| {
            let (mx, my, mw, mh) = monitor_rect(monitor);
            x >= mx && x < mx + mw && y >= my && y < my + mh
        })
}

fn clamp_to_monitor(win: &WebviewWindow, x: f64, y: f64) -> (f64, f64) {
    let (ww, wh) = window_logical_size(win);
    clamp_with_size(win, x, y, ww, wh)
}

fn clamp_with_size(win: &WebviewWindow, x: f64, y: f64, ww: f64, wh: f64) -> (f64, f64) {
    let Some(monitor) = monitor_at(win, x, y)
        .or_else(|| win.primary_monitor().ok().flatten())
        .or_else(|| win.current_monitor().ok().flatten())
    else {
        return (x, y);
    };

    let (mx, my, mw, mh) = monitor_rect(&monitor);
    let max_x = mx + mw - ww;
    let max_y = my + mh - wh;
    (x.max(mx).min(max_x), y.max(my).min(max_y))
}

pub fn place_initial(win: &WebviewWindow, settings: &Settings) {
    if let (Some(x), Some(y)) = (settings.x, settings.y) {
        if monitor_at(win, x, y).is_some() {
            let (cx, cy) = clamp_to_monitor(win, x, y);
            let _ = win.set_position(LogicalPosition::new(cx, cy));
            return;
        }
    }
    place_top_right(win);
}

/// 默认停靠主屏右上角（不用当前窗口所在屏，避免新窗口被系统丢到副屏）
pub fn place_top_right(win: &WebviewWindow) {
    let monitor = win
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| win.current_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };

    let (mx, my, mw, _mh) = monitor_rect(&monitor);
    let (ww, _wh) = window_logical_size(win);

    let x = mx + mw - ww - MARGIN;
    let y = my + MARGIN;
    let _ = win.set_position(LogicalPosition::new(x, y));
}

/// 保存位置（转成逻辑点，跨屏坐标才自洽）
pub fn note_position(app: &AppHandle, position: tauri::PhysicalPosition<i32>) {
    let scale = app
        .get_webview_window("main")
        .map(|win| window_scale(&win))
        .unwrap_or(1.0);
    if let Ok(mut guard) = app.state::<SettingsState>().0.lock() {
        guard.x = Some(position.x as f64 / scale);
        guard.y = Some(position.y as f64 / scale);
    }
    POSITION_DIRTY.store(true, Ordering::Relaxed);
}

/// 后台线程把位置变化节流写入 settings.json
pub fn start_position_saver(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(400));
        if POSITION_DIRTY.swap(false, Ordering::Relaxed) {
            let settings = settings::snapshot(&app);
            settings::save(&app, &settings);
        }
    });
}

/// 前端量出内容尺寸（CSS 像素）后调用；保持右边缘不动，避免右上角锚点漂移
#[tauri::command]
pub fn fit_window(window: WebviewWindow, width: f64, height: f64) {
    if !width.is_finite() || !height.is_finite() || width < 1.0 || height < 1.0 {
        return;
    }

    let (current_width, current_height) = window_logical_size(&window);
    #[cfg(debug_assertions)]
    eprintln!(
        "[fit] request={}x{} current={}x{}",
        width, height, current_width, current_height
    );
    if (current_width - width).abs() <= 1.0 && (current_height - height).abs() <= 1.0 {
        return;
    }

    let Some((pos_x, pos_y)) = window_logical_position(&window) else { return };
    let right_edge = pos_x + current_width;

    let _ = window.set_size(LogicalSize::new(width, height));

    let target_x = right_edge - width;
    let (clamped_x, clamped_y) = clamp_with_size(&window, target_x, pos_y, width, height);
    let _ = window.set_position(LogicalPosition::new(clamped_x, clamped_y));
}

#[tauri::command]
pub fn hide_window(window: WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
