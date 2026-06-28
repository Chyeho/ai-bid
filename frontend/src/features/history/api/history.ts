import request from '@/api/request';
import { queryOptions } from '@tanstack/react-query';
import type { BaseResponse, PageResponse } from '@/api/types';
import type {
   HistoryQueryParams,
   HistoryRecord,
   HistoryStatistics,
} from '../types';

const normalizeParam = (value?: string) => {
   const normalized = value?.trim();
   if (!normalized) return undefined;
   if (normalized === 'all' || normalized === '全部') return undefined;
   return normalized;
};

// 获取历史记录列表
export const getHistoryList = async (
   params: HistoryQueryParams
): Promise<PageResponse<HistoryRecord>> => {
   const queryParams = {
      page: params.page,
      size: params.size,
      startDate: normalizeParam(params.startDate),
      endDate: normalizeParam(params.endDate),
      fileCategory: normalizeParam(params.fileCategory),
      projectName: normalizeParam(params.projectName),
      auditUserName: normalizeParam(params.auditUserName),
      auditResult:
         params.auditResult === 'all' ? undefined : params.auditResult,
   };

   // 发起请求
   const res = await request.get<
      unknown,
      BaseResponse<PageResponse<HistoryRecord>>
   >('/api/audit-history', {
      params: queryParams,
   });

   return (
      res.data ?? {
         records: [],
         total: 0,
      }
   );
};

// 获取历史记录详情
export const getHistoryDetail = async (
   auditId: string | number
): Promise<HistoryRecord> => {
   const res = await request.get<unknown, BaseResponse<HistoryRecord>>(
      `/api/audit-history/${auditId}`
   );
   return (
      res.data ?? {
         records: [],
         total: 0,
      }
   );
};

// 获取历史记录统计数据
export const getHistoryStatistics = async (
   params: HistoryQueryParams
): Promise<HistoryStatistics> => {
   const queryParams = {
      startDate: normalizeParam(params.startDate),
      endDate: normalizeParam(params.endDate),
      fileCategory: normalizeParam(params.fileCategory),
      projectName: normalizeParam(params.projectName),
      auditUserName: normalizeParam(params.auditUserName),
      auditResult:
         params.auditResult === 'all' ? undefined : params.auditResult,
   };

   const res = await request.get<unknown, BaseResponse<HistoryStatistics>>(
      '/api/audit-history/statistics',
      {
         params: queryParams,
      }
   );
   return res.data ?? [];
};

export const historyOptions = {
   list: (params: HistoryQueryParams) =>
      queryOptions({
         queryKey: ['auditHistoryList', params],
         queryFn: () => getHistoryList(params),
         placeholderData: (previousData) => previousData,
         staleTime: 10 * 1000,
      }),

   stats: (params: HistoryQueryParams) =>
      queryOptions({
         queryKey: ['auditHistoryStats', params],
         queryFn: () => getHistoryStatistics(params),
         staleTime: 10 * 1000,
      }),
};
