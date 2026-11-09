import { useEffect, useState } from "react";

const QUERY = "(prefers-color-scheme: dark)";

export type Theme = "dark" | "light";

/** 跟随 macOS 系统外观，切换时实时生效。 */
export function useSystemTheme(): Theme {
  const [dark, setDark] = useState(
    () => typeof window !== "undefined" && window.matchMedia(QUERY).matches,
  );

  useEffect(() => {
    const mq = window.matchMedia(QUERY);
    const onChange = (event: MediaQueryListEvent) => setDark(event.matches);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  return dark ? "dark" : "light";
}
