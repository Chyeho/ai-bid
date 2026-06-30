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

// ── SSE Streaming ─────────────────────────────────────────────────

export interface ChatStreamCallbacks {
  onThinking: (message: string) => void;
  onToolCall: (name: string, args: string) => void;
  onAnswer: (data: SendChatResponse) => void;
  onDone: (data: SendChatResponse) => void;
  onError: (message: string) => void;
}

/**
 * Connect to the SSE chat stream endpoint.
 *
 * Uses fetch + ReadableStream to consume SSE events.
 * Returns an abort function to cancel the stream.
 */
export const connectChatStream = (
  params: SendChatRequest,
  callbacks: ChatStreamCallbacks,
): (() => void) => {
  const token =
    localStorage.getItem('token') || sessionStorage.getItem('token');
  const baseUrl = import.meta.env.VITE_API_BASE_URL || '';
  const url = `${baseUrl}/api/chat/stream`;

  const abortController = new AbortController();

  const run = async () => {
    try {
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: token ? `Bearer ${token}` : '',
          Accept: 'text/event-stream',
        },
        body: JSON.stringify(params),
        signal: abortController.signal,
      });

      if (!response.ok) throw new Error(`SSE 连接失败: ${response.status}`);
      if (!response.body) throw new Error('浏览器不支持 Stream');

      const reader = response.body.getReader();
      const decoder = new TextDecoder('utf-8');
      let buffer = '';
      let currentEvent = 'message';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed) {
            // Empty line = end of SSE event
            currentEvent = 'message';
            continue;
          }

          if (trimmed.startsWith('event:')) {
            currentEvent = trimmed.slice(6).trim();
          } else if (trimmed.startsWith('data:')) {
            const dataStr = trimmed.slice(5).trim();
            try {
              const parsed = JSON.parse(dataStr);

              switch (currentEvent) {
                case 'thinking': {
                  const msg = parsed.message || '';
                  callbacks.onThinking(msg);
                  break;
                }
                case 'tool_call': {
                  callbacks.onToolCall(
                    parsed.name || '',
                    parsed.args || '',
                  );
                  break;
                }
                case 'answer': {
                  callbacks.onAnswer({
                    content: parsed.answer || '',
                    reasoning: parsed.reasoning || [],
                    citations: parsed.references || [],
                    confidence: parsed.confidence,
                    suggestedActions: parsed.suggested_actions,
                  });
                  break;
                }
                case 'done': {
                  callbacks.onDone({
                    content: parsed.answer || '',
                    reasoning: parsed.reasoning || [],
                    citations: parsed.references || [],
                    confidence: parsed.confidence,
                    suggestedActions: parsed.suggested_actions,
                  });
                  return; // stream complete
                }
                case 'error': {
                  const msg = parsed.message || '未知错误';
                  callbacks.onError(msg);
                  return;
                }
              }
            } catch {
              console.warn('[ChatSSE] 数据解析失败:', dataStr);
            }
            currentEvent = 'message';
          }
        }
      }
    } catch (error: any) {
      if (error.name !== 'AbortError') {
        callbacks.onError(error.message || '连接失败');
      }
    }
  };

  run();

  return () => abortController.abort();
};

// ── React Query ───────────────────────────────────────────────────

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

