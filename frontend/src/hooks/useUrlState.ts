import { useSearchParams } from 'react-router-dom';
import { useMemo } from 'react';

/**
 * 将组件状态同步到 URL Search Params 的自定义 Hook
 * @param initialState 默认状态对象 (例如: { page: 1, keyword: '', tab: 'all' })
 */
export function useUrlState<T extends Record<string, any>>(initialState: T) {
   const [searchParams, setSearchParams] = useSearchParams();

   const state = useMemo(() => {
      const result = { ...initialState };
      for (const key in initialState) {
         const urlValue = searchParams.get(key);
         if (urlValue !== null) {
            result[key] =
               typeof initialState[key] === 'number'
                  ? (Number(urlValue) as T[Extract<keyof T, string>])
                  : (urlValue as T[Extract<keyof T, string>]);
         }
      }
      return result;
   }, [searchParams, initialState]);

   // 2. 更新状态并同步到 URL
   const setState = (newState: Partial<T>) => {
      const nextParams = new URLSearchParams(searchParams);

      Object.entries(newState).forEach(([key, value]) => {
         if (value === undefined || value === null || value === '') {
            nextParams.delete(key);
         } else {
            nextParams.set(key, String(value));
         }
      });

      setSearchParams(nextParams, { replace: true });
   };

   return [state, setState] as const;
}
