import request from '@/api/request';
import { queryOptions } from '@tanstack/react-query';
import type { BaseResponse } from '@/api/types';
import type {
   SendChatRequest,
   SendChatResponse,
   ChatHistoryItem,
   FetchChatHistoryParams,
} from '../types';

export const sendChatMessage = async (
   params: SendChatRequest
): Promise<SendChatResponse> => {
   const res = await request.post<
      unknown,
      BaseResponse<SendChatResponse>,
      SendChatRequest
   >(
      '/api/chat',
      params,
      {
         timeout: 120000,
      }
   );
   return res.data;
};

export const fetchChatHistory = async (
   params: FetchChatHistoryParams
): Promise<ChatHistoryItem[]> => {
   const res = await request.get<unknown, BaseResponse<ChatHistoryItem[]>>(
      '/api/chat/history',
      { params }
   );
   return res.data ?? [];
};

export const chatOptions = {
   history: (projectId: number, bidId: number, days?: number) =>
      queryOptions({
         queryKey: ['chatHistory', projectId, bidId, days ?? 10],
         queryFn: () => fetchChatHistory({ projectId, bidId, days: 10 }),
         enabled: !!projectId && !!bidId,
         staleTime: 5 * 60 * 1000,
         refetchOnWindowFocus: false,
      }),
};

export interface CommitChatRequest {
   projectId: number;
   bidId: number;
   userContent?: string;
   aiContent?: string;
   useLatest?: boolean;
   normalizeBeforeSave?: boolean;
}

export const commitChatHistory = async (
   params: CommitChatRequest
): Promise<string> => {
   const res = await request.post<unknown, BaseResponse<string>, CommitChatRequest>(
      '/api/chat/commit',
      params,
      {
         timeout: 120000,
      }
   );
   return res.data || '';
};
