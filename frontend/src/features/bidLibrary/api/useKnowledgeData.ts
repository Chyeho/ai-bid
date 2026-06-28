import { useQuery } from '@tanstack/react-query';
import { knowledgeFileApi } from './knowledgeFile';
import type { KnowledgeQueryConfig } from '../types';

// --- 3. 导出 TanStack Query Hook ---
export const useKnowledgeData = (params: KnowledgeQueryConfig) => {
   return useQuery({
      queryKey: ['knowledgeFiles', params],
      queryFn: async () => {
         // 处理前端默认的 all 分类，后端可能不认识 'all' 这个分类字符串，应该传空
         const queryParams = { ...params };
         if (queryParams.category === 'all') {
            delete queryParams.category;
         }

         // Axios response interceptor returns response.data, which is a BaseResponse object
         const response = await knowledgeFileApi.getList(queryParams) as any;
         // Extract the actual page data from the 'data' field of the generic BaseResponse
         // 如果 response.data 存在（标准的 Result<T> 结构），就用 response.data；
         // 如果 response 直接就是包含 records 的对象，就用 response 本身。
         return response.data || response;
      },
      placeholderData: (previousData) => previousData, // 保持上一页数据，防止分页切换时白屏闪烁
   });
};

// --- 补充：真实统计数据请求 ---
const fetchKnowledgeStats = async (params: KnowledgeQueryConfig) => {
   // 由于后端暂时没有提供专门的知识库统计接口
   // 我们通过调用一次获取所有数据的列表接口来自己统计
   // 这是一个临时的替代方案，直到后端加上 /api/knowledge-files/statistics 接口
   try {
      const queryParams = { ...params, page: 1, size: 10000 };
      if (queryParams.category === 'all') {
         delete queryParams.category;
      }

      const res = await knowledgeFileApi.getList(queryParams) as any;

      const responseData = res.data || res;
      const records: any[] = responseData.records || [];

      const counts = {
         all: records.length,
         regulation: records.filter((i: any) => i.category === 'regulation').length,
         price: records.filter((i: any) => i.category === 'price').length,
         supplier: records.filter((i: any) => i.category === 'supplier').length,
         contract: records.filter((i: any) => i.category === 'contract').length,
         case: records.filter((i: any) => i.category === 'case').length,
         other: records.filter((i: any) => i.category === 'other').length,
      };

      return counts;
   } catch (error) {
      return {
         all: 0,
         regulation: 0,
         price: 0,
         supplier: 0,
         contract: 0,
         case: 0,
         other: 0,
      };
   }
};

// 导出统计数据的 Hook
export const useKnowledgeStatistics = (params: KnowledgeQueryConfig) => {
   return useQuery({
      queryKey: ['knowledgeStats', params],
      queryFn: () => fetchKnowledgeStats(params),
      placeholderData: (prev) => prev,
   });
};
