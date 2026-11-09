import { mkdir } from "node:fs/promises";
import sharp from "sharp";

// 应用图标源图（交给 `pnpm tauri icon` 生成全平台图标集）
await sharp("assets/app-icon.svg").resize(1024, 1024).png().toFile("app-icon.png");

// 菜单栏托盘图标：macOS 模板图标按像素渲染，22px 是标准菜单栏高度
await mkdir("src-tauri/icons", { recursive: true });
await sharp("assets/tray.svg")
  .resize(22, 22, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
  .png()
  .toFile("src-tauri/icons/tray.png");

console.log("icons generated");
