import { QueryClient } from '@tanstack/react-query';

export const queryClient = new QueryClient({
   defaultOptions: {
      queries: {
         retry: 1,
         refetchOnWindowFocus: false,
         staleTime: 1000 * 60 * 5,
      },
   },
});

// 兼容 React Router v7
(queryClient as any).defaultQueryOptions = (options: any) => ({
   ...options,
   ...queryClient.getDefaultOptions(),
});

export default queryClient;
