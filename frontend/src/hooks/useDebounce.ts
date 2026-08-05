import { useRef, useCallback, useEffect } from 'react';

export function useDebounce<T extends (...args: any[]) => any>(
   fn: T,
   wait: number
) {
   const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
   const fnRef = useRef(fn);

   useEffect(() => {
      fnRef.current = fn;
   }, [fn]);

   useEffect(() => {
      return () => {
         if (timer.current) clearTimeout(timer.current);
      };
   }, []);

   const run = useCallback(
      (...args: any[]) => {
         if (timer.current) {
            clearTimeout(timer.current);
         }
         timer.current = setTimeout(() => {
            fnRef.current(...args);
         }, wait);
      },
      [wait]
   );

   return { run };
}
