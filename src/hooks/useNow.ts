import { useEffect, useState } from "react";

/**
 * 对齐到整秒的心跳：避免 setInterval 累积漂移导致秒显示与系统时间不同步。
 */
export function useNow(enabled = true): Date {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    if (!enabled) return;
    let timer = 0;

    const tick = () => {
      const current = new Date();
      setNow(current);
      // +12ms 容差，确保每次都在下一秒起点之后触发
      timer = window.setTimeout(tick, 1000 - (current.getTime() % 1000) + 12);
    };

    timer = window.setTimeout(tick, 1000 - (Date.now() % 1000) + 12);
    return () => window.clearTimeout(timer);
  }, [enabled]);

  return now;
}
