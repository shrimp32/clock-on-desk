import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Clock from "./components/Clock";
import { useNow } from "./hooks/useNow";
import { useSettings } from "./hooks/useSettings";
import { useSystemTheme } from "./hooks/useSystemTheme";
import { fitWindow, showContextMenu } from "./lib/tauri";

export default function App() {
  const { settings } = useSettings();
  const now = useNow();
  const theme = useSystemTheme();
  const panelRef = useRef<HTMLDivElement>(null);
  const [ipcError, setIpcError] = useState(false);

  // IPC 联通性探针：失败时在面板上提示（正常情况下不显示）
  useEffect(() => {
    invoke("client_ready", { label: "webview" }).catch(() => setIpcError(true));
  }, []);

  // 主题跟随系统外观
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
  }, [theme]);

  // 内容尺寸 → 原生窗口尺寸（Rust 侧保持右边缘不动）
  useEffect(() => {
    const panel = panelRef.current;
    if (!panel) return;

    let timer = 0;
    const report = () => {
      const rect = panel.getBoundingClientRect();
      fitWindow(Math.ceil(rect.width), Math.ceil(rect.height));
    };
    const schedule = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(report, 40);
    };

    // 不能依赖 requestAnimationFrame：窗口被遮挡或不在当前桌面空间时会被系统暂停
    report();

    const observer = new ResizeObserver(schedule);
    observer.observe(panel);

    // 字体加载、窗口首次真正显示之后补测
    const retries = [120, 500, 1500].map((delay) => window.setTimeout(report, delay));

    window.addEventListener("resize", schedule);
    document.addEventListener("visibilitychange", report);

    return () => {
      observer.disconnect();
      window.clearTimeout(timer);
      retries.forEach((id) => window.clearTimeout(id));
      window.removeEventListener("resize", schedule);
      document.removeEventListener("visibilitychange", report);
    };
  }, []);

  return (
    <div
      ref={panelRef}
      className="panel"
      style={
        {
          "--alpha": settings.bgAlpha,
          "--scale": settings.fontScale,
        } as React.CSSProperties
      }
      data-tauri-drag-region
      onContextMenu={(event) => {
        event.preventDefault();
        showContextMenu();
      }}
    >
      <Clock
        now={now}
        showSeconds={settings.showSeconds}
        showDate={settings.showDate}
      />
      {ipcError && <div className="probe">IPC 未连接</div>}
    </div>
  );
}
