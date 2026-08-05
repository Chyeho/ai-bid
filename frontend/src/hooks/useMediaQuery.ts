import { useState, useEffect } from 'react';

/**
 * 通用媒体查询 Hook
 * @param query CSS 媒体查询字符串，例如 '(max-width: 768px)'
 */
export function useMediaQuery(query: string): boolean {
   // 兼容服务器端渲染 (SSR)
   const [matches, setMatches] = useState(
      typeof window !== 'undefined' ? window.matchMedia(query).matches : false
   );

   useEffect(() => {
      if (typeof window === 'undefined') return;

      const media = window.matchMedia(query);

      const listener = (e: MediaQueryListEvent) => setMatches(e.matches);

      media.addEventListener('change', listener);

      return () => {
         media.removeEventListener('change', listener);
      };
   }, [query]);

   return matches;
}

export function useIsMobile() {
   return useMediaQuery('(max-width: 768px)');
}
