import request from '@/api/request';
import { queryOptions } from '@tanstack/react-query';
import type { BaseResponse } from '@/api/types';
import type {
   AuditStatus,
   AuditResult,
   CreateTaskParams,
   BidDetail,
} from '../types';

export const createTask = async (
   params: CreateTaskParams
): Promise<{ taskId: string }> => {
   const res = await request.post<unknown, BaseResponse<{ taskId: string }>>(
      '/api/audit-tasks',
      params
   );
   
   return res.data;
};

export const getAuditStatus = async (taskId: string): Promise<AuditStatus> => {
   const res = await request.get<unknown, BaseResponse<AuditStatus>>(
      `/api/audit-tasks/${taskId}`
   );

   return res.data;
};

export const getAuditResult = async (
   taskId: string,
   params?: { page?: number; size?: number; sinceIssueNo?: string }
): Promise<AuditResult> => {
   const res = await request.get<unknown, BaseResponse<AuditResult>>(
      `/api/audit-tasks/${taskId}/result`,
      {
         params,
      }
   );

   return res.data;
};

export const getBidDetail = async (id: number): Promise<BidDetail> => {
   const res = await request.get<unknown, BaseResponse<BidDetail>>(
      `/api/bid-documents/${id}`
   );

   return res.data;
};

/** 所有 SSE 事件类型（与 Java SseEventTypeEnum 对齐） */
export type SseEventType =
   | 'progress'
   | 'issue'
   | 'issues'
   | 'complete'
   | 'agent_progress'
   | 'trace'
   | 'phase'
   | 'stats'
   | 'finding_added'
   | 'finding_updated'
   | 'finding_removed'
   | 'call_log';

export const connectStream = async (
   taskId: string,
   lastEventId: string,
   onMessage: (type: SseEventType, data: unknown) => void,
   onComplete: () => void,
   onError: (err: Error) => void
) => {
   const token =
      localStorage.getItem('token') || sessionStorage.getItem('token');
   const baseUrl = import.meta.env.VITE_API_BASE_URL || '';
   const url = `${baseUrl}/api/audit-tasks/${taskId}/stream`;

   try {
      const response = await fetch(url, {
         headers: {
            Authorization: token ? `Bearer ${token}` : '',
            'Last-Event-ID': lastEventId,
            Accept: 'text/event-stream',
         },
      });

      if (!response.ok) throw new Error(`SSE 连接失败: ${response.status}`);
      if (!response.body) throw new Error('浏览器不支持 Stream');

      const reader = response.body.getReader();
      const decoder = new TextDecoder('utf-8');

      let buffer = '';

      // 所有需要转发到 onMessage 的事件类型
      const knownTypes = new Set([
         'issue', 'issues', 'progress', 'agent_progress',
         'trace', 'phase', 'stats', 'finding_added',
         'finding_updated', 'finding_removed', 'call_log',
      ]);

      while (true) {
         const { done, value } = await reader.read();
         if (done) break;

         buffer += decoder.decode(value, { stream: true });
         const lines = buffer.split('\n'); // 按行分割
         buffer = lines.pop() || ''; // 保留最后一行(可能未读取完整)

         let currentEvent = 'message';
         let lastId = '';

         for (const line of lines) {
            const trimmedLine = line.trim();
            if (!trimmedLine) {
               currentEvent = 'message';
               continue;
            }

            // 0. 捕获 id（存 localStorage 用于断线重连）
            if (trimmedLine.startsWith('id:')) {
               lastId = trimmedLine.slice(3).trim();
               try {
                  localStorage.setItem(`auditLastEvent:${taskId}`, lastId);
               } catch { /* ignore */ }
               continue;
            }

            // 1. 捕获 event 类型
            if (trimmedLine.startsWith('event:')) {
               currentEvent = trimmedLine.slice(6).trim();
               continue;
            }

            // 2. 捕获 data 并解析
            if (trimmedLine.startsWith('data:')) {
               const dataStr = trimmedLine.slice(5).trim();
               try {
                  const parsed = JSON.parse(dataStr);

                  // 1) COMPLETE 事件：后端使用 event: complete
                  if (currentEvent === 'complete' || parsed.complete === true) {
                     onComplete();
                     return;
                  }

                  // 2) 通用事件转发：已知类型 → onMessage
                  if (knownTypes.has(currentEvent)) {
                     onMessage(currentEvent as SseEventType, parsed);
                  }
                  // 3) 默认 event: message — 检查 JSON 内 event 字段（兼容旧格式）
                  else if (currentEvent === 'message' && parsed.event) {
                     const innerType = parsed.event as string;
                     if (knownTypes.has(innerType)) {
                        onMessage(innerType as SseEventType, parsed.data ?? parsed);
                     }
                  }
               } catch {
                  console.warn('[SSE] 数据解析失败:', dataStr);
               }
               // 数据处理完重置 event，保持默认行为
               currentEvent = 'message';
            }
         }
      }
      onComplete();
   } catch (error) {
      onError(error as Error);
   }
};

export const auditDetailOptions = {
   status: (taskId: string) =>
      queryOptions({
         queryKey: ['auditStatus', taskId],
         queryFn: () => getAuditStatus(taskId),
         enabled: !!taskId,
         refetchInterval: 3000,
      }),

   result: (
      taskId: string,
      params?: { page?: number; size?: number; sinceIssueNo?: string }
   ) =>
      queryOptions({
         queryKey: ['auditResult', taskId, params],
         queryFn: () => getAuditResult(taskId, params),
         enabled: !!taskId,
         staleTime: 5 * 60 * 1000,
      }),

   bidDetail: (id: number) =>
      queryOptions({
         queryKey: ['bidDetail', id],
         queryFn: () => getBidDetail(id),
         enabled: !!id,
         staleTime: 5 * 60 * 1000,
      }),
};
