import { useState, useCallback, useRef, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { message as antdMessage } from 'antd';
import {
   connectChatStream,
   chatOptions,
} from '../api/chat';
import type { ChatCitation } from '../types';

export interface ChatMessage {
   id: number;
   role: 'user' | 'assistant';
   content: string;
   /** 推理链（按 ReAct turn 顺序，每条为 LLM 在该轮的 thought） */
   reasoning?: string[];
   citations?: ChatCitation[];
   /** ChatAgent 置信度 [0, 1] */
   confidence?: number;
   /** ChatAgent 建议的后续操作 */
   suggestedActions?: string[];
   createTime: number;
   status?: 'sending' | 'streaming' | 'sent' | 'error';
}

export interface UseAiChatOptions {
   projectId: number;
   bidId: number;
   days?: number;
}

let _seed = 0;
const generateId = (): number => {
   _seed++;
   return Date.now() * 1000 + (_seed % 1000);
};

const STORAGE_KEY_PREFIX = 'aiChat:v3';

const readPersistedMessages = (storageKey: string): ChatMessage[] | null => {
   if (typeof window === 'undefined') return null;
   try {
      const raw = window.localStorage.getItem(storageKey);
      if (!raw) return null;
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed) ? parsed : null;
   } catch {
      return null;
   }
};

const writePersistedMessages = (storageKey: string, messages: ChatMessage[]) => {
   if (typeof window === 'undefined') return;
   try {
      window.localStorage.setItem(storageKey, JSON.stringify(messages));
   } catch {
      // ignore quota/storage errors
   }
};

export function useAiChat({ projectId, bidId, days = 10 }: UseAiChatOptions) {
   const queryClient = useQueryClient();
   const storageKey = `${STORAGE_KEY_PREFIX}:${projectId}:${bidId}`;
   const initialPersisted = useRef<ChatMessage[] | null>(
      readPersistedMessages(storageKey)
   );
   const [messages, setMessages] = useState<ChatMessage[]>(
      () => initialPersisted.current ?? []
   );

   const abortRef = useRef<(() => void) | null>(null);
   const [isStreaming, setIsStreaming] = useState(false);
   const pendingMsgId = useRef<number | null>(null);

   const historyQuery = useQuery(chatOptions.history(projectId, bidId, days));

   // When history loads, initialize messages if not already populated
   const isInitialized = useRef((initialPersisted.current?.length ?? 0) > 0);

   useEffect(() => {
      if (historyQuery.data && !isInitialized.current) {
         const mapped: ChatMessage[] = historyQuery.data.map((item) => ({
            id: item.id,
            role: item.role,
            content: item.content,
            citations: undefined,
            createTime: new Date(item.createTime).getTime(),
            status: 'sent' as const,
         }));
         queueMicrotask(() => {
            setMessages(mapped);
         });
         isInitialized.current = true;
      }
   }, [historyQuery.data]);

   // Persist messages to localStorage on change
   useEffect(() => {
      writePersistedMessages(storageKey, messages);
   }, [messages, storageKey]);

   const sendMessage = useCallback(
      (content: string) => {
         const trimmed = content.trim();
         if (!trimmed || isStreaming) return;

         // Abort previous stream if any
         abortRef.current?.();

         // Add user message
         const userMsgId = generateId();
         const userMsg: ChatMessage = {
            id: userMsgId,
            role: 'user',
            content: trimmed,
            createTime: Date.now(),
            status: 'sending',
         };

         // Add AI placeholder
         const aiMsgId = generateId();
         const aiMsg: ChatMessage = {
            id: aiMsgId,
            role: 'assistant',
            content: '',
            createTime: Date.now(),
            status: 'streaming',
         };

         pendingMsgId.current = aiMsgId;
         setMessages((prev) => [...prev, userMsg, aiMsg]);
         setIsStreaming(true);

         // Accumulate reasoning steps during ReAct loop (for streaming display)
         const reasoningSteps: string[] = [];

         const abort = connectChatStream(
            {
               projectId,
               bidId,
               content: trimmed,
               saveToKnowledgeBase: false,
            },
            {
               onThinking: (message) => {
                  reasoningSteps.push(message);
                  setMessages((prev) =>
                     prev.map((m) => {
                        if (m.id === pendingMsgId.current) {
                           return {
                              ...m,
                              reasoning: [...reasoningSteps],
                           };
                        }
                        return m;
                     })
                  );
               },
               onToolCall: (_name) => {
                  // Tool call events are for logging only; reasoning comes from onThinking
               },
               onAnswer: (data) => {
                  const backendReasoning = data.reasoning && data.reasoning.length > 0
                     ? data.reasoning
                     : reasoningSteps.length > 0
                        ? [...new Set(reasoningSteps)]
                        : undefined;
                  setMessages((prev) =>
                     prev.map((m) => {
                        if (m.id === pendingMsgId.current) {
                           return {
                              ...m,
                              content: data.content || '',
                              reasoning: backendReasoning,
                              citations: data.citations,
                              confidence: data.confidence,
                              suggestedActions: data.suggestedActions,
                           };
                        }
                        return m;
                     })
                  );
               },
               onDone: (data) => {
                  setMessages((prev) =>
                     prev.map((m) => {
                        if (m.id === pendingMsgId.current) {
                           return {
                              ...m,
                              content: data.content || m.content,
                              reasoning: data.reasoning || m.reasoning,
                              citations: data.citations,
                              confidence: data.confidence,
                              suggestedActions: data.suggestedActions,
                              status: 'sent' as const,
                           };
                        }
                        return m;
                     })
                  );
                  setIsStreaming(false);
                  queryClient.invalidateQueries({ queryKey: ['auditResult'] });
                  queryClient.invalidateQueries({ queryKey: ['auditStatus'] });
               },
               onError: (message) => {
                  antdMessage.error(`AI 请求失败: ${message}`);
                  setMessages((prev) =>
                     prev.map((m) => {
                        if (m.id === pendingMsgId.current) {
                           return {
                              ...m,
                              content: `错误: ${message}`,
                              status: 'error' as const,
                           };
                        }
                        return m;
                     })
                  );
                  setIsStreaming(false);
               },
            }
         );

         abortRef.current = abort;
      },
      [projectId, bidId, isStreaming, queryClient]
   );

   const clearMessages = useCallback(() => {
      abortRef.current?.();
      setIsStreaming(false);
      setMessages([]);
      if (typeof window !== 'undefined') {
         window.localStorage.removeItem(storageKey);
      }
   }, [storageKey]);

   return {
      messages,
      sendMessage,
      clearMessages,
      isLoading: isStreaming,
      isHistoryLoading: historyQuery.isLoading,
   };
}
