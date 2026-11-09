import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Settings {
  x: number | null;
  y: number | null;
  fontScale: number;
  bgAlpha: number;
  showSeconds: boolean;
  showDate: boolean;
  alwaysOnTop: boolean;
  allWorkspaces: boolean;
}

export const DEFAULT_SETTINGS: Settings = {
  x: null,
  y: null,
  fontScale: 1,
  bgAlpha: 0.45,
  showSeconds: true,
  showDate: true,
  alwaysOnTop: true,
  allWorkspaces: true,
};

// 纯浏览器预览（pnpm dev）时 IPC 会失败，自动降级为 localStorage，样式仍可调试
const LS_KEY = "clock-on-desk:settings";

function readLocal(): Settings {
  try {
    const raw = localStorage.getItem(LS_KEY);
    return raw ? { ...DEFAULT_SETTINGS, ...JSON.parse(raw) } : DEFAULT_SETTINGS;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export async function getSettings(): Promise<Settings> {
  try {
    return await invoke<Settings>("get_settings");
  } catch {
    return readLocal();
  }
}

export async function updateSettings(patch: Partial<Settings>): Promise<Settings> {
  try {
    return await invoke<Settings>("update_settings", { patch });
  } catch {
    const next = { ...readLocal(), ...patch };
    localStorage.setItem(LS_KEY, JSON.stringify(next));
    return next;
  }
}

export async function onSettingsChanged(cb: (s: Settings) => void): Promise<UnlistenFn> {
  try {
    return await listen<Settings>("settings-changed", (event) => cb(event.payload));
  } catch {
    return () => {};
  }
}

/** 让原生窗口尺寸贴合内容（Rust 侧保持右边缘不动） */
export function fitWindow(width: number, height: number): void {
  invoke("fit_window", { width, height }).catch((err) => {
    invoke("client_ready", {
      label: `fit_window failed: ${String(err)} w=${width} h=${height}`,
    }).catch(() => {});
  });
}

export function showContextMenu(): void {
  invoke("show_context_menu").catch(() => {});
}

export function hideWindow(): void {
  invoke("hide_window").catch(() => {});
}
