import { useEffect, useRef } from 'react';

interface UseCtrlWheelZoomOptions {
   min?: number;
   max?: number;
   step?: number;
}

/**
 * 监听容器内的 Ctrl + 鼠标滚轮事件，用于控制缩放比例
 * * @param setScale 状态更新函数 (支持传入回调来获取旧值)
 * @param options 配置项：包含最小值(min)、最大值(max)和步长(step)
 * @returns 绑定到目标 DOM 容器上的 ref
 */
export function useCtrlWheelZoom<T extends HTMLElement = HTMLDivElement>(
   setScale: React.Dispatch<React.SetStateAction<number>>,
   options: UseCtrlWheelZoomOptions = {}
) {
   // 默认配置：50% ~ 200%，每次滚动步长为 10%
   const { min = 50, max = 200, step = 10 } = options;

   const containerRef = useRef<T>(null);

   useEffect(() => {
      const container = containerRef.current;
      if (!container) return;

      const handleWheel = (e: WheelEvent) => {
         // 兼容 Windows 的 Ctrl 和 Mac 的 Command 键
         if (e.ctrlKey || e.metaKey) {
            e.preventDefault(); // 阻止浏览器默认的网页全局缩放

            requestAnimationFrame(() => {
               setScale((prev) => {
                  const zoomOut = e.deltaY > 0;
                  const nextScale = zoomOut ? prev - step : prev + step;
                  // 限制缩放的安全区间
                  return Math.min(Math.max(nextScale, min), max);
               });
            });
         }
      };

      // { passive: false } 是必须的，否则无法调用 preventDefault()
      container.addEventListener('wheel', handleWheel, { passive: false });

      return () => {
         container.removeEventListener('wheel', handleWheel);
      };
   }, [setScale, min, max, step]); // 依赖项更新时重新绑定

   return containerRef;
}
