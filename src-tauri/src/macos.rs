//! macOS 专属窗口层级强化。
//!
//! tao 的 `set_visible_on_all_workspaces` 只设置 `CanJoinAllSpaces`，缺少
//! `FullScreenAuxiliary`，导致其他 App 全屏时时钟被挡在空间之外。
//! 这里直接对 NSWindow 补上 collectionBehavior 与窗口层级。

#![allow(unexpected_cfgs)]

use objc2::runtime::AnyObject;
use tauri::WebviewWindow;

// NSWindowCollectionBehavior 位标志
const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
// stationary：切换空间时不跟随动画位移，是浮层能稳定留在全屏 space 上的关键
const STATIONARY: usize = 1 << 4;
const IGNORES_CYCLE: usize = 1 << 6;
const FULL_SCREEN_AUXILIARY: usize = 1 << 8;

// NSWindowLevel：status 级可覆盖普通窗口；全屏 space 无法被第三方 App 覆盖，
// 全屏场景由菜单栏时间兜底
const NS_STATUS_WINDOW_LEVEL: i64 = 25;
const NS_NORMAL_WINDOW_LEVEL: i64 = 0;

/// 诊断：打印 NSWindow 的真实可见性/空间归属/层级
#[allow(dead_code)]
pub fn log_overlay_state(window: &WebviewWindow) {
    let Ok(raw) = window.ns_window() else { return };
    if raw.is_null() {
        return;
    }
    let ns_window = raw as *mut AnyObject;
    unsafe {
        let visible: bool = objc2::msg_send![ns_window, isVisible];
        let on_active_space: bool = objc2::msg_send![ns_window, isOnActiveSpace];
        let behavior: usize = objc2::msg_send![ns_window, collectionBehavior];
        let level: i64 = objc2::msg_send![ns_window, level];
        let opaque: bool = objc2::msg_send![ns_window, isOpaque];
        let style: usize = objc2::msg_send![ns_window, styleMask];
        let alpha: f64 = objc2::msg_send![ns_window, alphaValue];
        let sharing: i64 = objc2::msg_send![ns_window, sharingType];
        let ignores_mouse: bool = objc2::msg_send![ns_window, ignoresMouseEvents];
        let can_key: bool = objc2::msg_send![ns_window, canBecomeKeyWindow];
        let occlusion: usize = objc2::msg_send![ns_window, occlusionState];
        let number: i64 = objc2::msg_send![ns_window, windowNumber];
        println!(
            "[overlay-state] visible={visible} onActiveSpace={on_active_space} behavior={behavior:#x} level={level} opaque={opaque} style={style:#x} alpha={alpha} sharing={sharing} ignoresMouse={ignores_mouse} canKey={can_key} occlusion={occlusion:#x} number={number}"
        );
    }
}

pub fn apply_overlay(window: &WebviewWindow, always_on_top: bool, all_workspaces: bool) {
    let Ok(raw) = window.ns_window() else { return };
    if raw.is_null() {
        return;
    }
    let ns_window = raw as *mut AnyObject;

    let mut behavior = FULL_SCREEN_AUXILIARY | STATIONARY | IGNORES_CYCLE;
    if all_workspaces {
        behavior |= CAN_JOIN_ALL_SPACES;
    }
    let level = if always_on_top {
        NS_STATUS_WINDOW_LEVEL
    } else {
        NS_NORMAL_WINDOW_LEVEL
    };

    unsafe {
        let _: () = objc2::msg_send![ns_window, setCollectionBehavior: behavior];
        let _: () = objc2::msg_send![ns_window, setLevel: level];
        // 应用不处于激活状态时，普通 orderFront 不会把它提到前台 App 之上
        if always_on_top {
            let _: () = objc2::msg_send![ns_window, orderFrontRegardless];
        }

        #[cfg(debug_assertions)]
        {
            let actual_behavior: usize = objc2::msg_send![ns_window, collectionBehavior];
            let actual_level: i64 = objc2::msg_send![ns_window, level];
            eprintln!(
                "[overlay] want behavior={:#x} level={} got behavior={:#x} level={}",
                behavior, level, actual_behavior, actual_level
            );
        }
    }
}
