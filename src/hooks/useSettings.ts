import { useEffect, useState } from "react";
import {
  DEFAULT_SETTINGS,
  getSettings,
  onSettingsChanged,
  type Settings,
} from "../lib/tauri";

export interface SettingsApi {
  settings: Settings;
  /** Rust 侧推送（菜单操作）与前端修改共用同一份状态 */
  ready: boolean;
}

export function useSettings(): SettingsApi {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;

    void getSettings()
      .then((value) => {
        if (alive) {
          setSettings(value);
          setReady(true);
        }
      })
      .catch(() => alive && setReady(true));

    void onSettingsChanged((value) => {
      if (alive) setSettings(value);
    }).then((fn) => {
      if (alive) unlisten = fn;
      else fn();
    });

    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  return { settings, ready };
}
